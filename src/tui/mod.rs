pub mod app;
pub mod event;
pub mod ui;
pub mod views;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, poll, read},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::db::Db;
use app::{App, Message, RunningState};

/// Main entry point for TUI mode
pub fn run_tui(db: Db) -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(&db);

    // Main loop
    let result = run_app(&mut terminal, &mut app, &db);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    db: &Db,
) -> io::Result<()> {
    loop {
        // Render
        terminal.draw(|f| ui::draw(f, app))?;

        // Handle events with timeout (for timer updates)
        if poll(Duration::from_millis(250))? {
            if let Event::Key(key) = read()? {
                if let Some(msg) = event::handle_key(key, &app) {
                    // Process message and any follow-up messages
                    let mut current_msg = Some(msg);
                    while let Some(m) = current_msg {
                        current_msg = app.update(m, db);
                    }
                }
            }
        } else {
            // Tick for timer updates
            app.update(Message::Tick, db);
        }

        // Check if we should quit
        if app.running_state == RunningState::Done {
            return Ok(());
        }
    }
}
