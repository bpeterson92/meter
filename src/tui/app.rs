use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike, Utc};
use std::collections::HashMap;

use crate::db::Db;
use crate::invoice::{InvoiceParams, ProjectRate, write_invoice};
use crate::models::{Client, Entry, InvoiceSettings, PomodoroConfig, Project};
use crate::notification;

/// The active screen/view in the TUI
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Screen {
    #[default]
    Timer,
    Entries,
    Invoice,
    Projects,
    Pomodoro,
    Clients,
    Settings,
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
    // Pomodoro editing modes
    EditingPomodoroWork,
    EditingPomodoroShortBreak,
    EditingPomodoroLongBreak,
    EditingPomodoroCycles,
    // Client editing modes
    EditingClient,
    // Invoice settings editing modes
    EditingSettings,
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

/// Pomodoro timer state
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PomodoroState {
    #[default]
    Idle, // No timer running or Pomodoro disabled
    Working,       // In work period
    WorkComplete,  // Work done, waiting for user to start break
    OnBreak,       // In break period
    BreakComplete, // Break done, waiting for user to resume work
}

/// Which field is selected in the Pomodoro config screen
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PomodoroField {
    #[default]
    Enabled,
    WorkDuration,
    ShortBreak,
    LongBreak,
    Cycles,
}

/// Which field is selected in the client edit dialog
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ClientField {
    #[default]
    Name,
    Contact,
    Street,
    City,
    State,
    Postal,
    Country,
    Email,
}

/// Which field is selected in the settings edit dialog
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SettingsField {
    #[default]
    BusinessName,
    Street,
    City,
    State,
    Postal,
    Country,
    Email,
    Phone,
    TaxId,
    PaymentTerms,
    DefaultTaxRate,
    PaymentInstructions,
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
    pub project_rates: HashMap<String, ProjectRate>,

    // Pomodoro state
    pub pomodoro_config: PomodoroConfig,
    pub pomodoro_state: PomodoroState,
    pub pomodoro_cycles_completed: u32,
    pub pomodoro_interval_start: Option<DateTime<Utc>>,
    /// Stores the project/description for resuming after break
    pub pomodoro_last_project: Option<String>,
    pub pomodoro_last_description: Option<String>,

    // Pomodoro config editing state
    pub pomodoro_field: PomodoroField,
    pub pomodoro_work_input: String,
    pub pomodoro_short_break_input: String,
    pub pomodoro_long_break_input: String,
    pub pomodoro_cycles_input: String,

    // Clients state
    pub clients: Vec<Client>,
    pub selected_client_index: usize,
    pub selected_invoice_client: Option<i64>,

    // Client editing state
    pub editing_client: Option<Client>,
    pub adding_new_client: bool,
    pub confirm_delete_client: Option<i64>,
    pub client_field: ClientField,
    pub client_name_input: String,
    pub client_contact_input: String,
    pub client_street_input: String,
    pub client_city_input: String,
    pub client_state_input: String,
    pub client_postal_input: String,
    pub client_country_input: String,
    pub client_email_input: String,

    // Invoice settings state
    pub invoice_settings: InvoiceSettings,

    // Settings editing state
    pub editing_settings: bool,
    pub settings_field: SettingsField,
    pub settings_business_name_input: String,
    pub settings_street_input: String,
    pub settings_city_input: String,
    pub settings_state_input: String,
    pub settings_postal_input: String,
    pub settings_country_input: String,
    pub settings_email_input: String,
    pub settings_phone_input: String,
    pub settings_tax_id_input: String,
    pub settings_payment_terms_input: String,
    pub settings_default_tax_rate_input: String,
    pub settings_payment_instructions_input: String,
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

    // Pomodoro actions
    TogglePomodoroMode,
    AcknowledgePomodoro, // User presses key to start break or resume work
    RefreshPomodoroConfig,

    // Pomodoro config screen actions
    PomodoroNextField,
    PomodoroPrevField,
    PomodoroFieldInput(char),
    PomodoroFieldBackspace,
    SavePomodoroConfig,
    CancelPomodoroEdit,

    // Client actions
    RefreshClients,
    SelectNextClient,
    SelectPreviousClient,
    SelectInvoiceClient(Option<i64>),
    CycleInvoiceClient,

    // Client editing actions
    AddClient,
    EditClient(i64),
    DeleteClient(i64),
    ConfirmDeleteClient,
    CancelDeleteClient,
    ClientNextField,
    ClientPrevField,
    ClientFieldInput(char),
    ClientFieldBackspace,
    SaveClient,
    CancelEditClient,

    // Settings editing actions
    EditSettings,
    SettingsNextField,
    SettingsPrevField,
    SettingsFieldInput(char),
    SettingsFieldBackspace,
    SaveSettings,
    CancelEditSettings,
}

impl App {
    pub fn new(db: &Db) -> Self {
        let mut app = App::default();
        app.description_input = "Work session".to_string();
        app.refresh_entries(db);
        app.refresh_active_timer(db);
        app.refresh_pomodoro_config(db);
        app.refresh_clients(db);
        app.refresh_invoice_settings(db);

        // If there's an active timer and Pomodoro is enabled, set state to Working
        if app.active_entry.is_some() && app.pomodoro_config.enabled {
            app.pomodoro_state = PomodoroState::Working;
            app.pomodoro_interval_start = app.active_entry.as_ref().map(|e| e.start);
        }

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
                if screen == Screen::Pomodoro {
                    self.refresh_pomodoro_config(db);
                    self.load_pomodoro_inputs();
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
                    if db
                        .start_timer(&self.project_input, &self.description_input)
                        .is_ok()
                    {
                        // Store project info for Pomodoro resume
                        self.pomodoro_last_project = Some(self.project_input.clone());
                        self.pomodoro_last_description = Some(self.description_input.clone());

                        self.project_input.clear();
                        self.description_input = "Work session".to_string();
                        self.status_message = Some("Timer started".to_string());
                        self.input_mode = InputMode::Normal;

                        // If Pomodoro enabled, set state to Working
                        if self.pomodoro_config.enabled {
                            self.pomodoro_state = PomodoroState::Working;
                            self.pomodoro_interval_start = Some(Utc::now());
                        }

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

                        // Reset Pomodoro state
                        self.pomodoro_state = PomodoroState::Idle;
                        self.pomodoro_interval_start = None;
                        self.pomodoro_cycles_completed = 0;

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

                // Check Pomodoro state transitions
                if self.pomodoro_config.enabled {
                    if let Some(interval_start) = self.pomodoro_interval_start {
                        let elapsed_secs = (Utc::now() - interval_start).num_seconds();

                        match self.pomodoro_state {
                            PomodoroState::Working => {
                                let work_secs = self.pomodoro_config.work_duration as i64 * 60;
                                if elapsed_secs >= work_secs {
                                    // Work period complete - stop the timer
                                    if let Some(ref entry) = self.active_entry {
                                        self.pomodoro_last_project = Some(entry.project.clone());
                                        self.pomodoro_last_description =
                                            Some(entry.description.clone());
                                    }
                                    let _ = db.stop_active_timer();
                                    self.active_entry = None;
                                    self.pomodoro_state = PomodoroState::WorkComplete;
                                    self.pomodoro_interval_start = None;
                                    notification::notify_work_complete();
                                    self.status_message = Some(
                                        "Work period complete! Press [Space] to start break"
                                            .to_string(),
                                    );
                                }
                            }
                            PomodoroState::OnBreak => {
                                let break_secs = self.get_current_break_duration() as i64 * 60;
                                if elapsed_secs >= break_secs {
                                    // Break complete
                                    self.pomodoro_state = PomodoroState::BreakComplete;
                                    self.pomodoro_interval_start = None;
                                    notification::notify_break_complete();
                                    self.status_message = Some(
                                        "Break complete! Press [s] to resume work".to_string(),
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }

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

            // Pomodoro actions
            Message::TogglePomodoroMode => {
                self.pomodoro_config.enabled = !self.pomodoro_config.enabled;
                let _ = db.set_pomodoro_enabled(self.pomodoro_config.enabled);

                if self.pomodoro_config.enabled {
                    self.status_message = Some("Pomodoro mode enabled".to_string());
                    // If timer is already running, set state to Working
                    if self.active_entry.is_some() {
                        self.pomodoro_state = PomodoroState::Working;
                        self.pomodoro_interval_start = Some(Utc::now());
                    }
                } else {
                    self.status_message = Some("Pomodoro mode disabled".to_string());
                    // Reset Pomodoro state but keep timer running
                    self.pomodoro_state = PomodoroState::Idle;
                    self.pomodoro_interval_start = None;
                    self.pomodoro_cycles_completed = 0;
                }
                None
            }

            Message::AcknowledgePomodoro => {
                match self.pomodoro_state {
                    PomodoroState::WorkComplete => {
                        // Start break
                        self.pomodoro_state = PomodoroState::OnBreak;
                        self.pomodoro_interval_start = Some(Utc::now());
                        let break_type = if self.is_long_break_next() {
                            "long"
                        } else {
                            "short"
                        };
                        let break_mins = self.get_current_break_duration();
                        self.status_message = Some(format!(
                            "Starting {} break ({} min)",
                            break_type, break_mins
                        ));
                    }
                    PomodoroState::BreakComplete => {
                        // Increment cycle count
                        self.pomodoro_cycles_completed += 1;
                        if self.pomodoro_cycles_completed
                            >= self.pomodoro_config.cycles_before_long as u32
                        {
                            self.pomodoro_cycles_completed = 0;
                        }

                        // Return to Idle - user must manually start next work period
                        self.pomodoro_state = PomodoroState::Idle;
                        self.pomodoro_interval_start = None;

                        // Pre-fill the project input with the last project
                        if let Some(ref proj) = self.pomodoro_last_project {
                            self.project_input = proj.clone();
                        }
                        if let Some(ref desc) = self.pomodoro_last_description {
                            self.description_input = desc.clone();
                        }

                        self.status_message = Some("Ready to start next work period".to_string());
                    }
                    _ => {}
                }
                None
            }

            Message::RefreshPomodoroConfig => {
                self.refresh_pomodoro_config(db);
                None
            }

            // Pomodoro config screen actions
            Message::PomodoroNextField => {
                self.pomodoro_field = match self.pomodoro_field {
                    PomodoroField::Enabled => PomodoroField::WorkDuration,
                    PomodoroField::WorkDuration => PomodoroField::ShortBreak,
                    PomodoroField::ShortBreak => PomodoroField::LongBreak,
                    PomodoroField::LongBreak => PomodoroField::Cycles,
                    PomodoroField::Cycles => PomodoroField::Enabled,
                };
                self.input_mode = match self.pomodoro_field {
                    PomodoroField::Enabled => InputMode::Normal,
                    PomodoroField::WorkDuration => InputMode::EditingPomodoroWork,
                    PomodoroField::ShortBreak => InputMode::EditingPomodoroShortBreak,
                    PomodoroField::LongBreak => InputMode::EditingPomodoroLongBreak,
                    PomodoroField::Cycles => InputMode::EditingPomodoroCycles,
                };
                None
            }
            Message::PomodoroPrevField => {
                self.pomodoro_field = match self.pomodoro_field {
                    PomodoroField::Enabled => PomodoroField::Cycles,
                    PomodoroField::WorkDuration => PomodoroField::Enabled,
                    PomodoroField::ShortBreak => PomodoroField::WorkDuration,
                    PomodoroField::LongBreak => PomodoroField::ShortBreak,
                    PomodoroField::Cycles => PomodoroField::LongBreak,
                };
                self.input_mode = match self.pomodoro_field {
                    PomodoroField::Enabled => InputMode::Normal,
                    PomodoroField::WorkDuration => InputMode::EditingPomodoroWork,
                    PomodoroField::ShortBreak => InputMode::EditingPomodoroShortBreak,
                    PomodoroField::LongBreak => InputMode::EditingPomodoroLongBreak,
                    PomodoroField::Cycles => InputMode::EditingPomodoroCycles,
                };
                None
            }
            Message::PomodoroFieldInput(c) => {
                if c.is_ascii_digit() {
                    match self.pomodoro_field {
                        PomodoroField::WorkDuration => self.pomodoro_work_input.push(c),
                        PomodoroField::ShortBreak => self.pomodoro_short_break_input.push(c),
                        PomodoroField::LongBreak => self.pomodoro_long_break_input.push(c),
                        PomodoroField::Cycles => self.pomodoro_cycles_input.push(c),
                        _ => {}
                    }
                }
                None
            }
            Message::PomodoroFieldBackspace => {
                match self.pomodoro_field {
                    PomodoroField::WorkDuration => {
                        self.pomodoro_work_input.pop();
                    }
                    PomodoroField::ShortBreak => {
                        self.pomodoro_short_break_input.pop();
                    }
                    PomodoroField::LongBreak => {
                        self.pomodoro_long_break_input.pop();
                    }
                    PomodoroField::Cycles => {
                        self.pomodoro_cycles_input.pop();
                    }
                    _ => {}
                }
                None
            }
            Message::SavePomodoroConfig => {
                // Parse inputs and update config
                if let Ok(work) = self.pomodoro_work_input.parse::<i32>() {
                    if work > 0 {
                        self.pomodoro_config.work_duration = work;
                    }
                }
                if let Ok(short) = self.pomodoro_short_break_input.parse::<i32>() {
                    if short > 0 {
                        self.pomodoro_config.short_break = short;
                    }
                }
                if let Ok(long) = self.pomodoro_long_break_input.parse::<i32>() {
                    if long > 0 {
                        self.pomodoro_config.long_break = long;
                    }
                }
                if let Ok(cycles) = self.pomodoro_cycles_input.parse::<i32>() {
                    if cycles > 0 {
                        self.pomodoro_config.cycles_before_long = cycles;
                    }
                }

                let _ = db.set_pomodoro_config(&self.pomodoro_config);
                self.status_message = Some("Pomodoro settings saved".to_string());
                self.input_mode = InputMode::Normal;
                self.pomodoro_field = PomodoroField::Enabled;
                None
            }
            Message::CancelPomodoroEdit => {
                self.load_pomodoro_inputs();
                self.input_mode = InputMode::Normal;
                self.pomodoro_field = PomodoroField::Enabled;
                None
            }

            // Client actions
            Message::RefreshClients => {
                self.refresh_clients(db);
                None
            }
            Message::SelectNextClient => {
                if !self.clients.is_empty() {
                    self.selected_client_index =
                        (self.selected_client_index + 1) % self.clients.len();
                }
                None
            }
            Message::SelectPreviousClient => {
                if !self.clients.is_empty() {
                    self.selected_client_index = if self.selected_client_index == 0 {
                        self.clients.len() - 1
                    } else {
                        self.selected_client_index - 1
                    };
                }
                None
            }
            Message::SelectInvoiceClient(client_id) => {
                self.selected_invoice_client = client_id;
                None
            }
            Message::CycleInvoiceClient => {
                // Cycle through: None -> Client 1 -> Client 2 -> ... -> None
                if self.clients.is_empty() {
                    self.selected_invoice_client = None;
                } else {
                    match self.selected_invoice_client {
                        None => {
                            self.selected_invoice_client = Some(self.clients[0].id);
                        }
                        Some(current_id) => {
                            // Find current index and move to next
                            if let Some(idx) = self.clients.iter().position(|c| c.id == current_id)
                            {
                                if idx + 1 < self.clients.len() {
                                    self.selected_invoice_client = Some(self.clients[idx + 1].id);
                                } else {
                                    self.selected_invoice_client = None;
                                }
                            } else {
                                self.selected_invoice_client = None;
                            }
                        }
                    }
                }
                None
            }

            // Client editing actions
            Message::AddClient => {
                self.editing_client = Some(Client::default());
                self.adding_new_client = true;
                self.client_field = ClientField::Name;
                self.clear_client_inputs();
                self.input_mode = InputMode::EditingClient;
                None
            }
            Message::EditClient(id) => {
                if let Some(client) = self.clients.iter().find(|c| c.id == id).cloned() {
                    self.editing_client = Some(client.clone());
                    self.adding_new_client = false;
                    self.client_field = ClientField::Name;
                    self.load_client_inputs(&client);
                    self.input_mode = InputMode::EditingClient;
                }
                None
            }
            Message::DeleteClient(id) => {
                self.confirm_delete_client = Some(id);
                None
            }
            Message::ConfirmDeleteClient => {
                if let Some(id) = self.confirm_delete_client.take() {
                    if db.delete_client(id).is_ok() {
                        self.status_message = Some(format!("Client {} deleted", id));
                        if self.selected_client_index > 0 {
                            self.selected_client_index -= 1;
                        }
                        return Some(Message::RefreshClients);
                    }
                }
                None
            }
            Message::CancelDeleteClient => {
                self.confirm_delete_client = None;
                None
            }
            Message::ClientNextField => {
                self.client_field = match self.client_field {
                    ClientField::Name => ClientField::Contact,
                    ClientField::Contact => ClientField::Street,
                    ClientField::Street => ClientField::City,
                    ClientField::City => ClientField::State,
                    ClientField::State => ClientField::Postal,
                    ClientField::Postal => ClientField::Country,
                    ClientField::Country => ClientField::Email,
                    ClientField::Email => ClientField::Name,
                };
                None
            }
            Message::ClientPrevField => {
                self.client_field = match self.client_field {
                    ClientField::Name => ClientField::Email,
                    ClientField::Contact => ClientField::Name,
                    ClientField::Street => ClientField::Contact,
                    ClientField::City => ClientField::Street,
                    ClientField::State => ClientField::City,
                    ClientField::Postal => ClientField::State,
                    ClientField::Country => ClientField::Postal,
                    ClientField::Email => ClientField::Country,
                };
                None
            }
            Message::ClientFieldInput(c) => {
                match self.client_field {
                    ClientField::Name => self.client_name_input.push(c),
                    ClientField::Contact => self.client_contact_input.push(c),
                    ClientField::Street => self.client_street_input.push(c),
                    ClientField::City => self.client_city_input.push(c),
                    ClientField::State => self.client_state_input.push(c),
                    ClientField::Postal => self.client_postal_input.push(c),
                    ClientField::Country => self.client_country_input.push(c),
                    ClientField::Email => self.client_email_input.push(c),
                }
                None
            }
            Message::ClientFieldBackspace => {
                match self.client_field {
                    ClientField::Name => {
                        self.client_name_input.pop();
                    }
                    ClientField::Contact => {
                        self.client_contact_input.pop();
                    }
                    ClientField::Street => {
                        self.client_street_input.pop();
                    }
                    ClientField::City => {
                        self.client_city_input.pop();
                    }
                    ClientField::State => {
                        self.client_state_input.pop();
                    }
                    ClientField::Postal => {
                        self.client_postal_input.pop();
                    }
                    ClientField::Country => {
                        self.client_country_input.pop();
                    }
                    ClientField::Email => {
                        self.client_email_input.pop();
                    }
                }
                None
            }
            Message::SaveClient => {
                let client = Client {
                    id: self.editing_client.as_ref().map(|c| c.id).unwrap_or(0),
                    name: self.client_name_input.clone(),
                    contact_person: self.client_contact_input.clone(),
                    address_street: self.client_street_input.clone(),
                    address_city: self.client_city_input.clone(),
                    address_state: self.client_state_input.clone(),
                    address_postal: self.client_postal_input.clone(),
                    address_country: self.client_country_input.clone(),
                    email: self.client_email_input.clone(),
                };

                if self.adding_new_client {
                    if db.add_client(&client).is_ok() {
                        self.status_message = Some(format!("Client '{}' added", client.name));
                    } else {
                        self.status_message = Some("Failed to add client".to_string());
                    }
                } else if db.update_client(&client).is_ok() {
                    self.status_message = Some(format!("Client '{}' updated", client.name));
                } else {
                    self.status_message = Some("Failed to update client".to_string());
                }

                self.editing_client = None;
                self.adding_new_client = false;
                self.input_mode = InputMode::Normal;
                self.clear_client_inputs();
                Some(Message::RefreshClients)
            }
            Message::CancelEditClient => {
                self.editing_client = None;
                self.adding_new_client = false;
                self.input_mode = InputMode::Normal;
                self.clear_client_inputs();
                None
            }

            // Settings editing actions
            Message::EditSettings => {
                self.editing_settings = true;
                self.settings_field = SettingsField::BusinessName;
                self.load_settings_inputs();
                self.input_mode = InputMode::EditingSettings;
                None
            }
            Message::SettingsNextField => {
                self.settings_field = match self.settings_field {
                    SettingsField::BusinessName => SettingsField::Street,
                    SettingsField::Street => SettingsField::City,
                    SettingsField::City => SettingsField::State,
                    SettingsField::State => SettingsField::Postal,
                    SettingsField::Postal => SettingsField::Country,
                    SettingsField::Country => SettingsField::Email,
                    SettingsField::Email => SettingsField::Phone,
                    SettingsField::Phone => SettingsField::TaxId,
                    SettingsField::TaxId => SettingsField::PaymentTerms,
                    SettingsField::PaymentTerms => SettingsField::DefaultTaxRate,
                    SettingsField::DefaultTaxRate => SettingsField::PaymentInstructions,
                    SettingsField::PaymentInstructions => SettingsField::BusinessName,
                };
                None
            }
            Message::SettingsPrevField => {
                self.settings_field = match self.settings_field {
                    SettingsField::BusinessName => SettingsField::PaymentInstructions,
                    SettingsField::Street => SettingsField::BusinessName,
                    SettingsField::City => SettingsField::Street,
                    SettingsField::State => SettingsField::City,
                    SettingsField::Postal => SettingsField::State,
                    SettingsField::Country => SettingsField::Postal,
                    SettingsField::Email => SettingsField::Country,
                    SettingsField::Phone => SettingsField::Email,
                    SettingsField::TaxId => SettingsField::Phone,
                    SettingsField::PaymentTerms => SettingsField::TaxId,
                    SettingsField::DefaultTaxRate => SettingsField::PaymentTerms,
                    SettingsField::PaymentInstructions => SettingsField::DefaultTaxRate,
                };
                None
            }
            Message::SettingsFieldInput(c) => {
                match self.settings_field {
                    SettingsField::BusinessName => self.settings_business_name_input.push(c),
                    SettingsField::Street => self.settings_street_input.push(c),
                    SettingsField::City => self.settings_city_input.push(c),
                    SettingsField::State => self.settings_state_input.push(c),
                    SettingsField::Postal => self.settings_postal_input.push(c),
                    SettingsField::Country => self.settings_country_input.push(c),
                    SettingsField::Email => self.settings_email_input.push(c),
                    SettingsField::Phone => self.settings_phone_input.push(c),
                    SettingsField::TaxId => self.settings_tax_id_input.push(c),
                    SettingsField::PaymentTerms => self.settings_payment_terms_input.push(c),
                    SettingsField::DefaultTaxRate => {
                        if c.is_ascii_digit()
                            || (c == '.' && !self.settings_default_tax_rate_input.contains('.'))
                        {
                            self.settings_default_tax_rate_input.push(c);
                        }
                    }
                    SettingsField::PaymentInstructions => {
                        self.settings_payment_instructions_input.push(c)
                    }
                }
                None
            }
            Message::SettingsFieldBackspace => {
                match self.settings_field {
                    SettingsField::BusinessName => {
                        self.settings_business_name_input.pop();
                    }
                    SettingsField::Street => {
                        self.settings_street_input.pop();
                    }
                    SettingsField::City => {
                        self.settings_city_input.pop();
                    }
                    SettingsField::State => {
                        self.settings_state_input.pop();
                    }
                    SettingsField::Postal => {
                        self.settings_postal_input.pop();
                    }
                    SettingsField::Country => {
                        self.settings_country_input.pop();
                    }
                    SettingsField::Email => {
                        self.settings_email_input.pop();
                    }
                    SettingsField::Phone => {
                        self.settings_phone_input.pop();
                    }
                    SettingsField::TaxId => {
                        self.settings_tax_id_input.pop();
                    }
                    SettingsField::PaymentTerms => {
                        self.settings_payment_terms_input.pop();
                    }
                    SettingsField::DefaultTaxRate => {
                        self.settings_default_tax_rate_input.pop();
                    }
                    SettingsField::PaymentInstructions => {
                        self.settings_payment_instructions_input.pop();
                    }
                }
                None
            }
            Message::SaveSettings => {
                let settings = InvoiceSettings {
                    business_name: self.settings_business_name_input.clone(),
                    address_street: self.settings_street_input.clone(),
                    address_city: self.settings_city_input.clone(),
                    address_state: self.settings_state_input.clone(),
                    address_postal: self.settings_postal_input.clone(),
                    address_country: self.settings_country_input.clone(),
                    email: self.settings_email_input.clone(),
                    phone: self.settings_phone_input.clone(),
                    tax_id: self.settings_tax_id_input.clone(),
                    payment_instructions: self.settings_payment_instructions_input.clone(),
                    default_payment_terms: self.settings_payment_terms_input.clone(),
                    default_tax_rate: self.settings_default_tax_rate_input.parse().unwrap_or(0.0),
                };

                if db.set_invoice_settings(&settings).is_ok() {
                    self.invoice_settings = settings;
                    self.status_message = Some("Settings saved".to_string());
                } else {
                    self.status_message = Some("Failed to save settings".to_string());
                }

                self.editing_settings = false;
                self.input_mode = InputMode::Normal;
                None
            }
            Message::CancelEditSettings => {
                self.editing_settings = false;
                self.input_mode = InputMode::Normal;
                self.load_settings_inputs();
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
                    self.project_rates
                        .insert(proj.name, ProjectRate { rate, currency });
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

        // Determine year/month for invoice filename
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

        // Get invoice settings and next invoice number
        let settings = db.get_invoice_settings().unwrap_or_default();
        let invoice_number = db.get_next_invoice_number().unwrap_or(1);

        let params = InvoiceParams {
            entries: &entries,
            project_rates: &self.project_rates,
            year,
            month,
            invoice_number,
            settings: &settings,
            client: self.get_selected_invoice_client(),
            tax_rate: settings.default_tax_rate,
        };

        // Use shared invoice generation
        match write_invoice(&params) {
            Ok(result) => {
                // Record the invoice
                let invoice_record = crate::models::Invoice {
                    id: 0,
                    invoice_number,
                    client_id: self.selected_invoice_client,
                    date_issued: result.date_issued.clone(),
                    due_date: result.due_date.clone(),
                    subtotal: result.subtotal,
                    tax_rate: settings.default_tax_rate,
                    tax_amount: result.tax_amount,
                    total: result.total,
                    file_path: result.file_path.clone(),
                };
                let _ = db.record_invoice(&invoice_record);

                self.status_message = Some(format!(
                    "Invoice #{} written to {}",
                    invoice_number, result.file_path
                ));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to write invoice: {}", e));
            }
        }
    }

    pub fn get_selected_entry(&self) -> Option<&Entry> {
        self.entries.get(self.selected_entry_index)
    }

    fn refresh_pomodoro_config(&mut self, db: &Db) {
        self.pomodoro_config = db.get_pomodoro_config().unwrap_or_default();
    }

    fn refresh_clients(&mut self, db: &Db) {
        self.clients = db.list_clients().unwrap_or_default();
        if self.selected_client_index >= self.clients.len() && !self.clients.is_empty() {
            self.selected_client_index = self.clients.len() - 1;
        }
    }

    fn refresh_invoice_settings(&mut self, db: &Db) {
        self.invoice_settings = db.get_invoice_settings().unwrap_or_default();
    }

    /// Get the selected client for invoicing
    pub fn get_selected_invoice_client(&self) -> Option<&Client> {
        self.selected_invoice_client
            .and_then(|id| self.clients.iter().find(|c| c.id == id))
    }

    /// Check if the next break should be a long break
    pub fn is_long_break_next(&self) -> bool {
        (self.pomodoro_cycles_completed + 1) >= self.pomodoro_config.cycles_before_long as u32
    }

    /// Get the duration of the current/next break in minutes
    pub fn get_current_break_duration(&self) -> i32 {
        if self.is_long_break_next() {
            self.pomodoro_config.long_break
        } else {
            self.pomodoro_config.short_break
        }
    }

    /// Get remaining time in current Pomodoro interval (work or break) in seconds
    pub fn get_pomodoro_remaining_secs(&self) -> Option<i64> {
        let interval_start = self.pomodoro_interval_start?;
        let elapsed_secs = (Utc::now() - interval_start).num_seconds();

        let total_secs = match self.pomodoro_state {
            PomodoroState::Working => self.pomodoro_config.work_duration as i64 * 60,
            PomodoroState::OnBreak => self.get_current_break_duration() as i64 * 60,
            _ => return None,
        };

        Some((total_secs - elapsed_secs).max(0))
    }

    /// Load Pomodoro config values into input fields
    fn load_pomodoro_inputs(&mut self) {
        self.pomodoro_work_input = self.pomodoro_config.work_duration.to_string();
        self.pomodoro_short_break_input = self.pomodoro_config.short_break.to_string();
        self.pomodoro_long_break_input = self.pomodoro_config.long_break.to_string();
        self.pomodoro_cycles_input = self.pomodoro_config.cycles_before_long.to_string();
    }

    /// Load client values into input fields
    fn load_client_inputs(&mut self, client: &Client) {
        self.client_name_input = client.name.clone();
        self.client_contact_input = client.contact_person.clone();
        self.client_street_input = client.address_street.clone();
        self.client_city_input = client.address_city.clone();
        self.client_state_input = client.address_state.clone();
        self.client_postal_input = client.address_postal.clone();
        self.client_country_input = client.address_country.clone();
        self.client_email_input = client.email.clone();
    }

    /// Clear client input fields
    fn clear_client_inputs(&mut self) {
        self.client_name_input.clear();
        self.client_contact_input.clear();
        self.client_street_input.clear();
        self.client_city_input.clear();
        self.client_state_input.clear();
        self.client_postal_input.clear();
        self.client_country_input.clear();
        self.client_email_input.clear();
    }

    /// Load invoice settings values into input fields
    fn load_settings_inputs(&mut self) {
        self.settings_business_name_input = self.invoice_settings.business_name.clone();
        self.settings_street_input = self.invoice_settings.address_street.clone();
        self.settings_city_input = self.invoice_settings.address_city.clone();
        self.settings_state_input = self.invoice_settings.address_state.clone();
        self.settings_postal_input = self.invoice_settings.address_postal.clone();
        self.settings_country_input = self.invoice_settings.address_country.clone();
        self.settings_email_input = self.invoice_settings.email.clone();
        self.settings_phone_input = self.invoice_settings.phone.clone();
        self.settings_tax_id_input = self.invoice_settings.tax_id.clone();
        self.settings_payment_terms_input = self.invoice_settings.default_payment_terms.clone();
        self.settings_default_tax_rate_input = self.invoice_settings.default_tax_rate.to_string();
        self.settings_payment_instructions_input =
            self.invoice_settings.payment_instructions.clone();
    }

    /// Get the selected client for editing
    pub fn get_selected_client(&self) -> Option<&Client> {
        self.clients.get(self.selected_client_index)
    }
}
