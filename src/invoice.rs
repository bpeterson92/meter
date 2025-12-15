use chrono::{Datelike, Days, Local, TimeZone, Utc};
use genpdf::elements::{Break, Paragraph, TableLayout};
use genpdf::fonts::{FontData, FontFamily};
use genpdf::style::Style;
use genpdf::{Document, Element, SimplePageDecorator};
use std::collections::HashMap;
use std::fs;
use std::io;

use crate::models::{Client, Entry, InvoiceSettings};

/// Project rate information for invoice calculations
#[derive(Debug, Clone)]
pub struct ProjectRate {
    pub rate: f64,
    pub currency: String,
}

/// Parameters for invoice generation
pub struct InvoiceParams<'a> {
    pub entries: &'a [Entry],
    pub project_rates: &'a HashMap<String, ProjectRate>,
    pub year: i32,
    pub month: u32,
    pub invoice_number: i64,
    pub settings: &'a InvoiceSettings,
    pub client: Option<&'a Client>,
    pub tax_rate: f64,
}

/// Result of invoice generation
#[derive(Debug)]
pub struct InvoiceResult {
    pub file_path: String,
    pub date_issued: String,
    pub due_date: String,
    pub subtotal: f64,
    pub tax_amount: f64,
    pub total: f64,
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
    let font_configs = [
        (
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/System/Library/Fonts/Supplemental/Arial Italic.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf",
        ),
        (
            "/System/Library/Fonts/Supplemental/Courier New.ttf",
            "/System/Library/Fonts/Supplemental/Courier New Bold.ttf",
            "/System/Library/Fonts/Supplemental/Courier New Italic.ttf",
            "/System/Library/Fonts/Supplemental/Courier New Bold Italic.ttf",
        ),
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

/// Calculate due date based on payment terms
fn calculate_due_date(payment_terms: &str) -> String {
    let today = Utc::now();
    let days = if payment_terms.to_lowercase().contains("net 30") {
        30
    } else if payment_terms.to_lowercase().contains("net 15") {
        15
    } else if payment_terms.to_lowercase().contains("net 60") {
        60
    } else {
        0 // Due on receipt
    };

    if days > 0 {
        let due = today.checked_add_days(Days::new(days)).unwrap_or(today);
        due.format("%Y-%m-%d").to_string()
    } else {
        today.format("%Y-%m-%d").to_string()
    }
}

/// Generate and write invoice to PDF file
pub fn write_invoice(params: &InvoiceParams) -> io::Result<InvoiceResult> {
    let invoice_dir = get_invoice_dir()?;
    let file_path = format!("{}/invoice_{:04}.pdf", invoice_dir, params.invoice_number);

    let date_issued = Utc::now().format("%Y-%m-%d").to_string();
    let due_date = calculate_due_date(&params.settings.default_payment_terms);

    // Group entries by project
    let mut entries_by_project: HashMap<String, Vec<&Entry>> = HashMap::new();
    for entry in params.entries {
        entries_by_project
            .entry(entry.project.clone())
            .or_default()
            .push(entry);
    }

    // Load font and create document
    let font_family = load_font_family()?;
    let mut doc = Document::new(font_family);
    doc.set_title(format!("Invoice #{:04}", params.invoice_number));

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(20);
    doc.set_page_decorator(decorator);

    // Styles
    let title_style = Style::new().bold().with_font_size(24);
    let heading_style = Style::new().bold().with_font_size(14);
    let normal_style = Style::new().with_font_size(10);
    let small_style = Style::new().with_font_size(9);
    let bold_style = Style::new().bold().with_font_size(10);

    // === HEADER: Invoice title and number ===
    doc.push(Paragraph::new(format!("INVOICE #{:04}", params.invoice_number)).styled(title_style));
    doc.push(Break::new(1.0));

    // === FROM / TO Section ===
    // Create a simple two-column layout using text

    // From (Your business info)
    if !params.settings.business_name.is_empty() {
        doc.push(Paragraph::new("From:").styled(bold_style));
        doc.push(Paragraph::new(&params.settings.business_name).styled(normal_style));
        let addr = params.settings.formatted_address();
        if !addr.is_empty() {
            for line in addr.lines() {
                doc.push(Paragraph::new(line).styled(small_style));
            }
        }
        if !params.settings.email.is_empty() {
            doc.push(Paragraph::new(&params.settings.email).styled(small_style));
        }
        if !params.settings.phone.is_empty() {
            doc.push(Paragraph::new(&params.settings.phone).styled(small_style));
        }
        if !params.settings.tax_id.is_empty() {
            doc.push(
                Paragraph::new(format!("Tax ID: {}", params.settings.tax_id)).styled(small_style),
            );
        }
        doc.push(Break::new(0.5));
    }

    // To (Client info)
    if let Some(client) = params.client {
        doc.push(Paragraph::new("Bill To:").styled(bold_style));
        doc.push(Paragraph::new(&client.name).styled(normal_style));
        if !client.contact_person.is_empty() {
            doc.push(
                Paragraph::new(format!("Attn: {}", client.contact_person)).styled(small_style),
            );
        }
        let addr = client.formatted_address();
        if !addr.is_empty() {
            for line in addr.lines() {
                doc.push(Paragraph::new(line).styled(small_style));
            }
        }
        if !client.email.is_empty() {
            doc.push(Paragraph::new(&client.email).styled(small_style));
        }
        doc.push(Break::new(0.5));
    }

    // === Invoice metadata ===
    doc.push(Break::new(0.5));
    doc.push(Paragraph::new(format!("Invoice Date: {}", date_issued)).styled(normal_style));
    doc.push(Paragraph::new(format!("Due Date: {}", due_date)).styled(normal_style));
    doc.push(
        Paragraph::new(format!("Terms: {}", params.settings.default_payment_terms))
            .styled(normal_style),
    );
    doc.push(
        Paragraph::new(format!("Period: {}-{:02}", params.year, params.month)).styled(normal_style),
    );
    doc.push(Break::new(1.0));

    // === LINE ITEMS ===
    doc.push(Paragraph::new("Services").styled(heading_style));
    doc.push(Break::new(0.5));

    let mut subtotal = 0.0;

    for (project, proj_entries) in &entries_by_project {
        let rate_info = params.project_rates.get(project);

        // Project header
        let project_style = Style::new().bold().with_font_size(12);
        doc.push(Paragraph::new(format!("Project: {}", project)).styled(project_style));

        if let Some(r) = rate_info {
            let rate_style = Style::new().with_font_size(9).italic();
            doc.push(
                Paragraph::new(format!("Rate: {}{:.2}/hr", r.currency, r.rate)).styled(rate_style),
            );
        }
        doc.push(Break::new(0.3));

        // Create table for entries
        let mut table = TableLayout::new(vec![4, 2, 2, 1]);
        table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(
            false, false, false,
        ));

        // Table header
        let header_style = Style::new().bold().with_font_size(9);
        let mut header_row = table.row();
        header_row.push_element(Paragraph::new("Description").styled(header_style));
        header_row.push_element(Paragraph::new("Start").styled(header_style));
        header_row.push_element(Paragraph::new("End").styled(header_style));
        header_row.push_element(Paragraph::new("Hours").styled(header_style));
        header_row.push().expect("Failed to push header row");

        let cell_style = Style::new().with_font_size(8);
        let mut project_total = 0.0;

        for entry in proj_entries {
            if let Some(end) = entry.end {
                let hours = (end - entry.start).num_seconds() as f64 / 3600.0;
                let start_local = Local.from_utc_datetime(&entry.start.naive_utc());
                let end_local = Local.from_utc_datetime(&end.naive_utc());

                let mut row = table.row();
                row.push_element(Paragraph::new(&entry.description).styled(cell_style));
                row.push_element(
                    Paragraph::new(start_local.format("%m/%d %H:%M").to_string())
                        .styled(cell_style),
                );
                row.push_element(
                    Paragraph::new(end_local.format("%m/%d %H:%M").to_string()).styled(cell_style),
                );
                row.push_element(Paragraph::new(format!("{:.2}", hours)).styled(cell_style));
                row.push().expect("Failed to push row");

                project_total += hours;
            }
        }

        doc.push(table);
        doc.push(Break::new(0.2));

        // Project subtotal
        if let Some(r) = rate_info {
            let project_cost = project_total * r.rate;
            doc.push(
                Paragraph::new(format!(
                    "  {:.2} hrs × {}{:.2} = {}{:.2}",
                    project_total, r.currency, r.rate, r.currency, project_cost
                ))
                .styled(bold_style),
            );
            subtotal += project_cost;
        } else {
            doc.push(Paragraph::new(format!("  {:.2} hrs", project_total)).styled(bold_style));
        }

        doc.push(Break::new(0.8));
    }

    // === TOTALS ===
    doc.push(Break::new(0.5));

    let total_style = Style::new().bold().with_font_size(12);

    doc.push(Paragraph::new(format!("Subtotal: ${:.2}", subtotal)).styled(normal_style));

    let tax_amount = subtotal * (params.tax_rate / 100.0);
    if params.tax_rate > 0.0 {
        doc.push(
            Paragraph::new(format!("Tax ({:.1}%): ${:.2}", params.tax_rate, tax_amount))
                .styled(normal_style),
        );
    }

    let total = subtotal + tax_amount;
    doc.push(Break::new(0.3));
    doc.push(Paragraph::new(format!("TOTAL DUE: ${:.2}", total)).styled(total_style));

    // === PAYMENT INSTRUCTIONS ===
    if !params.settings.payment_instructions.is_empty() {
        doc.push(Break::new(1.5));
        doc.push(Paragraph::new("Payment Instructions").styled(heading_style));
        doc.push(Break::new(0.3));
        for line in params.settings.payment_instructions.lines() {
            doc.push(Paragraph::new(line).styled(small_style));
        }
    }

    // Render to file
    doc.render_to_file(&file_path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    Ok(InvoiceResult {
        file_path,
        date_issued,
        due_date,
        subtotal,
        tax_amount,
        total,
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
