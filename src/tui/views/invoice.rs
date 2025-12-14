use chrono::{Datelike, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::tui::app::{App, InvoiceMode};

pub fn draw_invoice(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_mode_selection(frame, app, chunks[0]);

    if app.invoice_mode == InvoiceMode::SelectEntries {
        draw_entry_selection(frame, app, chunks[1]);
    } else {
        draw_preview(frame, app, chunks[1]);
    }
}

fn draw_mode_selection(frame: &mut Frame, app: &App, area: Rect) {
    let now = Utc::now();
    let current_month = now.format("%B %Y").to_string();

    let prior_month = {
        let first_of_current = now.with_day(1).unwrap();
        let last_of_prior = first_of_current - chrono::Duration::days(1);
        last_of_prior.format("%B %Y").to_string()
    };

    let modes = [
        (
            InvoiceMode::CurrentMonth,
            format!("Current Month ({})", current_month),
        ),
        (
            InvoiceMode::PriorMonth,
            format!("Prior Month ({})", prior_month),
        ),
        (InvoiceMode::CustomRange, "Custom Date Range".to_string()),
        (
            InvoiceMode::SelectEntries,
            "Select Specific Entries".to_string(),
        ),
    ];

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Select invoice period:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, (mode, label)) in modes.iter().enumerate() {
        let is_selected =
            app.invoice_mode_index == i && app.invoice_mode != InvoiceMode::SelectEntries;
        let marker = if is_selected { ">" } else { " " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if *mode == app.invoice_mode {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(Span::styled(
            format!("  {} {}", marker, label),
            style,
        )));
        lines.push(Line::from(""));
    }

    if app.invoice_mode != InvoiceMode::SelectEntries {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  [Enter] Generate invoice",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let block = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Invoice Mode "),
    );

    frame.render_widget(block, area);
}

fn draw_preview(frame: &mut Frame, app: &App, area: Rect) {
    let now = Utc::now();
    let (title, entries) = match app.invoice_mode {
        InvoiceMode::CurrentMonth => {
            let title = format!("Preview: {}", now.format("%B %Y"));
            // Get entries for current month from app's entries list (billed ones)
            let entries: Vec<_> = app
                .entries
                .iter()
                .filter(|e| {
                    e.billed
                        && e.end.is_some()
                        && e.end.unwrap().month() == now.month()
                        && e.end.unwrap().year() == now.year()
                })
                .collect();
            (title, entries)
        }
        InvoiceMode::PriorMonth => {
            let first_of_current = now.with_day(1).unwrap();
            let last_of_prior = first_of_current - chrono::Duration::days(1);
            let title = format!("Preview: {}", last_of_prior.format("%B %Y"));
            let entries: Vec<_> = app
                .entries
                .iter()
                .filter(|e| {
                    e.billed
                        && e.end.is_some()
                        && e.end.unwrap().month() == last_of_prior.month()
                        && e.end.unwrap().year() == last_of_prior.year()
                })
                .collect();
            (title, entries)
        }
        InvoiceMode::CustomRange => {
            let title = "Preview: Custom Range".to_string();
            // Would need date picker implementation
            (title, Vec::new())
        }
        InvoiceMode::SelectEntries => {
            // Shouldn't reach here as we draw entry selection instead
            ("".to_string(), Vec::new())
        }
    };

    // Calculate hours by project
    let mut project_hours: std::collections::HashMap<String, f64> =
        std::collections::HashMap::new();
    for entry in &entries {
        if let Some(end) = entry.end {
            let hours = (end - entry.start).num_seconds() as f64 / 3600.0;
            *project_hours.entry(entry.project.clone()).or_insert(0.0) += hours;
        }
    }

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", title),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if project_hours.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No billed entries found",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let mut total_hours = 0.0;
        let mut total_cost = 0.0;
        let mut has_rates = false;

        for (project, hours) in &project_hours {
            if let Some((rate, currency)) = app.project_rates.get(project) {
                has_rates = true;
                let cost = hours * rate;
                lines.push(Line::from(format!(
                    "  {}: {:.2} hrs x {}{:.2} = {}{:.2}",
                    project, hours, currency, rate, currency, cost
                )));
                total_cost += cost;
            } else {
                lines.push(Line::from(format!("  {}: {:.2} hrs", project, hours)));
            }
            total_hours += hours;
        }
        lines.push(Line::from(""));
        lines.push(Line::from("  ----------------"));
        lines.push(Line::from(Span::styled(
            format!("  Total: {:.2} hrs", total_hours),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        if has_rates {
            lines.push(Line::from(Span::styled(
                format!("  Total Cost: ${:.2}", total_cost),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        }
    }

    let block =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Preview "));

    frame.render_widget(block, area);
}

fn draw_entry_selection(frame: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["Sel", "ID", "Project", "Description", "Duration"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.invoice_entries.iter().enumerate().map(|(i, entry)| {
        let is_selected = app.selected_entry_ids.contains(&entry.id);
        let checkbox = if is_selected { "[x]" } else { "[ ]" };

        let duration = match entry.end {
            Some(end) => {
                let hrs = (end - entry.start).num_seconds() as f64 / 3600.0;
                format!("{:.2}h", hrs)
            }
            None => "-".to_string(),
        };

        let cells = vec![
            Cell::from(checkbox),
            Cell::from(entry.id.to_string()),
            Cell::from(entry.project.clone()),
            Cell::from(truncate_string(&entry.description, 20)),
            Cell::from(duration),
        ];

        let row = Row::new(cells);
        if i == app.invoice_select_index {
            row.style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            row
        }
    });

    // Calculate selected total
    let selected_hours: f64 = app
        .invoice_entries
        .iter()
        .filter(|e| app.selected_entry_ids.contains(&e.id))
        .filter_map(|e| {
            e.end
                .map(|end| (end - e.start).num_seconds() as f64 / 3600.0)
        })
        .sum();

    let widths = [
        Constraint::Length(5),
        Constraint::Length(6),
        Constraint::Percentage(25),
        Constraint::Percentage(35),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default().borders(Borders::ALL).title(format!(
            " Select Entries (Selected: {:.2} hrs) ",
            selected_hours
        )),
    );

    frame.render_widget(table, area);
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
