use chrono::{Datelike, Local, TimeZone};
use std::collections::HashMap;
use std::fs;
use std::io;

use crate::models::Entry;

/// Project rate information for invoice calculations
#[derive(Debug, Clone)]
pub struct ProjectRate {
    pub rate: f64,
    pub currency: String,
}

/// Result of invoice generation
#[derive(Debug)]
pub struct InvoiceResult {
    pub file_path: String,
    pub total_hours: f64,
    pub total_cost: f64,
    pub has_rates: bool,
}

/// Generate invoice content from entries and project rates
/// Returns the formatted invoice text
pub fn generate_invoice_content(
    entries: &[Entry],
    project_rates: &HashMap<String, ProjectRate>,
    year: i32,
    month: u32,
) -> (String, f64, f64, bool) {
    // Group entries by project
    let mut entries_by_project: HashMap<String, Vec<&Entry>> = HashMap::new();
    for entry in entries {
        entries_by_project
            .entry(entry.project.clone())
            .or_default()
            .push(entry);
    }

    let mut content = format!("Invoice for {}-{:02}\n", year, month);
    content.push_str("=========================\n\n");

    let mut total_hours = 0.0;
    let mut total_cost = 0.0;
    let mut has_any_rates = false;

    for (project, proj_entries) in &entries_by_project {
        let rate_info = project_rates.get(project);
        if rate_info.is_some() {
            has_any_rates = true;
        }

        content.push_str(&format!("Project: {}\n", project));
        if let Some(r) = rate_info {
            content.push_str(&format!("Rate: {}{:.2}/hr\n", r.currency, r.rate));
        }
        content.push_str(&format!("{}\n", "-".repeat(40)));

        let mut project_total = 0.0;
        for entry in proj_entries {
            if let Some(end) = entry.end {
                let hours = (end - entry.start).num_seconds() as f64 / 3600.0;
                let start_local = Local.from_utc_datetime(&entry.start.naive_utc());
                let end_local = Local.from_utc_datetime(&end.naive_utc());

                content.push_str(&format!(
                    "  {:<24} | {} - {} | {:>6.2} hrs\n",
                    entry.description,
                    start_local.format("%Y-%m-%d %H:%M"),
                    end_local.format("%Y-%m-%d %H:%M"),
                    hours
                ));
                project_total += hours;
            }
        }

        // Project subtotal with cost if rate exists
        if let Some(r) = rate_info {
            let project_cost = project_total * r.rate;
            content.push_str(&format!(
                "  Subtotal: {:>6.2} hrs x {}{:.2} = {}{:.2}\n\n",
                project_total, r.currency, r.rate, r.currency, project_cost
            ));
            total_cost += project_cost;
        } else {
            content.push_str(&format!("  Subtotal: {:>6.2} hrs\n\n", project_total));
        }
        total_hours += project_total;
    }

    content.push_str(&format!("{}\n", "=".repeat(50)));
    if has_any_rates {
        content.push_str(&format!(
            "Total: {:>6.2} hrs | ${:.2}\n",
            total_hours, total_cost
        ));
    } else {
        content.push_str(&format!("Total: {:>6.2} hrs\n", total_hours));
    }

    (content, total_hours, total_cost, has_any_rates)
}

/// Get the invoice directory path (creates if needed)
pub fn get_invoice_dir() -> io::Result<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let invoice_dir = format!("{}/meter/invoices", home);
    fs::create_dir_all(&invoice_dir)?;
    Ok(invoice_dir)
}

/// Generate and write invoice to file
/// Returns the result with file path and totals
pub fn write_invoice(
    entries: &[Entry],
    project_rates: &HashMap<String, ProjectRate>,
    year: i32,
    month: u32,
) -> io::Result<InvoiceResult> {
    let invoice_dir = get_invoice_dir()?;
    let file_path = format!("{}/invoice_{}_{:02}.txt", invoice_dir, year, month);

    let (content, total_hours, total_cost, has_rates) =
        generate_invoice_content(entries, project_rates, year, month);

    fs::write(&file_path, content)?;

    Ok(InvoiceResult {
        file_path,
        total_hours,
        total_cost,
        has_rates,
    })
}

/// Filter entries by month and year
pub fn filter_entries_by_month(entries: &[Entry], year: i32, month: u32) -> Vec<Entry> {
    entries
        .iter()
        .filter(|e| {
            if let Some(end) = e.end {
                end.year() == year && end.month() == month
            } else {
                false
            }
        })
        .cloned()
        .collect()
}
