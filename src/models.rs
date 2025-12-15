use chrono::{DateTime, Utc};
use rusqlite::{Connection, Result, params};

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: i64,
    pub project: String,
    pub description: String,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub billed: bool,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub rate: Option<f64>,
    pub currency: Option<String>,
}

impl Project {
    /// Format the rate with currency for display (e.g., "$150.00/hr")
    pub fn formatted_rate(&self) -> Option<String> {
        match (&self.rate, &self.currency) {
            (Some(r), Some(c)) => Some(format!("{}{:.2}/hr", c, r)),
            (Some(r), None) => Some(format!("${:.2}/hr", r)),
            _ => None,
        }
    }
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project TEXT NOT NULL,
            description TEXT NOT NULL,
            start TEXT NOT NULL,
            end TEXT,
            billed INTEGER NOT NULL DEFAULT 0
        )",
        params![],
    )?;
    Ok(())
}

pub fn init_projects_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            rate TEXT,
            currency TEXT DEFAULT '$'
        )",
        params![],
    )?;
    Ok(())
}

/// Pomodoro timer configuration
#[derive(Debug, Clone)]
pub struct PomodoroConfig {
    pub enabled: bool,
    pub work_duration: i32,      // minutes (default: 45)
    pub short_break: i32,        // minutes (default: 15)
    pub long_break: i32,         // minutes (default: 60)
    pub cycles_before_long: i32, // count (default: 4)
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            work_duration: 45,
            short_break: 15,
            long_break: 60,
            cycles_before_long: 4,
        }
    }
}

impl PomodoroConfig {
    /// Format config for display
    pub fn format_status(&self) -> String {
        if self.enabled {
            format!(
                "ON ({}m work, {}m short break, {}m long break after {} cycles)",
                self.work_duration, self.short_break, self.long_break, self.cycles_before_long
            )
        } else {
            "OFF".to_string()
        }
    }
}

pub fn init_pomodoro_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS pomodoro_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            enabled INTEGER NOT NULL DEFAULT 0,
            work_duration INTEGER NOT NULL DEFAULT 45,
            short_break INTEGER NOT NULL DEFAULT 15,
            long_break INTEGER NOT NULL DEFAULT 60,
            cycles_before_long INTEGER NOT NULL DEFAULT 4
        )",
        params![],
    )?;
    // Insert default row if not exists
    conn.execute(
        "INSERT OR IGNORE INTO pomodoro_config (id, enabled, work_duration, short_break, long_break, cycles_before_long)
         VALUES (1, 0, 45, 15, 60, 4)",
        params![],
    )?;
    Ok(())
}

/// Invoice settings (your business info)
#[derive(Debug, Clone, Default)]
pub struct InvoiceSettings {
    pub business_name: String,
    pub address_street: String,
    pub address_city: String,
    pub address_state: String,
    pub address_postal: String,
    pub address_country: String,
    pub email: String,
    pub phone: String,
    pub tax_id: String,
    pub payment_instructions: String,
    pub default_payment_terms: String,
    pub default_tax_rate: f64,
}

impl InvoiceSettings {
    pub fn formatted_address(&self) -> String {
        let mut parts = Vec::new();
        if !self.address_street.is_empty() {
            parts.push(self.address_street.clone());
        }
        let city_state_postal = [
            self.address_city.as_str(),
            self.address_state.as_str(),
            self.address_postal.as_str(),
        ]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
        if !city_state_postal.is_empty() {
            parts.push(city_state_postal);
        }
        if !self.address_country.is_empty() {
            parts.push(self.address_country.clone());
        }
        parts.join("\n")
    }
}

pub fn init_invoice_settings_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS invoice_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            business_name TEXT NOT NULL DEFAULT '',
            address_street TEXT NOT NULL DEFAULT '',
            address_city TEXT NOT NULL DEFAULT '',
            address_state TEXT NOT NULL DEFAULT '',
            address_postal TEXT NOT NULL DEFAULT '',
            address_country TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL DEFAULT '',
            phone TEXT NOT NULL DEFAULT '',
            tax_id TEXT NOT NULL DEFAULT '',
            payment_instructions TEXT NOT NULL DEFAULT '',
            default_payment_terms TEXT NOT NULL DEFAULT 'Due on receipt',
            default_tax_rate REAL NOT NULL DEFAULT 0.0
        )",
        params![],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO invoice_settings (id) VALUES (1)",
        params![],
    )?;
    Ok(())
}

/// Client information for invoicing
#[derive(Debug, Clone, Default)]
pub struct Client {
    pub id: i64,
    pub name: String,
    pub contact_person: String,
    pub address_street: String,
    pub address_city: String,
    pub address_state: String,
    pub address_postal: String,
    pub address_country: String,
    pub email: String,
}

impl Client {
    pub fn formatted_address(&self) -> String {
        let mut parts = Vec::new();
        if !self.address_street.is_empty() {
            parts.push(self.address_street.clone());
        }
        let city_state_postal = [
            self.address_city.as_str(),
            self.address_state.as_str(),
            self.address_postal.as_str(),
        ]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
        if !city_state_postal.is_empty() {
            parts.push(city_state_postal);
        }
        if !self.address_country.is_empty() {
            parts.push(self.address_country.clone());
        }
        parts.join("\n")
    }
}

pub fn init_clients_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            contact_person TEXT NOT NULL DEFAULT '',
            address_street TEXT NOT NULL DEFAULT '',
            address_city TEXT NOT NULL DEFAULT '',
            address_state TEXT NOT NULL DEFAULT '',
            address_postal TEXT NOT NULL DEFAULT '',
            address_country TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL DEFAULT ''
        )",
        params![],
    )?;
    Ok(())
}

/// Invoice record for tracking issued invoices
#[derive(Debug, Clone)]
pub struct Invoice {
    pub id: i64,
    pub invoice_number: i64,
    pub client_id: Option<i64>,
    pub date_issued: String,
    pub due_date: String,
    pub subtotal: f64,
    pub tax_rate: f64,
    pub tax_amount: f64,
    pub total: f64,
    pub file_path: String,
}

pub fn init_invoices_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS invoices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            invoice_number INTEGER NOT NULL UNIQUE,
            client_id INTEGER,
            date_issued TEXT NOT NULL,
            due_date TEXT NOT NULL,
            subtotal REAL NOT NULL,
            tax_rate REAL NOT NULL,
            tax_amount REAL NOT NULL,
            total REAL NOT NULL,
            file_path TEXT NOT NULL,
            FOREIGN KEY (client_id) REFERENCES clients(id)
        )",
        params![],
    )?;
    Ok(())
}
