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
