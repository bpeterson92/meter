use chrono::{Datelike, Duration, Utc};
use clap::Parser;
use std::collections::HashMap;
use std::env;

mod cli;
mod db;
mod invoice;
mod models;
mod notification;
mod tui;

use cli::{Cli, ClientCommands, Commands};
use db::Db;
use invoice::{InvoiceParams, ProjectRate, filter_entries_by_month, write_invoice};
use models::{Client, Entry, InvoiceSettings};

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
    models::init_pomodoro_db(db.conn()).expect("Failed to init Pomodoro DB");
    models::init_invoice_settings_db(db.conn()).expect("Failed to init invoice settings DB");
    models::init_clients_db(db.conn()).expect("Failed to init clients DB");
    models::init_invoices_db(db.conn()).expect("Failed to init invoices DB");

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
        Commands::Invoice {
            month,
            year,
            client,
            tax_rate,
        } => {
            let all_entries = db.list(Some(true)).expect("Failed to list billed entries");
            let month = month.unwrap_or(Utc::now().month() as u32);
            let year = year.unwrap_or(Utc::now().year());

            // Filter entries by month
            let entries = filter_entries_by_month(&all_entries, year, month);

            if entries.is_empty() {
                println!("No billed entries found for {}-{:02}", year, month);
                return;
            }

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

            // Get invoice settings
            let settings = db
                .get_invoice_settings()
                .expect("Failed to get invoice settings");

            // Get client if specified
            let client_info = if let Some(client_id) = client {
                match db.get_client(*client_id) {
                    Ok(Some(c)) => Some(c),
                    Ok(None) => {
                        eprintln!("Client with ID {} not found", client_id);
                        return;
                    }
                    Err(e) => {
                        eprintln!("Failed to get client: {}", e);
                        return;
                    }
                }
            } else {
                None
            };

            // Get next invoice number
            let invoice_number = db
                .get_next_invoice_number()
                .expect("Failed to get invoice number");

            // Determine tax rate
            let effective_tax_rate = tax_rate.unwrap_or(settings.default_tax_rate);

            let params = InvoiceParams {
                entries: &entries,
                project_rates: &project_rates,
                year,
                month,
                invoice_number,
                settings: &settings,
                client: client_info.as_ref(),
                tax_rate: effective_tax_rate,
            };

            match write_invoice(&params) {
                Ok(result) => {
                    // Record the invoice in database
                    let invoice_record = models::Invoice {
                        id: 0,
                        invoice_number,
                        client_id: *client,
                        date_issued: result.date_issued.clone(),
                        due_date: result.due_date.clone(),
                        subtotal: result.subtotal,
                        tax_rate: effective_tax_rate,
                        tax_amount: result.tax_amount,
                        total: result.total,
                        file_path: result.file_path.clone(),
                    };
                    db.record_invoice(&invoice_record)
                        .expect("Failed to record invoice");

                    println!(
                        "Invoice #{} written to {}",
                        invoice_number, result.file_path
                    );
                    println!("  Subtotal: ${:.2}", result.subtotal);
                    if effective_tax_rate > 0.0 {
                        println!(
                            "  Tax ({:.1}%): ${:.2}",
                            effective_tax_rate, result.tax_amount
                        );
                    }
                    println!("  Total: ${:.2}", result.total);
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
        Commands::Pomodoro {
            enable,
            disable,
            work,
            short_break,
            long_break,
            cycles,
        } => {
            let mut config = db
                .get_pomodoro_config()
                .expect("Failed to get Pomodoro config");

            // Check if any arguments were provided
            let has_changes = *enable
                || *disable
                || work.is_some()
                || short_break.is_some()
                || long_break.is_some()
                || cycles.is_some();

            if has_changes {
                // Update config based on arguments
                if *enable {
                    config.enabled = true;
                }
                if *disable {
                    config.enabled = false;
                }
                if let Some(w) = work {
                    config.work_duration = *w;
                }
                if let Some(sb) = short_break {
                    config.short_break = *sb;
                }
                if let Some(lb) = long_break {
                    config.long_break = *lb;
                }
                if let Some(c) = cycles {
                    config.cycles_before_long = *c;
                }

                db.set_pomodoro_config(&config)
                    .expect("Failed to update Pomodoro config");
                println!("Pomodoro configuration updated");
            }

            // Always display current config
            println!("\nPomodoro Settings:");
            println!(
                "  Enabled:           {}",
                if config.enabled { "Yes" } else { "No" }
            );
            println!("  Work duration:     {} minutes", config.work_duration);
            println!("  Short break:       {} minutes", config.short_break);
            println!("  Long break:        {} minutes", config.long_break);
            println!("  Cycles before long break: {}", config.cycles_before_long);
        }
        Commands::InvoiceSettings {
            business_name,
            street,
            city,
            state,
            postal,
            country,
            email,
            phone,
            tax_id,
            payment_instructions,
            payment_terms,
            tax_rate,
        } => {
            let mut settings = db
                .get_invoice_settings()
                .expect("Failed to get invoice settings");

            // Check if any arguments were provided
            let has_changes = business_name.is_some()
                || street.is_some()
                || city.is_some()
                || state.is_some()
                || postal.is_some()
                || country.is_some()
                || email.is_some()
                || phone.is_some()
                || tax_id.is_some()
                || payment_instructions.is_some()
                || payment_terms.is_some()
                || tax_rate.is_some();

            if has_changes {
                if let Some(v) = business_name {
                    settings.business_name = v.clone();
                }
                if let Some(v) = street {
                    settings.address_street = v.clone();
                }
                if let Some(v) = city {
                    settings.address_city = v.clone();
                }
                if let Some(v) = state {
                    settings.address_state = v.clone();
                }
                if let Some(v) = postal {
                    settings.address_postal = v.clone();
                }
                if let Some(v) = country {
                    settings.address_country = v.clone();
                }
                if let Some(v) = email {
                    settings.email = v.clone();
                }
                if let Some(v) = phone {
                    settings.phone = v.clone();
                }
                if let Some(v) = tax_id {
                    settings.tax_id = v.clone();
                }
                if let Some(v) = payment_instructions {
                    settings.payment_instructions = v.clone();
                }
                if let Some(v) = payment_terms {
                    settings.default_payment_terms = v.clone();
                }
                if let Some(v) = tax_rate {
                    settings.default_tax_rate = *v;
                }

                db.set_invoice_settings(&settings)
                    .expect("Failed to update invoice settings");
                println!("Invoice settings updated\n");
            }

            // Display current settings
            println!("Invoice Settings:");
            println!("  Business Name:     {}", settings.business_name);
            println!("  Address:");
            if !settings.address_street.is_empty() {
                println!("    Street:          {}", settings.address_street);
            }
            if !settings.address_city.is_empty() {
                println!("    City:            {}", settings.address_city);
            }
            if !settings.address_state.is_empty() {
                println!("    State:           {}", settings.address_state);
            }
            if !settings.address_postal.is_empty() {
                println!("    Postal:          {}", settings.address_postal);
            }
            if !settings.address_country.is_empty() {
                println!("    Country:         {}", settings.address_country);
            }
            println!("  Email:             {}", settings.email);
            println!("  Phone:             {}", settings.phone);
            if !settings.tax_id.is_empty() {
                println!("  Tax ID:            {}", settings.tax_id);
            }
            println!("  Payment Terms:     {}", settings.default_payment_terms);
            println!("  Default Tax Rate:  {}%", settings.default_tax_rate);
            if !settings.payment_instructions.is_empty() {
                println!("  Payment Instructions:");
                for line in settings.payment_instructions.lines() {
                    println!("    {}", line);
                }
            }
        }
        Commands::Client(cmd) => match cmd {
            ClientCommands::Add {
                name,
                contact,
                street,
                city,
                state,
                postal,
                country,
                email,
            } => {
                let client = Client {
                    id: 0,
                    name: name.clone(),
                    contact_person: contact.clone().unwrap_or_default(),
                    address_street: street.clone().unwrap_or_default(),
                    address_city: city.clone().unwrap_or_default(),
                    address_state: state.clone().unwrap_or_default(),
                    address_postal: postal.clone().unwrap_or_default(),
                    address_country: country.clone().unwrap_or_default(),
                    email: email.clone().unwrap_or_default(),
                };
                let id = db.add_client(&client).expect("Failed to add client");
                println!("Added client '{}' with ID {}", name, id);
            }
            ClientCommands::List => {
                let clients = db.list_clients().expect("Failed to list clients");
                if clients.is_empty() {
                    println!("No clients found");
                } else {
                    println!(
                        "{:<5} {:<30} {:<30} {:<30}",
                        "ID", "Name", "Contact", "Email"
                    );
                    println!("{}", "-".repeat(95));
                    for client in clients {
                        println!(
                            "{:<5} {:<30} {:<30} {:<30}",
                            client.id, client.name, client.contact_person, client.email
                        );
                    }
                }
            }
            ClientCommands::Edit {
                id,
                name,
                contact,
                street,
                city,
                state,
                postal,
                country,
                email,
            } => {
                let mut client = match db.get_client(*id) {
                    Ok(Some(c)) => c,
                    Ok(None) => {
                        eprintln!("Client with ID {} not found", id);
                        return;
                    }
                    Err(e) => {
                        eprintln!("Failed to get client: {}", e);
                        return;
                    }
                };

                if let Some(v) = name {
                    client.name = v.clone();
                }
                if let Some(v) = contact {
                    client.contact_person = v.clone();
                }
                if let Some(v) = street {
                    client.address_street = v.clone();
                }
                if let Some(v) = city {
                    client.address_city = v.clone();
                }
                if let Some(v) = state {
                    client.address_state = v.clone();
                }
                if let Some(v) = postal {
                    client.address_postal = v.clone();
                }
                if let Some(v) = country {
                    client.address_country = v.clone();
                }
                if let Some(v) = email {
                    client.email = v.clone();
                }

                db.update_client(&client).expect("Failed to update client");
                println!("Updated client '{}'", client.name);
            }
            ClientCommands::Delete { id } => {
                if db.delete_client(*id).expect("Failed to delete client") {
                    println!("Deleted client with ID {}", id);
                } else {
                    eprintln!("Client with ID {} not found", id);
                }
            }
        },
    }
}
