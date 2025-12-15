use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

use crate::tui::app::{App, ClientField, InputMode};

pub fn draw_clients(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10)])
        .split(area);

    // Header info
    let header_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "  {} clients configured. Press [a] to add, [e] to edit, [d] to delete.",
                app.clients.len()
            ),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let header = Paragraph::new(header_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Client Management "),
    );
    frame.render_widget(header, chunks[0]);

    // Clients table
    if app.clients.is_empty() {
        let empty_lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "  No clients configured",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press [a] to add a new client",
                Style::default().fg(Color::Cyan),
            )),
        ];

        let empty = Paragraph::new(empty_lines)
            .block(Block::default().borders(Borders::ALL).title(" Clients "));
        frame.render_widget(empty, chunks[1]);
    } else {
        let header_cells = ["ID", "Name", "Contact", "Email", "City"].iter().map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });

        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = app.clients.iter().enumerate().map(|(i, client)| {
            let cells = vec![
                Cell::from(client.id.to_string()),
                Cell::from(client.name.clone()),
                Cell::from(client.contact_person.clone()),
                Cell::from(client.email.clone()),
                Cell::from(client.address_city.clone()),
            ];

            let row = Row::new(cells);
            if i == app.selected_client_index {
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
            Constraint::Length(5),
            Constraint::Percentage(25),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(15),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(" Clients "));

        frame.render_widget(table, chunks[1]);
    }

    // Draw edit dialog if editing
    if app.input_mode == InputMode::EditingClient {
        draw_client_edit_dialog(frame, app);
    }

    // Draw delete confirmation if active
    if app.confirm_delete_client.is_some() {
        draw_delete_confirm(frame, app);
    }
}

fn draw_client_edit_dialog(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 60, frame.area());

    let title = if app.adding_new_client {
        " Add Client "
    } else {
        " Edit Client "
    };

    let field_style = |field: ClientField| -> Style {
        if app.client_field == field {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        }
    };

    let cursor = |field: ClientField| -> &str { if app.client_field == field { "_" } else { "" } };

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name:      ", field_style(ClientField::Name)),
            Span::styled(
                format!("{}{}", app.client_name_input, cursor(ClientField::Name)),
                field_style(ClientField::Name),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Contact:   ", field_style(ClientField::Contact)),
            Span::styled(
                format!(
                    "{}{}",
                    app.client_contact_input,
                    cursor(ClientField::Contact)
                ),
                field_style(ClientField::Contact),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Address:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("  Street:    ", field_style(ClientField::Street)),
            Span::styled(
                format!("{}{}", app.client_street_input, cursor(ClientField::Street)),
                field_style(ClientField::Street),
            ),
        ]),
        Line::from(vec![
            Span::styled("  City:      ", field_style(ClientField::City)),
            Span::styled(
                format!("{}{}", app.client_city_input, cursor(ClientField::City)),
                field_style(ClientField::City),
            ),
        ]),
        Line::from(vec![
            Span::styled("  State:     ", field_style(ClientField::State)),
            Span::styled(
                format!("{}{}", app.client_state_input, cursor(ClientField::State)),
                field_style(ClientField::State),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Postal:    ", field_style(ClientField::Postal)),
            Span::styled(
                format!("{}{}", app.client_postal_input, cursor(ClientField::Postal)),
                field_style(ClientField::Postal),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Country:   ", field_style(ClientField::Country)),
            Span::styled(
                format!(
                    "{}{}",
                    app.client_country_input,
                    cursor(ClientField::Country)
                ),
                field_style(ClientField::Country),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Email:     ", field_style(ClientField::Email)),
            Span::styled(
                format!("{}{}", app.client_email_input, cursor(ClientField::Email)),
                field_style(ClientField::Email),
            ),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "  [Tab] Next field  [Shift+Tab] Prev  [Enter] Save  [Esc] Cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let dialog = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(dialog, area);
}

fn draw_delete_confirm(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 20, frame.area());

    let id = app.confirm_delete_client.unwrap_or(0);
    let client_name = app
        .clients
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name.as_str())
        .unwrap_or("Unknown");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Delete client '{}'?", client_name),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Press [y] to confirm, [n] to cancel"),
    ];

    let confirm = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm Delete ")
                .style(Style::default().fg(Color::Red)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(Clear, area);
    frame.render_widget(confirm, area);
}

/// Helper to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
