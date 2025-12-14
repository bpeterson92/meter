use crossterm::event::{KeyCode, KeyEvent};

use super::app::{App, InputMode, InvoiceMode, Message, Screen};

/// Map key events to messages based on current app state
pub fn handle_key(key: KeyEvent, app: &App) -> Option<Message> {
    // Handle help toggle globally
    if key.code == KeyCode::Char('?') && app.input_mode == InputMode::Normal {
        return Some(Message::ToggleHelp);
    }

    // If help is shown, any key closes it
    if app.show_help {
        return Some(Message::ToggleHelp);
    }

    // Handle confirm delete dialog
    if app.confirm_delete.is_some() {
        return match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(Message::ConfirmDelete),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Some(Message::CancelDelete),
            _ => None,
        };
    }

    // Handle input modes
    match app.input_mode {
        InputMode::EditingProject => {
            return match key.code {
                KeyCode::Enter => Some(Message::StartTimer),
                KeyCode::Esc => Some(Message::ExitInputMode),
                KeyCode::Tab => Some(Message::EnterInputMode(InputMode::EditingDescription)),
                KeyCode::Backspace => Some(Message::DeleteProjectChar),
                KeyCode::Char(c) => Some(Message::UpdateProjectInput(c)),
                _ => None,
            };
        }
        InputMode::EditingDescription => {
            return match key.code {
                KeyCode::Enter => Some(Message::StartTimer),
                KeyCode::Esc => Some(Message::ExitInputMode),
                KeyCode::Tab => Some(Message::EnterInputMode(InputMode::EditingProject)),
                KeyCode::Backspace => Some(Message::DeleteDescriptionChar),
                KeyCode::Char(c) => Some(Message::UpdateDescriptionInput(c)),
                _ => None,
            };
        }
        InputMode::EditEntryProject
        | InputMode::EditEntryDescription
        | InputMode::EditEntryStart
        | InputMode::EditEntryEnd => {
            return match key.code {
                KeyCode::Enter => Some(Message::SaveEditEntry),
                KeyCode::Esc => Some(Message::CancelEditEntry),
                KeyCode::Tab => Some(Message::EditNextField),
                KeyCode::BackTab => Some(Message::EditPrevField),
                KeyCode::Backspace => Some(Message::EditFieldBackspace),
                KeyCode::Char(c) => Some(Message::EditFieldInput(c)),
                _ => None,
            };
        }
        InputMode::EditingRate => {
            return match key.code {
                KeyCode::Enter => Some(Message::SaveProjectRate),
                KeyCode::Esc => Some(Message::CancelEditRate),
                KeyCode::Tab => Some(Message::EnterInputMode(InputMode::EditingCurrency)),
                KeyCode::Backspace => Some(Message::DeleteRateChar),
                KeyCode::Char(c) => Some(Message::UpdateRateInput(c)),
                _ => None,
            };
        }
        InputMode::EditingCurrency => {
            return match key.code {
                KeyCode::Enter => Some(Message::SaveProjectRate),
                KeyCode::Esc => Some(Message::CancelEditRate),
                KeyCode::Tab => Some(Message::EnterInputMode(InputMode::EditingRate)),
                KeyCode::Backspace => Some(Message::DeleteCurrencyChar),
                KeyCode::Char(c) => Some(Message::UpdateCurrencyInput(c)),
                _ => None,
            };
        }
        InputMode::Normal => {}
    }

    // Global keys (when not in input mode)
    match key.code {
        KeyCode::Char('q') => return Some(Message::Quit),
        KeyCode::Char('1') => return Some(Message::SwitchScreen(Screen::Timer)),
        KeyCode::Char('2') => return Some(Message::SwitchScreen(Screen::Entries)),
        KeyCode::Char('3') => return Some(Message::SwitchScreen(Screen::Invoice)),
        KeyCode::Char('4') => return Some(Message::SwitchScreen(Screen::Projects)),
        _ => {}
    }

    // Screen-specific keys
    match app.current_screen {
        Screen::Timer => handle_timer_keys(key, app),
        Screen::Entries => handle_entries_keys(key, app),
        Screen::Invoice => handle_invoice_keys(key, app),
        Screen::Projects => handle_projects_keys(key, app),
    }
}

fn handle_timer_keys(key: KeyEvent, app: &App) -> Option<Message> {
    match key.code {
        KeyCode::Char('s') | KeyCode::Char('S') => {
            if app.active_entry.is_some() {
                Some(Message::StopTimer)
            } else {
                Some(Message::EnterInputMode(InputMode::EditingProject))
            }
        }
        KeyCode::Enter => {
            if app.active_entry.is_none() {
                Some(Message::EnterInputMode(InputMode::EditingProject))
            } else {
                None
            }
        }
        KeyCode::Tab => {
            if app.active_entry.is_none() {
                Some(Message::EnterInputMode(InputMode::EditingProject))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn handle_entries_keys(key: KeyEvent, app: &App) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Message::SelectNextEntry),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::SelectPreviousEntry),
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if let Some(entry) = app.get_selected_entry() {
                Some(Message::EditEntry(entry.id))
            } else {
                None
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(entry) = app.get_selected_entry() {
                Some(Message::DeleteEntry(entry.id))
            } else {
                None
            }
        }
        KeyCode::Char('b') | KeyCode::Char('B') => {
            if let Some(entry) = app.get_selected_entry() {
                if !entry.billed {
                    Some(Message::MarkEntryBilled(entry.id))
                } else {
                    None
                }
            } else {
                None
            }
        }
        KeyCode::Char('u') => {
            if let Some(entry) = app.get_selected_entry() {
                if entry.billed {
                    Some(Message::UnbillEntry(entry.id))
                } else {
                    None
                }
            } else {
                None
            }
        }
        KeyCode::Char('f') | KeyCode::Char('F') => Some(Message::ToggleBilledFilter),
        KeyCode::Char('g') => Some(Message::SelectPreviousEntry), // go to top (simplified)
        KeyCode::Char('G') => Some(Message::SelectNextEntry),     // go to bottom (simplified)
        _ => None,
    }
}

fn handle_invoice_keys(key: KeyEvent, app: &App) -> Option<Message> {
    if app.invoice_mode == InvoiceMode::SelectEntries {
        // In entry selection mode
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(Message::NextInvoiceEntry),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::PrevInvoiceEntry),
            KeyCode::Char(' ') => {
                if let Some(entry) = app.invoice_entries.get(app.invoice_select_index) {
                    Some(Message::ToggleEntrySelection(entry.id))
                } else {
                    None
                }
            }
            KeyCode::Enter => Some(Message::GenerateInvoice),
            KeyCode::Esc => Some(Message::ExitInputMode),
            _ => None,
        }
    } else {
        // In mode selection
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(Message::NextInvoiceMode),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::PrevInvoiceMode),
            KeyCode::Enter => {
                if app.invoice_mode_index == 3 {
                    Some(Message::SelectInvoiceMode)
                } else {
                    Some(Message::GenerateInvoice)
                }
            }
            _ => None,
        }
    }
}

fn handle_projects_keys(key: KeyEvent, app: &App) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Message::SelectNextProject),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::SelectPreviousProject),
        KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Enter => {
            if let Some(project) = app.projects.get(app.selected_project_index) {
                Some(Message::EditProjectRate(project.id))
            } else {
                None
            }
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if let Some(project) = app.projects.get(app.selected_project_index) {
                Some(Message::ClearProjectRate(project.id))
            } else {
                None
            }
        }
        _ => None,
    }
}
