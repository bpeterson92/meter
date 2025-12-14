use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Timelike, Utc};
use std::collections::HashMap;

use crate::db::Db;
use crate::models::{Entry, Project};

/// The active screen/view in the TUI
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Screen {
    #[default]
    Timer,
    Entries,
    Invoice,
    Projects,
}

/// Running state of the application
#[derive(Debug, Clone, PartialEq, Default)]
pub enum RunningState {
    #[default]
    Running,
    Done,
}

/// Invoice date range selection mode
#[derive(Debug, Clone, PartialEq, Default)]
pub enum InvoiceMode {
    #[default]
    CurrentMonth,
    PriorMonth,
    CustomRange,
    SelectEntries,
}

/// Input mode for text entry
#[derive(Debug, Clone, PartialEq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    EditingProject,
    EditingDescription,
    // Entry editing modes
    EditEntryProject,
    EditEntryDescription,
    EditEntryStart,
    EditEntryEnd,
    // Project rate editing modes
    EditingRate,
    EditingCurrency,
}

/// Which field is selected in the edit entry dialog
#[derive(Debug, Clone, PartialEq, Default)]
pub enum EditField {
    #[default]
    Project,
    Description,
    Start,
    End,
}

/// Main application state
#[derive(Debug, Default)]
pub struct App {
    // Core state
    pub running_state: RunningState,
    pub current_screen: Screen,

    // Timer state
    pub active_entry: Option<Entry>,
    pub project_input: String,
    pub description_input: String,
    pub input_mode: InputMode,

    // Entries list state
    pub entries: Vec<Entry>,
    pub selected_entry_index: usize,
    pub show_only_unbilled: bool,
    pub confirm_delete: Option<i64>,

    // Edit entry state
    pub editing_entry: Option<Entry>,
    pub edit_field: EditField,
    pub edit_project_input: String,
    pub edit_description_input: String,
    pub edit_start_input: String,
    pub edit_end_input: String,

    // Invoice state
    pub invoice_mode: InvoiceMode,
    pub invoice_mode_index: usize,
    pub custom_start_date: Option<NaiveDate>,
    pub custom_end_date: Option<NaiveDate>,
    pub selected_entry_ids: Vec<i64>,
    pub invoice_entries: Vec<Entry>,
    pub invoice_select_index: usize,

    // UI state
    pub show_help: bool,
    pub status_message: Option<String>,

    // Projects state
    pub projects: Vec<Project>,
    pub selected_project_index: usize,
    pub editing_project_rate: Option<i64>,
    pub rate_input: String,
    pub currency_input: String,

    // Project rates cache for invoice
    pub project_rates: HashMap<String, (f64, String)>,
}

/// All possible application messages/events
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    // Navigation
    SwitchScreen(Screen),
    Quit,

    // Timer actions
    StartTimer,
    StopTimer,
    UpdateProjectInput(char),
    UpdateDescriptionInput(char),
    DeleteProjectChar,
    DeleteDescriptionChar,

    // Entry list actions
    SelectNextEntry,
    SelectPreviousEntry,
    ToggleBilledFilter,
    DeleteEntry(i64),
    ConfirmDelete,
    CancelDelete,
    MarkEntryBilled(i64),
    UnbillEntry(i64),

    // Edit entry actions
    EditEntry(i64),
    EditNextField,
    EditPrevField,
    EditFieldInput(char),
    EditFieldBackspace,
    SaveEditEntry,
    CancelEditEntry,

    // Invoice actions
    NextInvoiceMode,
    PrevInvoiceMode,
    SelectInvoiceMode,
    ToggleEntrySelection(i64),
    NextInvoiceEntry,
    PrevInvoiceEntry,
    GenerateInvoice,

    // Input mode
    EnterInputMode(InputMode),
    ExitInputMode,

    // UI
    ToggleHelp,
    ClearStatus,
    Tick,

    // Data refresh
    RefreshEntries,
    RefreshActiveTimer,

    // Project rate actions
    RefreshProjects,
    SelectNextProject,
    SelectPreviousProject,
    EditProjectRate(i64),
    UpdateRateInput(char),
    UpdateCurrencyInput(char),
    DeleteRateChar,
    DeleteCurrencyChar,
    SaveProjectRate,
    CancelEditRate,
    ClearProjectRate(i64),
}

impl App {
    pub fn new(db: &Db) -> Self {
        let mut app = App::default();
        app.description_input = "Work session".to_string();
        app.refresh_entries(db);
        app.refresh_active_timer(db);
        app
    }

