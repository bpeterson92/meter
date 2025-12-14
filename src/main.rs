use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Utc};
use clap::Parser;
use std::env;
use std::fs::File;
use std::io::Write;

mod cli;
mod db;
mod models;
mod tui;

use cli::{Cli, Commands};
use db::Db;
use models::Entry;

fn main() {
    let cli = Cli::parse();

    // DB lives in the home directory
    let home = env::var("HOME").expect("HOME not set");
    let db_path = format!("{}/.meter/db.sqlite", home);
    let db = Db::new(&db_path).expect("Failed to open DB");

    // Ensure tables exist
    db.conn()
        .execute_batch(
            "
        PRAGMA foreign_keys = ON;
    ",
        )
        .unwrap();

    // Create tables if not present
    models::init_db(db.conn()).expect("Failed to init DB");
    models::init_projects_db(db.conn()).expect("Failed to init projects DB");

    // Sync existing entry projects to projects table
    db.sync_projects_from_entries()
        .expect("Failed to sync projects");

    match &cli.command {
        Commands::Start { project, desc } => {
            let entry = Entry {
                id: 0,
                project: project.clone(),
                description: desc.clone(),
                start: Utc::now(),
                end: None,
                billed: false,
            };
            db.insert(&entry).expect("Failed to insert entry");
            println!("Started timer for project '{}'", project);
        }
        Commands::Stop => {
            // Find the latest unended entry
            let mut stmt = db
                .conn()
                .prepare(
                    "SELECT id, project, description, start FROM entries
                 WHERE end IS NULL ORDER BY start DESC LIMIT 1",
                )
                .expect("Failed to prepare query");
            let row = stmt
                .query_row([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .expect("No running timer");

            let (id, project, description, start_str) = row;
            let start = DateTime::parse_from_rfc3339(&start_str)
                .unwrap()
                .with_timezone(&Utc);
            let now = Utc::now();
            let end_ts = now.to_rfc3339();

            db.conn()
                .execute(
                    "UPDATE entries SET end = ?1 WHERE id = ?2",
                    rusqlite::params![end_ts, id],
                )
                .expect("Failed to stop timer");
            println!(
                "Stopped timer for project '{}', duration {:.2} hrs",
                project,
                (now - start).num_seconds() as f64 / 3600.0
            );
        }
        Commands::Add {
            project,
            desc,
            duration,
        } => {
            let entry = Entry {
                id: 0,
                project: project.clone(),
                description: desc.clone(),
                start: Utc::now() - Duration::hours(*duration as i64),
                end: Some(Utc::now()),
                billed: false,
            };
            db.insert(&entry).expect("Failed to insert entry");
            println!(
                "Added manual entry for project '{}', duration {:.2} hrs",
                project, duration
            );
        }
        Commands::List { billed } => {
            let entries = db.list(Some(*billed)).expect("Failed to list entries");
            for e in entries {
                let dur = match e.end {
                    Some(end) => (end - e.start).num_seconds() as f64 / 3600.0,
                    None => 0.0,
                };
                println!(
                    "[{}] {} | {} | {:.2} hrs | {}",
                    e.id,
                    e.project,
                    e.description,
                    dur,
                    if e.billed { "billed" } else { "pending" }
                );
            }
        }
        Commands::Bill { id } => {
            if let Some(entry_id) = id {
                db.conn()
                    .execute(
                        "UPDATE entries SET billed = 1 WHERE id = ?1",
                        rusqlite::params![entry_id],
                    )
                    .expect("Failed to bill entry");
                println!("Marked entry {} as billed", entry_id);
            } else {
                // Mark all pending as billed
                db.conn()
                    .execute(
                        "UPDATE entries SET billed = 1 WHERE billed = 0",
                        rusqlite::params![],
                    )
                    .expect("Failed to bill all entries");
                println!("Marked all pending entries as billed");
            }
        }
        Commands::Unbill { id } => {
            if let Some(entry_id) = id {
                db.conn()
                    .execute(
                        "UPDATE entries SET billed = 0 WHERE id = ?1",
                        rusqlite::params![entry_id],
                    )
                    .expect("Failed to unbill entry");
                println!("Marked entry {} as unbilled", entry_id);
            } else {
                // Mark all billed as unbilled
                db.conn()
                    .execute(
                        "UPDATE entries SET billed = 0 WHERE billed = 1",
                        rusqlite::params![],
                    )
                    .expect("Failed to unbill all entries");
                println!("Marked all billed entries as unbilled");
            }
        }
        Commands::Invoice { month, year } => {
            let entries = db.list(Some(true)).expect("Failed to list billed entries");
            let month = month.unwrap_or(Utc::now().month() as u32);
            let year = year.unwrap_or(Utc::now().year());

            // Group entries by project
            let mut entries_by_project: std::collections::HashMap<String, Vec<Entry>> =
                std::collections::HashMap::new();
            for e in entries {
                let end = e.end.unwrap_or(Utc::now());
                if end.year() == year as i32 && end.month() == month {
                    entries_by_project
                        .entry(e.project.clone())
                        .or_insert_with(Vec::new)
                        .push(e);
                }
            }

            let invoice_dir = format!("{}/meter/invoices", home);
            std::fs::create_dir_all(&invoice_dir).ok();
    
            let file_path = format!("{}/invoice_{}_{:02}.txt", invoice_dir, year, month);
            
            let mut file = File::create(&file_path).expect("Failed to create invoice file");
            writeln!(file, "Invoice for {}-{:02}", year, month).unwrap();
            writeln!(file, "=========================").unwrap();
            writeln!(file).unwrap();

            let mut total_hours = 0.0;
            let mut total_cost = 0.0;
            let mut has_any_rates = false;

            for (project, proj_entries) in &entries_by_project {
                // Get project rate
                let project_data = db.get_project_by_name(project).ok().flatten();
                let rate = project_data.as_ref().and_then(|p| p.rate);
                let currency = project_data
                    .as_ref()
                    .and_then(|p| p.currency.clone())
                    .unwrap_or_else(|| "$".to_string());

                if rate.is_some() {
                    has_any_rates = true;
                }

                writeln!(file, "Project: {}", project).unwrap();
                if let Some(r) = rate {
                    writeln!(file, "Rate: {}{:.2}/hr", currency, r).unwrap();
                }
                writeln!(file, "{}", "-".repeat(40)).unwrap();

                let mut project_total = 0.0;
                for entry in proj_entries {
                    if let Some(end) = entry.end {
                        let hours = (end - entry.start).num_seconds() as f64 / 3600.0;
                        let start_local = Local.from_utc_datetime(&entry.start.naive_utc());
                        let end_local = Local.from_utc_datetime(&end.naive_utc());

                        writeln!(
                            file,
                            "  {:<20} | {} - {} | {:>6.2} hrs",
                            entry.description,
                            start_local.format("%Y-%m-%d %H:%M"),
                            end_local.format("%Y-%m-%d %H:%M"),
                            hours
                        )
                        .unwrap();
                        project_total += hours;
                    }
                }

                // Project subtotal with cost if rate exists
                if let Some(r) = rate {
                    let project_cost = project_total * r;
                    writeln!(
                        file,
                        "  Subtotal: {:>6.2} hrs x {}{:.2} = {}{:.2}",
                        project_total, currency, r, currency, project_cost
                    )
                    .unwrap();
                    total_cost += project_cost;
                } else {
                    writeln!(file, "  Subtotal: {:>6.2} hrs", project_total).unwrap();
                }
                writeln!(file).unwrap();
                total_hours += project_total;
            }
            writeln!(file, "{}", "=".repeat(50)).unwrap();
            if has_any_rates {
                writeln!(file, "Total: {:>6.2} hrs | ${:.2}", total_hours, total_cost).unwrap();
            } else {
                writeln!(file, "Total: {:>6.2} hrs", total_hours).unwrap();
            }

            println!("Invoice written to {}", file_path);
        }
        Commands::Tui => {
            tui::run_tui(db).expect("Failed to run TUI");
        }
        Commands::Rate {
            project,
            rate,
            currency,
        } => {
            if let Some(rate_value) = rate {
                db.set_project_rate(project, Some(*rate_value), Some(currency))
                    .expect("Failed to set rate");
                println!(
                    "Set rate for '{}' to {}{:.2}/hr",
                    project, currency, rate_value
                );
            } else {
                match db.get_project_by_name(project) {
                    Ok(Some(proj)) => {
                        if let Some(formatted) = proj.formatted_rate() {
                            println!("Rate for '{}': {}", project, formatted);
                        } else {
                            println!("No rate set for '{}'", project);
                        }
                    }
                    Ok(None) => println!("Project '{}' not found", project),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
        Commands::Projects => {
            let projects = db.list_projects().expect("Failed to list projects");
            if projects.is_empty() {
                println!("No projects found");
            } else {
                println!("{:<30} {:<15}", "Project", "Rate");
                println!("{}", "-".repeat(45));
                for proj in projects {
                    let rate_str = proj
                        .formatted_rate()
                        .unwrap_or_else(|| "Not set".to_string());
                    println!("{:<30} {:<15}", proj.name, rate_str);
                }
            }
        }
    }
}
