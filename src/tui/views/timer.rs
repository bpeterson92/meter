use chrono::Utc;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::app::{App, InputMode, PomodoroState};

pub fn draw_timer(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Spacer
            Constraint::Length(14), // Timer display (increased for Pomodoro info)
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

    // Check Pomodoro state for special displays
    match app.pomodoro_state {
        PomodoroState::WorkComplete => {
            draw_pomodoro_prompt(frame, app, inner[1], true);
        }
        PomodoroState::BreakComplete => {
            draw_pomodoro_prompt(frame, app, inner[1], false);
        }
        PomodoroState::OnBreak => {
            draw_break_timer(frame, app, inner[1]);
        }
        _ => {
            // Normal timer display
            if let Some(entry) = &app.active_entry {
                draw_active_timer(frame, app, entry, inner[1]);
            } else {
                draw_start_form(frame, app, inner[1]);
            }
        }
    }
}

fn format_remaining_time(secs: i64) -> String {
    let mins = secs / 60;
    let s = secs % 60;
    format!("{:02}:{:02}", mins, s)
}

fn draw_active_timer(frame: &mut Frame, app: &App, entry: &crate::models::Entry, area: Rect) {
    let elapsed = Utc::now() - entry.start;
    let hours = elapsed.num_hours();
    let mins = (elapsed.num_minutes() % 60).abs();
    let secs = (elapsed.num_seconds() % 60).abs();

    let time_display = format!("{:02}:{:02}:{:02}", hours, mins, secs);

    let mut content = vec![
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
    ];

    // Add Pomodoro info if enabled and in working state
    if app.pomodoro_config.enabled && app.pomodoro_state == PomodoroState::Working {
        if let Some(remaining) = app.get_pomodoro_remaining_secs() {
            content.push(Line::from(""));
            content.push(Line::from(vec![
                Span::styled("  Pomodoro: ", Style::default().fg(Color::Magenta)),
                Span::styled(
                    format!("{} remaining", format_remaining_time(remaining)),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        " (cycle {}/{})",
                        app.pomodoro_cycles_completed + 1,
                        app.pomodoro_config.cycles_before_long
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "  Press [s] to stop",
        Style::default().fg(Color::DarkGray),
    )));

    let title = if app.pomodoro_config.enabled {
        " Active Timer [P] "
    } else {
        " Active Timer "
    };

    let timer_block = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::Green)),
    );

    frame.render_widget(timer_block, area);
}

fn draw_break_timer(frame: &mut Frame, app: &App, area: Rect) {
    let break_type = if app.is_long_break_next() {
        "Long Break"
    } else {
        "Short Break"
    };
    let remaining = app.get_pomodoro_remaining_secs().unwrap_or(0);
    let remaining_display = format_remaining_time(remaining);

    let content = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  {}", break_type),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("  {} remaining", remaining_display),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!(
                "  Cycle {}/{}",
                app.pomodoro_cycles_completed + 1,
                app.pomodoro_config.cycles_before_long
            ),
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "  Take a break! Timer will notify when done.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let timer_block = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Break Time [P] ")
            .style(Style::default().fg(Color::Blue)),
    );

    frame.render_widget(timer_block, area);
}

fn draw_pomodoro_prompt(frame: &mut Frame, app: &App, area: Rect, is_work_complete: bool) {
    let (title, message, action, color) = if is_work_complete {
        let break_type = if app.is_long_break_next() {
            "long"
        } else {
            "short"
        };
        let break_mins = app.get_current_break_duration();
        (
            " Work Complete! ",
            format!("  Ready for {} break ({} min)", break_type, break_mins),
            "  Press [Space] to start break",
            Color::Yellow,
        )
    } else {
        (
            " Break Complete! ",
            "  Ready to resume work".to_string(),
            "  Press [s] to start next work period",
            Color::Green,
        )
    };

    let content = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            if is_work_complete {
                "  Great work!"
            } else {
                "  Break finished!"
            },
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            &message,
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!(
                "  Cycles completed: {}/{}",
                app.pomodoro_cycles_completed, app.pomodoro_config.cycles_before_long
            ),
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            action,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    let block = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(color)),
    );

    frame.render_widget(block, area);
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

    // Pomodoro status line
    let pomodoro_status = if app.pomodoro_config.enabled {
        Span::styled("[P] Pomodoro: ON", Style::default().fg(Color::Magenta))
    } else {
        Span::styled("[P] Pomodoro: OFF", Style::default().fg(Color::DarkGray))
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
        Line::from(vec![Span::raw("  "), pomodoro_status]),
        Line::from(""),
        Line::from(Span::styled(
            "  [s] Start editing  [p] Toggle Pomodoro  [Tab] Switch fields",
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