    /// Core update function
    pub fn update(&mut self, msg: Message, db: &Db) -> Option<Message> {
        match msg {
            // Navigation
            Message::SwitchScreen(screen) => {
                self.current_screen = screen.clone();
                self.input_mode = InputMode::Normal;
                self.confirm_delete = None;
                if screen == Screen::Invoice {
                    self.refresh_invoice_entries(db);
                }
                if screen == Screen::Projects {
                    self.projects = db.list_projects().unwrap_or_default();
                }
                None
            }
            Message::Quit => {
                self.running_state = RunningState::Done;
                None
            }

            // Timer actions
            Message::StartTimer => {
                if !self.project_input.is_empty() && self.active_entry.is_none() {
                    let entry = Entry {
                        id: 0,
                        project: self.project_input.clone(),
                        description: self.description_input.clone(),
                        start: Utc::now(),
                        end: None,
                        billed: false,
                    };
                    if db.insert(&entry).is_ok() {
                        self.project_input.clear();
                        self.description_input = "Work session".to_string();
                        self.status_message = Some("Timer started".to_string());
                        self.input_mode = InputMode::Normal;
                        return Some(Message::RefreshActiveTimer);
                    }
                }
                None
            }
            Message::StopTimer => {
                if self.active_entry.is_some() {
                    if db.stop_active_timer().is_ok() {
                        self.active_entry = None;
                        self.status_message = Some("Timer stopped".to_string());
                        return Some(Message::RefreshEntries);
                    }
                }
                None
            }
            Message::UpdateProjectInput(c) => {
                self.project_input.push(c);
                None
            }
            Message::UpdateDescriptionInput(c) => {
                self.description_input.push(c);
                None
            }
            Message::DeleteProjectChar => {
                self.project_input.pop();
                None
            }
            Message::DeleteDescriptionChar => {
                self.description_input.pop();
                None
            }

            // Entry navigation
            Message::SelectNextEntry => {
                if !self.entries.is_empty() {
                    self.selected_entry_index =
                        (self.selected_entry_index + 1).min(self.entries.len() - 1);
                }
                None
            }
            Message::SelectPreviousEntry => {
                self.selected_entry_index = self.selected_entry_index.saturating_sub(1);
                None
            }
            Message::ToggleBilledFilter => {
                self.show_only_unbilled = !self.show_only_unbilled;
                self.selected_entry_index = 0;
                Some(Message::RefreshEntries)
            }

            // Delete flow
            Message::DeleteEntry(id) => {
                self.confirm_delete = Some(id);
                None
            }
            Message::ConfirmDelete => {
                if let Some(id) = self.confirm_delete.take() {
                    if db.delete(id).is_ok() {
                        self.status_message = Some(format!("Entry {} deleted", id));
                        if self.selected_entry_index > 0 {
                            self.selected_entry_index -= 1;
                        }
                        return Some(Message::RefreshEntries);
                    }
                }
                None
            }
            Message::CancelDelete => {
                self.confirm_delete = None;
                None
            }

            // Edit entry
            Message::EditEntry(id) => {
                if let Some(entry) = self.entries.iter().find(|e| e.id == id) {
                    self.editing_entry = Some(entry.clone());
                    self.edit_field = EditField::Project;
                    self.edit_project_input = entry.project.clone();
                    self.edit_description_input = entry.description.clone();

                    let start_local = Local.from_utc_datetime(&entry.start.naive_utc());
                    self.edit_start_input = start_local.format("%Y-%m-%d %H:%M").to_string();

                    self.edit_end_input = match entry.end {
                        Some(end) => {
                            let end_local = Local.from_utc_datetime(&end.naive_utc());
                            end_local.format("%Y-%m-%d %H:%M").to_string()
                        }
                        None => String::new(),
                    };

                    self.input_mode = InputMode::EditEntryProject;
                }
                None
            }
            Message::EditNextField => {
                self.edit_field = match self.edit_field {
                    EditField::Project => EditField::Description,
                    EditField::Description => EditField::Start,
                    EditField::Start => EditField::End,
                    EditField::End => EditField::Project,
                };
                self.input_mode = match self.edit_field {
                    EditField::Project => InputMode::EditEntryProject,
                    EditField::Description => InputMode::EditEntryDescription,
                    EditField::Start => InputMode::EditEntryStart,
                    EditField::End => InputMode::EditEntryEnd,
                };
                None
            }
            Message::EditPrevField => {
                self.edit_field = match self.edit_field {
                    EditField::Project => EditField::End,
                    EditField::Description => EditField::Project,
                    EditField::Start => EditField::Description,
                    EditField::End => EditField::Start,
                };
                self.input_mode = match self.edit_field {
                    EditField::Project => InputMode::EditEntryProject,
                    EditField::Description => InputMode::EditEntryDescription,
                    EditField::Start => InputMode::EditEntryStart,
                    EditField::End => InputMode::EditEntryEnd,
                };
                None
            }
            Message::EditFieldInput(c) => {
                match self.edit_field {
                    EditField::Project => self.edit_project_input.push(c),
                    EditField::Description => self.edit_description_input.push(c),
                    EditField::Start => self.edit_start_input.push(c),
                    EditField::End => self.edit_end_input.push(c),
                }
                None
            }
            Message::EditFieldBackspace => {
                match self.edit_field {
                    EditField::Project => {
                        self.edit_project_input.pop();
                    }
                    EditField::Description => {
                        self.edit_description_input.pop();
                    }
                    EditField::Start => {
                        self.edit_start_input.pop();
                    }
                    EditField::End => {
                        self.edit_end_input.pop();
                    }
                }
                None
            }
            Message::SaveEditEntry => {
                if let Some(mut entry) = self.editing_entry.take() {
                    entry.project = self.edit_project_input.clone();
                    entry.description = self.edit_description_input.clone();

                    // Parse start time
                    if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(
                        &self.edit_start_input,
                        "%Y-%m-%d %H:%M",
                    ) {
                        entry.start = Local
                            .from_local_datetime(&parsed)
                            .single()
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or(entry.start);
                    }

                    // Parse end time
                    if self.edit_end_input.is_empty() {
                        entry.end = None;
                    } else if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(
                        &self.edit_end_input,
                        "%Y-%m-%d %H:%M",
                    ) {
                        entry.end = Local
                            .from_local_datetime(&parsed)
                            .single()
                            .map(|dt| dt.with_timezone(&Utc));
                    }

                    if db.update_entry(&entry).is_ok() {
                        self.status_message = Some(format!("Entry {} updated", entry.id));
                    } else {
                        self.status_message = Some("Failed to update entry".to_string());
                    }
                }
                self.input_mode = InputMode::Normal;
                self.edit_field = EditField::Project;
                Some(Message::RefreshEntries)
            }
            Message::CancelEditEntry => {
                self.editing_entry = None;
                self.input_mode = InputMode::Normal;
                self.edit_field = EditField::Project;
                None
            }

            // Bill entry
            Message::MarkEntryBilled(id) => {
                if db.mark_billed(id).is_ok() {
                    self.status_message = Some(format!("Entry {} marked as billed", id));
                    return Some(Message::RefreshEntries);
                }
                None
            }
            Message::UnbillEntry(id) => {
                if db.unmark_billed(id).is_ok() {
                    self.status_message = Some(format!("Entry {} unbilled", id));
                    return Some(Message::RefreshEntries);
                }
                None
            }

            // Invoice mode navigation
            Message::NextInvoiceMode => {
                if self.invoice_mode != InvoiceMode::SelectEntries {
                    self.invoice_mode_index = (self.invoice_mode_index + 1) % 4;
                    self.invoice_mode = match self.invoice_mode_index {
                        0 => InvoiceMode::CurrentMonth,
                        1 => InvoiceMode::PriorMonth,
                        2 => InvoiceMode::CustomRange,
                        _ => InvoiceMode::SelectEntries,
                    };
                } else {
                    // In select entries mode, navigate entries
                    if !self.invoice_entries.is_empty() {
                        self.invoice_select_index =
                            (self.invoice_select_index + 1).min(self.invoice_entries.len() - 1);
                    }
                }
                None
            }
            Message::PrevInvoiceMode => {
                if self.invoice_mode != InvoiceMode::SelectEntries {
                    self.invoice_mode_index = if self.invoice_mode_index == 0 {
                        3
                    } else {
                        self.invoice_mode_index - 1
                    };
                    self.invoice_mode = match self.invoice_mode_index {
                        0 => InvoiceMode::CurrentMonth,
                        1 => InvoiceMode::PriorMonth,
                        2 => InvoiceMode::CustomRange,
                        _ => InvoiceMode::SelectEntries,
                    };
                } else {
                    self.invoice_select_index = self.invoice_select_index.saturating_sub(1);
                }
                None
            }
            Message::SelectInvoiceMode => {
                if self.invoice_mode == InvoiceMode::SelectEntries {
                    // Already in select mode, do nothing special
                } else if self.invoice_mode_index == 3 {
                    // Enter select entries mode
                    self.invoice_mode = InvoiceMode::SelectEntries;
                    self.refresh_invoice_entries(db);
                }
                None
            }
            Message::NextInvoiceEntry => {
                if !self.invoice_entries.is_empty() {
                    self.invoice_select_index =
                        (self.invoice_select_index + 1).min(self.invoice_entries.len() - 1);
                }
                None
            }
            Message::PrevInvoiceEntry => {
                self.invoice_select_index = self.invoice_select_index.saturating_sub(1);
                None
            }
            Message::ToggleEntrySelection(id) => {
                if self.selected_entry_ids.contains(&id) {
                    self.selected_entry_ids.retain(|&x| x != id);
                } else {
                    self.selected_entry_ids.push(id);
                }
                None
            }
            Message::GenerateInvoice => {
                self.generate_invoice(db);
                None
            }

            // Input mode
            Message::EnterInputMode(mode) => {
                self.input_mode = mode;
                None
            }
            Message::ExitInputMode => {
                self.input_mode = InputMode::Normal;
                if self.invoice_mode == InvoiceMode::SelectEntries {
                    self.invoice_mode = InvoiceMode::CurrentMonth;
                    self.invoice_mode_index = 0;
                }
                None
            }

            // UI
            Message::ToggleHelp => {
                self.show_help = !self.show_help;
                None
            }
            Message::ClearStatus => {
                self.status_message = None;
                None
            }
            Message::Tick => {
                // Refresh active timer from database to detect external changes
                // (e.g., timer started/stopped from menu bar)
                self.refresh_active_timer(db);
                None
            }

            // Data refresh
            Message::RefreshEntries => {
                self.refresh_entries(db);
                None
            }
            Message::RefreshActiveTimer => {
                self.refresh_active_timer(db);
                None
            }

            // Project rate actions
            Message::RefreshProjects => {
                self.projects = db.list_projects().unwrap_or_default();
                None
            }
            Message::SelectNextProject => {
                if !self.projects.is_empty() {
                    self.selected_project_index =
                        (self.selected_project_index + 1).min(self.projects.len() - 1);
                }
                None
            }
            Message::SelectPreviousProject => {
                self.selected_project_index = self.selected_project_index.saturating_sub(1);
                None
            }
            Message::EditProjectRate(id) => {
                if let Some(project) = self.projects.iter().find(|p| p.id == id) {
                    self.editing_project_rate = Some(id);
                    self.rate_input = project
                        .rate
                        .map(|r| format!("{:.2}", r))
                        .unwrap_or_default();
                    self.currency_input =
                        project.currency.clone().unwrap_or_else(|| "$".to_string());
                    self.input_mode = InputMode::EditingRate;
                }
                None
            }
            Message::UpdateRateInput(c) => {
                if c.is_ascii_digit() || (c == '.' && !self.rate_input.contains('.')) {
                    self.rate_input.push(c);
                }
                None
            }
            Message::UpdateCurrencyInput(c) => {
                self.currency_input.push(c);
                None
            }
            Message::DeleteRateChar => {
                self.rate_input.pop();
                None
            }
            Message::DeleteCurrencyChar => {
                self.currency_input.pop();
                None
            }
            Message::SaveProjectRate => {
                if let Some(project_id) = self.editing_project_rate.take() {
                    if let Some(project) = self.projects.iter().find(|p| p.id == project_id) {
                        let rate = self.rate_input.parse::<f64>().ok();
                        let currency = if self.currency_input.is_empty() {
                            None
                        } else {
                            Some(self.currency_input.as_str())
                        };

                        if db.set_project_rate(&project.name, rate, currency).is_ok() {
                            self.status_message =
                                Some(format!("Rate updated for '{}'", project.name));
                        }
                    }
                }
                self.input_mode = InputMode::Normal;
                self.rate_input.clear();
                self.currency_input.clear();
                Some(Message::RefreshProjects)
            }
            Message::CancelEditRate => {
                self.editing_project_rate = None;
                self.input_mode = InputMode::Normal;
                self.rate_input.clear();
                self.currency_input.clear();
                None
            }
            Message::ClearProjectRate(id) => {
                if let Some(project) = self.projects.iter().find(|p| p.id == id) {
                    if db.set_project_rate(&project.name, None, None).is_ok() {
                        self.status_message = Some(format!("Rate cleared for '{}'", project.name));
                        return Some(Message::RefreshProjects);
                    }
                }
                None
            }
        }
    }

