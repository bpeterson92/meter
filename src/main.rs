use chrono::{Datelike, Duration, Utc};
use clap::Parser;
use std::collections::HashMap;
use std::env;

mod cli;
mod db;
mod invoice;
mod models;
mod tui;

use cli::{Cli, Commands};
use db::Db;
use invoice::{ProjectRate, filter_entries_by_month, write_invoice};
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
            db.start_timer(project, desc)
                .expect("Failed to start timer");
            println!("Started timer for project '{}'", project);
        }
        Commands::Stop => match db.stop_active_timer().expect("Failed to stop timer") {
            Some(entry) => {
                let duration = entry
                    .end
                    .map(|end| (end - entry.start).num_seconds() as f64 / 3600.0)
                    .unwrap_or(0.0);
                println!(
                    "Stopped timer for project '{}', duration {:.2} hrs",
                    entry.project, duration
                );
            }
            None => {
                println!("No running timer");
            }
        },
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
                db.mark_billed(*entry_id).expect("Failed to bill entry");
                println!("Marked entry {} as billed", entry_id);
            } else {
                let count = db.mark_all_billed().expect("Failed to bill all entries");
                println!("Marked {} entries as billed", count);
            }
        }
        Commands::Unbill { id } => {
            if let Some(entry_id) = id {
                db.unmark_billed(*entry_id).expect("Failed to unbill entry");
                println!("Marked entry {} as unbilled", entry_id);
            } else {
                let count = db
                    .unmark_all_billed()
                    .expect("Failed to unbill all entries");
                println!("Marked {} entries as unbilled", count);
            }
        }
        Commands::Invoice { month, year } => {
            let all_entries = db.list(Some(true)).expect("Failed to list billed entries");
            let month = month.unwrap_or(Utc::now().month() as u32);
            let year = year.unwrap_or(Utc::now().year());

            // Filter entries by month
            let entries = filter_entries_by_month(&all_entries, year, month);

            // Build project rates map
            let mut project_rates: HashMap<String, ProjectRate> = HashMap::new();
            for entry in &entries {
                if !project_rates.contains_key(&entry.project) {
                    if let Ok(Some(proj)) = db.get_project_by_name(&entry.project) {
                        if let Some(rate) = proj.rate {
                            project_rates.insert(
                                entry.project.clone(),
                                ProjectRate {
                                    rate,
                                    currency: proj.currency.unwrap_or_else(|| "$".to_string()),
                                },
                            );
                        }
                    }
                }
            }

            match write_invoice(&entries, &project_rates, year, month) {
                Ok(result) => {
                    println!("Invoice written to {}", result.file_path);
                }
                Err(e) => {
                    eprintln!("Failed to write invoice: {}", e);
                }
            }
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
