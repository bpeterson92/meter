use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::app::{App, InputMode, SettingsField};

pub fn draw_settings(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10)])
        .split(area);

    // Header info
    let header_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Invoice settings for your business. Press [e] to edit.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let header = Paragraph::new(header_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Invoice Settings "),
    );
    frame.render_widget(header, chunks[0]);

    // Settings display
    let settings = &app.invoice_settings;

    let settings_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Business Name:  ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(if settings.business_name.is_empty() {
                "(not set)"
            } else {
                &settings.business_name
            }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Address:        ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(if settings.address_street.is_empty() {
                "(not set)"
            } else {
                &settings.address_street
            }),
        ]),
        Line::from(vec![
            Span::raw("                  "),
            Span::raw(format!(
                "{}, {} {}",
                settings.address_city, settings.address_state, settings.address_postal
            )),
        ]),
        Line::from(vec![
            Span::raw("                  "),
            Span::raw(&settings.address_country),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Email:          ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(if settings.email.is_empty() {
                "(not set)"
            } else {
                &settings.email
            }),
        ]),
        Line::from(vec![
            Span::styled(
                "  Phone:          ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(if settings.phone.is_empty() {
                "(not set)"
            } else {
                &settings.phone
            }),
        ]),
        Line::from(vec![
            Span::styled(
                "  Tax ID:         ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(if settings.tax_id.is_empty() {
                "(not set)"
            } else {
                &settings.tax_id
            }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Payment Terms:  ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&settings.default_payment_terms),
        ]),
        Line::from(vec![
            Span::styled(
                "  Default Tax:    ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}%", settings.default_tax_rate)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Payment Instructions:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::raw("  "),
            Span::raw(if settings.payment_instructions.is_empty() {
                "(not set)"
            } else {
                &settings.payment_instructions
            }),
        ]),
    ];

    let settings_paragraph = Paragraph::new(settings_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Your Business Information "),
    );
    frame.render_widget(settings_paragraph, chunks[1]);

    // Draw edit dialog if editing
    if app.input_mode == InputMode::EditingSettings {
        draw_settings_edit_dialog(frame, app);
    }
}

fn draw_settings_edit_dialog(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 80, frame.area());

    let field_style = |field: SettingsField| -> Style {
        if app.settings_field == field {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        }
    };

    let cursor =
        |field: SettingsField| -> &str { if app.settings_field == field { "_" } else { "" } };

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Business Name:      ",
                field_style(SettingsField::BusinessName),
            ),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_business_name_input,
                    cursor(SettingsField::BusinessName)
                ),
                field_style(SettingsField::BusinessName),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Address:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("  Street:             ", field_style(SettingsField::Street)),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_street_input,
                    cursor(SettingsField::Street)
                ),
                field_style(SettingsField::Street),
            ),
        ]),
        Line::from(vec![
            Span::styled("  City:               ", field_style(SettingsField::City)),
            Span::styled(
                format!("{}{}", app.settings_city_input, cursor(SettingsField::City)),
                field_style(SettingsField::City),
            ),
        ]),
        Line::from(vec![
            Span::styled("  State:              ", field_style(SettingsField::State)),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_state_input,
                    cursor(SettingsField::State)
                ),
                field_style(SettingsField::State),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Postal:             ", field_style(SettingsField::Postal)),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_postal_input,
                    cursor(SettingsField::Postal)
                ),
                field_style(SettingsField::Postal),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Country:            ",
                field_style(SettingsField::Country),
            ),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_country_input,
                    cursor(SettingsField::Country)
                ),
                field_style(SettingsField::Country),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Contact:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("  Email:              ", field_style(SettingsField::Email)),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_email_input,
                    cursor(SettingsField::Email)
                ),
                field_style(SettingsField::Email),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Phone:              ", field_style(SettingsField::Phone)),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_phone_input,
                    cursor(SettingsField::Phone)
                ),
                field_style(SettingsField::Phone),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Tax ID:             ", field_style(SettingsField::TaxId)),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_tax_id_input,
                    cursor(SettingsField::TaxId)
                ),
                field_style(SettingsField::TaxId),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Payment:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled(
                "  Payment Terms:      ",
                field_style(SettingsField::PaymentTerms),
            ),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_payment_terms_input,
                    cursor(SettingsField::PaymentTerms)
                ),
                field_style(SettingsField::PaymentTerms),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Default Tax Rate %: ",
                field_style(SettingsField::DefaultTaxRate),
            ),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_default_tax_rate_input,
                    cursor(SettingsField::DefaultTaxRate)
                ),
                field_style(SettingsField::DefaultTaxRate),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Payment Instructions: ",
                field_style(SettingsField::PaymentInstructions),
            ),
            Span::styled(
                format!(
                    "{}{}",
                    app.settings_payment_instructions_input,
                    cursor(SettingsField::PaymentInstructions)
                ),
                field_style(SettingsField::PaymentInstructions),
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
            .title(" Edit Business Information ")
            .style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(dialog, area);
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