    fn refresh_entries(&mut self, db: &Db) {
        let filter = if self.show_only_unbilled {
            Some(false)
        } else {
            None
        };
        self.entries = db.list(filter).unwrap_or_default();
        if self.selected_entry_index >= self.entries.len() && !self.entries.is_empty() {
            self.selected_entry_index = self.entries.len() - 1;
        }
    }

    fn refresh_active_timer(&mut self, db: &Db) {
        self.active_entry = db.get_active_entry().unwrap_or(None);
    }

    fn refresh_invoice_entries(&mut self, db: &Db) {
        // Get billed entries for invoice selection
        self.invoice_entries = db.list(Some(true)).unwrap_or_default();

        // Fetch project rates for invoice preview
        self.project_rates.clear();
        if let Ok(projects) = db.list_projects() {
            for proj in projects {
                if let Some(rate) = proj.rate {
                    let currency = proj.currency.unwrap_or_else(|| "$".to_string());
                    self.project_rates.insert(proj.name, (rate, currency));
                }
            }
        }
    }

    fn generate_invoice(&mut self, db: &Db) {
        let now = Utc::now();
        let entries = match &self.invoice_mode {
            InvoiceMode::CurrentMonth => {
                let start = Utc::now()
                    .with_day(1)
                    .unwrap()
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap();
                let end = now;
                db.list_by_date_range(start, end, Some(true))
                    .unwrap_or_default()
            }
            InvoiceMode::PriorMonth => {
                let first_of_current = now.with_day(1).unwrap();
                let last_of_prior = first_of_current - Duration::days(1);
                let start = last_of_prior
                    .with_day(1)
                    .unwrap()
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap();
                let end = first_of_current;
                db.list_by_date_range(start, end, Some(true))
                    .unwrap_or_default()
            }
            InvoiceMode::CustomRange => {
                if let (Some(start), Some(end)) = (self.custom_start_date, self.custom_end_date) {
                    let start_dt = start.and_hms_opt(0, 0, 0).unwrap().and_utc();
                    let end_dt = end.and_hms_opt(23, 59, 59).unwrap().and_utc();
                    db.list_by_date_range(start_dt, end_dt, Some(true))
                        .unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
            InvoiceMode::SelectEntries => self
                .invoice_entries
                .iter()
                .filter(|e| self.selected_entry_ids.contains(&e.id))
                .cloned()
                .collect(),
        };

        // Group entries by project
        let mut entries_by_project: HashMap<String, Vec<&Entry>> = HashMap::new();
        for entry in &entries {
            entries_by_project
                .entry(entry.project.clone())
                .or_insert_with(Vec::new)
                .push(entry);
        }

        // Generate invoice file
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let (year, month) = match &self.invoice_mode {
            InvoiceMode::CurrentMonth => (now.year(), now.month()),
            InvoiceMode::PriorMonth => {
                let first_of_current = now.with_day(1).unwrap();
                let last_of_prior = first_of_current - Duration::days(1);
                (last_of_prior.year(), last_of_prior.month())
            }
            InvoiceMode::CustomRange => {
                if let Some(start) = self.custom_start_date {
                    (start.year(), start.month())
                } else {
                    (now.year(), now.month())
                }
            }
            InvoiceMode::SelectEntries => (now.year(), now.month()),
        };

        let file_path = format!("{}/invoice_{}_{:02}.txt", home, year, month);

        let mut content = format!("Invoice for {}-{:02}\n", year, month);
        content.push_str("=================\n\n");

        let mut total = 0.0;
        for (project, proj_entries) in &entries_by_project {
            content.push_str(&format!("Project: {}\n", project));
            content.push_str(&format!("{}\n", "-".repeat(40)));

            let mut project_total = 0.0;
            for entry in proj_entries {
                if let Some(end) = entry.end {
                    let hours = (end - entry.start).num_seconds() as f64 / 3600.0;
                    let start_local = Local.from_utc_datetime(&entry.start.naive_utc());
                    let end_local = Local.from_utc_datetime(&end.naive_utc());

                    content.push_str(&format!(
                        "  {} | {} - {} | {:.2} hrs\n",
                        entry.description,
                        start_local.format("%Y-%m-%d %H:%M"),
                        end_local.format("%Y-%m-%d %H:%M"),
                        hours
                    ));
                    project_total += hours;
                }
            }
            content.push_str(&format!("  Subtotal: {:.2} hrs\n\n", project_total));
            total += project_total;
        }
        content.push_str(&format!("{}\n", "=".repeat(40)));
        content.push_str(&format!("Total: {:.2} hrs\n", total));

        if std::fs::write(&file_path, content).is_ok() {
            self.status_message = Some(format!("Invoice written to {}", file_path));
        } else {
            self.status_message = Some("Failed to write invoice".to_string());
        }
    }

    pub fn get_selected_entry(&self) -> Option<&Entry> {
        self.entries.get(self.selected_entry_index)
    }
}
