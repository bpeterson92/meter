use chrono::{Datelike, Local, TimeZone};
use genpdf::elements::{Break, Paragraph, TableLayout};
use genpdf::fonts::{FontData, FontFamily};
use genpdf::style::Style;
use genpdf::{Document, Element, SimplePageDecorator};
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

/// Get the invoice directory path (creates if needed)
pub fn get_invoice_dir() -> io::Result<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let invoice_dir = format!("{}/meter/invoices", home);
    fs::create_dir_all(&invoice_dir)?;
    Ok(invoice_dir)
}

/// Load font from system paths
fn load_font_family() -> io::Result<FontFamily<FontData>> {
    // macOS system font paths - try Arial first as it's most reliable
    let font_configs = [
        // macOS Arial (individual files)
        (
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/System/Library/Fonts/Supplemental/Arial Italic.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf",
        ),
        // macOS Courier New as fallback
        (
            "/System/Library/Fonts/Supplemental/Courier New.ttf",
            "/System/Library/Fonts/Supplemental/Courier New Bold.ttf",
            "/System/Library/Fonts/Supplemental/Courier New Italic.ttf",
            "/System/Library/Fonts/Supplemental/Courier New Bold Italic.ttf",
        ),
        // Linux Liberation Sans
        (
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Italic.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-BoldItalic.ttf",
        ),
    ];

    for (regular, bold, italic, bold_italic) in &font_configs {
        if let Ok(regular_data) = fs::read(regular) {
            let regular_font = FontData::new(regular_data, None)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

            // Try to load bold/italic variants, fall back to regular if not found
            let bold_font = fs::read(bold)
                .ok()
                .and_then(|data| FontData::new(data, None).ok())
                .unwrap_or_else(|| regular_font.clone());

            let italic_font = fs::read(italic)
                .ok()
                .and_then(|data| FontData::new(data, None).ok())
                .unwrap_or_else(|| regular_font.clone());

            let bold_italic_font = fs::read(bold_italic)
                .ok()
                .and_then(|data| FontData::new(data, None).ok())
                .unwrap_or_else(|| regular_font.clone());

            return Ok(FontFamily {
                regular: regular_font,
                bold: bold_font,
                italic: italic_font,
                bold_italic: bold_italic_font,
            });
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No suitable font found. Please ensure Arial fonts are installed.",
    ))
}

/// Generate and write invoice to PDF file
/// Returns the result with file path and totals
pub fn write_invoice(
    entries: &[Entry],
    project_rates: &HashMap<String, ProjectRate>,
    year: i32,
    month: u32,
) -> io::Result<InvoiceResult> {
    let invoice_dir = get_invoice_dir()?;
    let file_path = format!("{}/invoice_{}_{:02}.pdf", invoice_dir, year, month);

    // Group entries by project
    let mut entries_by_project: HashMap<String, Vec<&Entry>> = HashMap::new();
    for entry in entries {
        entries_by_project
            .entry(entry.project.clone())
            .or_default()
            .push(entry);
    }

    // Calculate totals
    let mut total_hours = 0.0;
    let mut total_cost = 0.0;
    let mut has_any_rates = false;

    // Load font and create document
    let font_family = load_font_family()?;
    let mut doc = Document::new(font_family);
    doc.set_title(format!("Invoice {}-{:02}", year, month));

    // Set up page decorator with margins
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(20);
    doc.set_page_decorator(decorator);

    // Title
    let title_style = Style::new().bold().with_font_size(24);
    doc.push(Paragraph::new(format!("Invoice for {}-{:02}", year, month)).styled(title_style));
    doc.push(Break::new(1.5));

    // Process each project
    for (project, proj_entries) in &entries_by_project {
        let rate_info = project_rates.get(project);
        if rate_info.is_some() {
            has_any_rates = true;
        }

        // Project header
        let project_style = Style::new().bold().with_font_size(14);
        doc.push(Paragraph::new(format!("Project: {}", project)).styled(project_style));

        if let Some(r) = rate_info {
            let rate_style = Style::new().with_font_size(10).italic();
            doc.push(
                Paragraph::new(format!("Rate: {}{:.2}/hr", r.currency, r.rate)).styled(rate_style),
            );
        }
        doc.push(Break::new(0.5));

        // Create table for entries
        let mut table = TableLayout::new(vec![3, 2, 2, 1]);
        table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(
            false, false, false,
        ));

        // Table header
        let header_style = Style::new().bold().with_font_size(10);
        let mut header_row = table.row();
        header_row.push_element(Paragraph::new("Description").styled(header_style));
        header_row.push_element(Paragraph::new("Start").styled(header_style));
        header_row.push_element(Paragraph::new("End").styled(header_style));
        header_row.push_element(Paragraph::new("Hours").styled(header_style));
        header_row.push().expect("Failed to push header row");

        let cell_style = Style::new().with_font_size(9);
        let mut project_total = 0.0;

        for entry in proj_entries {
            if let Some(end) = entry.end {
                let hours = (end - entry.start).num_seconds() as f64 / 3600.0;
                let start_local = Local.from_utc_datetime(&entry.start.naive_utc());
                let end_local = Local.from_utc_datetime(&end.naive_utc());

                let mut row = table.row();
                row.push_element(Paragraph::new(&entry.description).styled(cell_style));
                row.push_element(
                    Paragraph::new(start_local.format("%Y-%m-%d %H:%M").to_string())
                        .styled(cell_style),
                );
                row.push_element(
                    Paragraph::new(end_local.format("%Y-%m-%d %H:%M").to_string())
                        .styled(cell_style),
                );
                row.push_element(Paragraph::new(format!("{:.2}", hours)).styled(cell_style));
                row.push().expect("Failed to push row");

                project_total += hours;
            }
        }

        doc.push(table);
        doc.push(Break::new(0.3));

        // Project subtotal
        let subtotal_style = Style::new().bold().with_font_size(10);
        if let Some(r) = rate_info {
            let project_cost = project_total * r.rate;
            doc.push(
                Paragraph::new(format!(
                    "Subtotal: {:.2} hrs × {}{:.2} = {}{:.2}",
                    project_total, r.currency, r.rate, r.currency, project_cost
                ))
                .styled(subtotal_style),
            );
            total_cost += project_cost;
        } else {
            doc.push(
                Paragraph::new(format!("Subtotal: {:.2} hrs", project_total))
                    .styled(subtotal_style),
            );
        }

        total_hours += project_total;
        doc.push(Break::new(1.0));
    }

    // Grand total
    doc.push(Break::new(0.5));
    let total_style = Style::new().bold().with_font_size(14);

    if has_any_rates {
        doc.push(
            Paragraph::new(format!(
                "Total: {:.2} hours | ${:.2}",
                total_hours, total_cost
            ))
            .styled(total_style),
        );
    } else {
        doc.push(Paragraph::new(format!("Total: {:.2} hours", total_hours)).styled(total_style));
    }

    // Render to file
    doc.render_to_file(&file_path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    Ok(InvoiceResult {
        file_path,
        total_hours,
        total_cost,
        has_rates: has_any_rates,
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

/// Generate invoice content as text (for preview/display)
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
