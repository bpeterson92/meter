use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::models::Entry;

/// Wrapper around a SQLite connection.
/// The inner `Connection` is intentionally private; use the `conn()` method to obtain
/// a read‑only reference when you need to run custom queries.
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Create a new database connection.  The database file is created if it does not exist.
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    /// Read‑only reference to the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Insert a new time entry.
    pub fn insert(&self, entry: &Entry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO entries (project, description, start, end, billed)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                entry.project,
                entry.description,
                entry.start.to_rfc3339(),
                entry.end.map(|e| e.to_rfc3339()),
                if entry.billed { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    /// Retrieve all entries, optionally filtered by billed status.
    pub fn list(&self, billed: Option<bool>) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project, description, start, end, billed
             FROM entries
             ORDER BY start DESC",
        )?;
        let entries_iter = stmt.query_map(params![], |row| {
            Ok(Entry {
                id: row.get(0)?,
                project: row.get(1)?,
                description: row.get(2)?,
                start: DateTime::parse_from_rfc3339(row.get::<_, String>(3)?.as_str())
                    .unwrap()
                    .with_timezone(&Utc),
                end: match row.get::<_, Option<String>>(4)? {
                    Some(s) => Some(
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc),
                    ),
                    None => None,
                },
                billed: row.get::<_, i64>(5)? != 0,
            })
        })?;

        let mut entries = Vec::new();
        for e in entries_iter {
            let e = e?;
            if let Some(b) = billed {
                if e.billed != b {
                    continue;
                }
            }
            entries.push(e);
        }
        Ok(entries)
    }

    /// Delete an entry by ID.
    pub fn delete(&self, id: i64) -> Result<bool> {
        let rows_affected = self
            .conn
            .execute("DELETE FROM entries WHERE id = ?1", params![id])?;
        Ok(rows_affected > 0)
    }

    /// Get the active (unended) timer entry, if any.
    pub fn get_active_entry(&self) -> Result<Option<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project, description, start, end, billed
             FROM entries
             WHERE end IS NULL
             ORDER BY start DESC
             LIMIT 1",
        )?;

        let entry = stmt
            .query_row([], |row| {
                Ok(Entry {
                    id: row.get(0)?,
                    project: row.get(1)?,
                    description: row.get(2)?,
                    start: DateTime::parse_from_rfc3339(row.get::<_, String>(3)?.as_str())
                        .unwrap()
                        .with_timezone(&Utc),
                    end: None,
                    billed: row.get::<_, i64>(5)? != 0,
                })
            })
            .optional()?;

        Ok(entry)
    }

    /// Stop the active timer by setting its end time to now.
    pub fn stop_active_timer(&self) -> Result<Option<Entry>> {
        if let Some(entry) = self.get_active_entry()? {
            let now = Utc::now().to_rfc3339();
            self.conn.execute(
                "UPDATE entries SET end = ?1 WHERE id = ?2",
                params![now, entry.id],
            )?;
            self.get_entry_by_id(entry.id)
        } else {
            Ok(None)
        }
    }

    /// Get a single entry by ID.
    pub fn get_entry_by_id(&self, id: i64) -> Result<Option<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project, description, start, end, billed
             FROM entries
             WHERE id = ?1",
        )?;

        stmt.query_row(params![id], |row| {
            Ok(Entry {
                id: row.get(0)?,
                project: row.get(1)?,
                description: row.get(2)?,
                start: DateTime::parse_from_rfc3339(row.get::<_, String>(3)?.as_str())
                    .unwrap()
                    .with_timezone(&Utc),
                end: match row.get::<_, Option<String>>(4)? {
                    Some(s) => Some(
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc),
                    ),
                    None => None,
                },
                billed: row.get::<_, i64>(5)? != 0,
            })
        })
        .optional()
    }

    /// List entries within a date range, optionally filtered by billed status.
    pub fn list_by_date_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        billed: Option<bool>,
    ) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project, description, start, end, billed
             FROM entries
             WHERE end IS NOT NULL
               AND end >= ?1
               AND end <= ?2
             ORDER BY start DESC",
        )?;

        let entries_iter =
            stmt.query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
                Ok(Entry {
                    id: row.get(0)?,
                    project: row.get(1)?,
                    description: row.get(2)?,
                    start: DateTime::parse_from_rfc3339(row.get::<_, String>(3)?.as_str())
                        .unwrap()
                        .with_timezone(&Utc),
                    end: match row.get::<_, Option<String>>(4)? {
                        Some(s) => Some(
                            DateTime::parse_from_rfc3339(&s)
                                .unwrap()
                                .with_timezone(&Utc),
                        ),
                        None => None,
                    },
                    billed: row.get::<_, i64>(5)? != 0,
                })
            })?;

        let mut entries = Vec::new();
        for e in entries_iter {
            let e = e?;
            if let Some(b) = billed {
                if e.billed != b {
                    continue;
                }
            }
            entries.push(e);
        }
        Ok(entries)
    }

    /// Mark an entry as billed.
    pub fn mark_billed(&self, id: i64) -> Result<bool> {
        let rows_affected = self
            .conn
            .execute("UPDATE entries SET billed = 1 WHERE id = ?1", params![id])?;
        Ok(rows_affected > 0)
    }
    
    /// Mark an entry as unbilled.
    pub fn unmark_billed(&self, id: i64) -> Result<bool> {
        let rows_affected = self
            .conn
            .execute("UPDATE entries SET billed = 0 WHERE id = ?1", params![id])?;
        Ok(rows_affected > 0)
    }

}
