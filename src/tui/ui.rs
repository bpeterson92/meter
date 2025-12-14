use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use super::app::{App, Screen};
use super::views::{draw_entries, draw_invoice, draw_timer};

/// Main draw function that delegates to screen-specific views
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Status/help bar
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);

    match app.current_screen {
        Screen::Timer => draw_timer(frame, app, chunks[1]),
        Screen::Entries => draw_entries(frame, app, chunks[1]),
        Screen::Invoice => draw_invoice(frame, app, chunks[1]),
    }

    draw_footer(frame, app, chunks[2]);

    // Draw help overlay if active
    if app.show_help {
        draw_help_overlay(frame, app);
    }

    // Draw delete confirmation if active
    if app.confirm_delete.is_some() {
        draw_delete_confirm(frame, app);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = vec![
        if app.current_screen == Screen::Timer {
            Span::styled(
                " [1] Timer ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(" [1] Timer ", Style::default().fg(Color::DarkGray))
        },
        if app.current_screen == Screen::Entries {
            Span::styled(
                " [2] Entries ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(" [2] Entries ", Style::default().fg(Color::DarkGray))
        },
        if app.current_screen == Screen::Invoice {
            Span::styled(
                " [3] Invoice ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(" [3] Invoice ", Style::default().fg(Color::DarkGray))
        },
    ];

    let header = Paragraph::new(Line::from(tabs))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" METER - Time Tracking "),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(header, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.current_screen {
        Screen::Timer => {
            if app.active_entry.is_some() {
                "[s] Stop timer  [?] Help  [q] Quit"
            } else {
                "[s] Start timer  [?] Help  [q] Quit"
            }
        }
        Screen::Entries => "[j/k] Navigate  [d] Delete  [b] Bill  [u] Unbill  [f] Filter  [?] Help  [q] Quit",
        Screen::Invoice => "[j/k] Select  [Enter] Generate  [?] Help  [q] Quit",
    };

    let status = if let Some(msg) = &app.status_message {
        Line::from(vec![
            Span::styled(msg, Style::default().fg(Color::Green)),
            Span::raw("  |  "),
            Span::styled(help_text, Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(Span::styled(
            help_text,
            Style::default().fg(Color::DarkGray),
        ))
    };

    let footer = Paragraph::new(status).block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, area);
}

fn draw_help_overlay(frame: &mut Frame, _app: &App) {
    let area = centered_rect(60, 70, frame.area());

    let help_text = vec![
        Line::from(Span::styled(
            "METER - Help",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Global Keys",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  q        - Quit application"),
        Line::from("  1        - Go to Timer screen"),
        Line::from("  2        - Go to Entries screen"),
        Line::from("  3        - Go to Invoice screen"),
        Line::from("  ?        - Toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            "Timer Screen",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  s        - Start/Stop timer"),
        Line::from("  Tab      - Switch input field"),
        Line::from("  Enter    - Confirm and start"),
        Line::from("  Esc      - Cancel input"),
        Line::from(""),
        Line::from(Span::styled(
            "Entries Screen",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  j/k      - Navigate up/down"),
        Line::from("  d        - Delete entry"),
        Line::from("  b        - Mark as billed"),
        Line::from("  f        - Toggle filter"),
        Line::from(""),
        Line::from(Span::styled(
            "Invoice Screen",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  j/k      - Select mode"),
        Line::from("  Enter    - Generate invoice"),
        Line::from("  Space    - Toggle entry (select mode)"),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(Clear, area);
    frame.render_widget(help, area);
}

fn draw_delete_confirm(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 20, frame.area());

    let id = app.confirm_delete.unwrap_or(0);
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Delete entry {}?", id),
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
