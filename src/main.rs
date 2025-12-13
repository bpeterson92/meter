use chrono::{DateTime, Datelike, Duration, Utc};
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
    let db_path = format!("{}/.meter.sqlite", home);
    let db = Db::new(&db_path).expect("Failed to open DB");

    // Ensure tables exist
    db.conn()
        .execute_batch(
            "
        PRAGMA foreign_keys = ON;
    ",
        )
        .unwrap();

    // Create table if not present
    models::init_db(db.conn()).expect("Failed to init DB");

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
        Commands::Invoice { month, year } => {
            let entries = db.list(Some(true)).expect("Failed to list billed entries");
            let month = month.unwrap_or(Utc::now().month() as u32);
            let year = year.unwrap_or(Utc::now().year());

            let mut project_map = std::collections::HashMap::new();
            for e in entries {
                let end = e.end.unwrap_or(Utc::now());
                if end.year() == year as i32 && end.month() == month {
                    let hrs = (end - e.start).num_seconds() as f64 / 3600.0;
                    *project_map.entry(e.project.clone()).or_insert(0.0) += hrs;
                }
            }

            let mut file_path = format!("{}/invoice_{}_{}.txt", home, year, month);
            let mut file = File::create(&file_path).expect("Failed to create invoice file");
            writeln!(file, "Invoice for {}-{:02}", year, month).unwrap();
            writeln!(file, "=================").unwrap();

            let mut total = 0.0;
            for (proj, hrs) in &project_map {
                writeln!(file, "{}: {:.2} hrs", proj, hrs).unwrap();
                total += hrs;
            }
            writeln!(file, "----------------").unwrap();
            writeln!(file, "Total: {:.2} hrs", total).unwrap();

            println!("Invoice written to {}", file_path);
        }
        Commands::Tui => {
            tui::run_tui(db).expect("Failed to run TUI");
        }
    }
}
