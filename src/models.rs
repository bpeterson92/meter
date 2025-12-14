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
