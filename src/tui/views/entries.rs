use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Row, Table},
};

use crate::tui::app::App;

pub fn draw_entries(frame: &mut Frame, app: &App, area: Rect) {
    let filter_text = if app.show_only_unbilled {
        "Filter: Unbilled"
    } else {
        "Filter: All"
    };

    let header_cells = ["ID", "Project", "Description", "Duration", "Status"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.entries.iter().enumerate().map(|(i, entry)| {
        let duration = match entry.end {
            Some(end) => {
                let hrs = (end - entry.start).num_seconds() as f64 / 3600.0;
                format!("{:.2}h", hrs)
            }
            None => "running".to_string(),
        };

        let status = if entry.end.is_none() {
            "active"
        } else if entry.billed {
            "billed"
        } else {
            "pending"
        };

        let status_style = if entry.end.is_none() {
            Style::default().fg(Color::Cyan)
        } else if entry.billed {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Green)
        };

        let cells = vec![
            Cell::from(entry.id.to_string()),
            Cell::from(entry.project.clone()),
            Cell::from(truncate_string(&entry.description, 30)),
            Cell::from(duration),
            Cell::from(Span::styled(status, status_style)),
        ];

        let row = Row::new(cells);
        if i == app.selected_entry_index {
            row.style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            row
        }
    });

    let widths = [
        Constraint::Length(6),
        Constraint::Percentage(20),
        Constraint::Percentage(40),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Entries ({}) ", filter_text)),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(table, area);
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
