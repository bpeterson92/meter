use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::app::{App, InputMode, PomodoroField};

pub fn draw_pomodoro(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Spacer
            Constraint::Length(16), // Config form
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

    draw_config_form(frame, app, inner[1]);
}

fn draw_config_form(frame: &mut Frame, app: &App, area: Rect) {
    let field_style = |field: PomodoroField, editing: bool| -> Style {
        if app.pomodoro_field == field {
            if editing {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            }
        } else {
            Style::default().fg(Color::White)
        }
    };

    let cursor = |field: PomodoroField| -> &str {
        if app.pomodoro_field == field
            && matches!(
                app.input_mode,
                InputMode::EditingPomodoroWork
                    | InputMode::EditingPomodoroShortBreak
                    | InputMode::EditingPomodoroLongBreak
                    | InputMode::EditingPomodoroCycles
            )
        {
            "_"
        } else {
            ""
        }
    };

    let enabled_text = if app.pomodoro_config.enabled {
        Span::styled(
            "ON",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "OFF",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    };

    let is_editing = matches!(
        app.input_mode,
        InputMode::EditingPomodoroWork
            | InputMode::EditingPomodoroShortBreak
            | InputMode::EditingPomodoroLongBreak
            | InputMode::EditingPomodoroCycles
    );

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Pomodoro Timer Configuration",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Enabled:           ",
                field_style(PomodoroField::Enabled, false),
            ),
            if app.pomodoro_field == PomodoroField::Enabled {
                Span::styled("[", Style::default().fg(Color::Cyan))
            } else {
                Span::raw(" ")
            },
            enabled_text,
            if app.pomodoro_field == PomodoroField::Enabled {
                Span::styled("]", Style::default().fg(Color::Cyan))
            } else {
                Span::raw(" ")
            },
            Span::styled(
                if app.pomodoro_field == PomodoroField::Enabled {
                    " (Enter to toggle)"
                } else {
                    ""
                },
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Work duration:     ",
                field_style(PomodoroField::WorkDuration, is_editing),
            ),
            Span::styled(
                format!(
                    "[{}{}]",
                    app.pomodoro_work_input,
                    cursor(PomodoroField::WorkDuration)
                ),
                field_style(PomodoroField::WorkDuration, is_editing),
            ),
            Span::styled(" minutes", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Short break:       ",
                field_style(PomodoroField::ShortBreak, is_editing),
            ),
            Span::styled(
                format!(
                    "[{}{}]",
                    app.pomodoro_short_break_input,
                    cursor(PomodoroField::ShortBreak)
                ),
                field_style(PomodoroField::ShortBreak, is_editing),
            ),
            Span::styled(" minutes", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Long break:        ",
                field_style(PomodoroField::LongBreak, is_editing),
            ),
            Span::styled(
                format!(
                    "[{}{}]",
                    app.pomodoro_long_break_input,
                    cursor(PomodoroField::LongBreak)
                ),
                field_style(PomodoroField::LongBreak, is_editing),
            ),
            Span::styled(" minutes", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Cycles before long break: ",
                field_style(PomodoroField::Cycles, is_editing),
            ),
            Span::styled(
                format!(
                    "[{}{}]",
                    app.pomodoro_cycles_input,
                    cursor(PomodoroField::Cycles)
                ),
                field_style(PomodoroField::Cycles, is_editing),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  [Tab] Next field  [Enter] Save  [Esc] Cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let title = if app.pomodoro_config.enabled {
        " Pomodoro Settings [ON] "
    } else {
        " Pomodoro Settings [OFF] "
    };

    let border_color = if app.pomodoro_config.enabled {
        Color::Magenta
    } else {
        Color::White
    };

    let form_block = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(border_color)),
    );

    frame.render_widget(form_block, area);
}
