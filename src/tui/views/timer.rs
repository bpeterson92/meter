use chrono::Utc;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::app::{App, InputMode};

pub fn draw_timer(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Spacer
            Constraint::Length(12), // Timer display
            Constraint::Min(0),     // Rest
        ])
        .split(area);

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(chunks[1]);

    if let Some(entry) = &app.active_entry {
        draw_active_timer(frame, entry, inner[1]);
    } else {
        draw_start_form(frame, app, inner[1]);
    }
}

fn draw_active_timer(frame: &mut Frame, entry: &crate::models::Entry, area: Rect) {
    let elapsed = Utc::now() - entry.start;
    let hours = elapsed.num_hours();
    let mins = (elapsed.num_minutes() % 60).abs();
    let secs = (elapsed.num_seconds() % 60).abs();

    let time_display = format!("{:02}:{:02}:{:02}", hours, mins, secs);

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Project: "),
            Span::styled(
                &entry.project,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Description: "),
            Span::styled(&entry.description, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  {}", time_display),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Started: "),
            Span::styled(
                entry.start.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press [s] to stop",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let timer_block = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Active Timer ")
            .style(Style::default().fg(Color::Green)),
    );

    frame.render_widget(timer_block, area);
}

fn draw_start_form(frame: &mut Frame, app: &App, area: Rect) {
    let project_style = if app.input_mode == InputMode::EditingProject {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let desc_style = if app.input_mode == InputMode::EditingDescription {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let project_cursor = if app.input_mode == InputMode::EditingProject {
        "_"
    } else {
        ""
    };
    let desc_cursor = if app.input_mode == InputMode::EditingDescription {
        "_"
    } else {
        ""
    };

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Start a new timer",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Project:     "),
            Span::styled(
                format!("[{}{}]", &app.project_input, project_cursor),
                project_style,
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Description: "),
            Span::styled(
                format!("[{}{}]", &app.description_input, desc_cursor),
                desc_style,
            ),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "  Press [s] or [Enter] to start editing",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  Press [Tab] to switch fields, [Enter] to start timer",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let form_block = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" New Timer ")
            .style(Style::default().fg(Color::White)),
    );

    frame.render_widget(form_block, area);
}
