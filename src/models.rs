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
