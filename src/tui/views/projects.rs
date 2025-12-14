use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

use crate::tui::app::App;

pub fn draw_projects(frame: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["ID", "Project Name", "Rate", "Currency"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.projects.iter().enumerate().map(|(i, project)| {
        let rate_str = project
            .rate
            .map(|r| format!("{:.2}", r))
            .unwrap_or_else(|| "-".to_string());

        let currency_str = project.currency.clone().unwrap_or_else(|| "-".to_string());

        let cells = vec![
            Cell::from(project.id.to_string()),
            Cell::from(project.name.clone()),
            Cell::from(rate_str),
            Cell::from(currency_str),
        ];

        let row = Row::new(cells);
        if i == app.selected_project_index {
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
        Constraint::Percentage(50),
        Constraint::Length(12),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Projects & Rates "),
    );

    frame.render_widget(table, area);
}
