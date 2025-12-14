use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::models::{Entry, Project};

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

    /// Start a new timer for a project.
    /// Returns the created entry.
    pub fn start_timer(&self, project: &str, description: &str) -> Result<Entry> {
        let entry = Entry {
            id: 0,
            project: project.to_string(),
            description: description.to_string(),
            start: Utc::now(),
            end: None,
            billed: false,
        };
        self.insert(&entry)?;

        // Get the inserted entry with its ID
        self.get_active_entry()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
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

    /// Mark all pending entries as billed.
    pub fn mark_all_billed(&self) -> Result<usize> {
        let rows_affected = self
            .conn
            .execute("UPDATE entries SET billed = 1 WHERE billed = 0", params![])?;
        Ok(rows_affected)
    }

    /// Mark an entry as unbilled.
    pub fn unmark_billed(&self, id: i64) -> Result<bool> {
        let rows_affected = self
            .conn
            .execute("UPDATE entries SET billed = 0 WHERE id = ?1", params![id])?;
        Ok(rows_affected > 0)
    }

    /// Mark all billed entries as unbilled.
    pub fn unmark_all_billed(&self) -> Result<usize> {
        let rows_affected = self
            .conn
            .execute("UPDATE entries SET billed = 0 WHERE billed = 1", params![])?;
        Ok(rows_affected)
    }

    /// Update an entry's fields.
    pub fn update_entry(&self, entry: &Entry) -> Result<bool> {
        let rows_affected = self.conn.execute(
            "UPDATE entries SET project = ?1, description = ?2, start = ?3, end = ?4, billed = ?5 WHERE id = ?6",
            params![
                entry.project,
                entry.description,
                entry.start.to_rfc3339(),
                entry.end.map(|e| e.to_rfc3339()),
                if entry.billed { 1 } else { 0 },
                entry.id,
            ],
        )?;
        Ok(rows_affected > 0)
    }

    // === Project Methods ===

    /// Get or create a project by name.
    pub fn get_or_create_project(&self, name: &str) -> Result<Project> {
        if let Some(project) = self.get_project_by_name(name)? {
            return Ok(project);
        }

        self.conn.execute(
            "INSERT INTO projects (name, rate, currency) VALUES (?1, NULL, '$')",
            params![name],
        )?;

        self.get_project_by_name(name)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    /// Get project by name.
    pub fn get_project_by_name(&self, name: &str) -> Result<Option<Project>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, rate, currency FROM projects WHERE name = ?1")?;

        stmt.query_row(params![name], |row| {
            let rate_str: Option<String> = row.get(2)?;
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                rate: rate_str.and_then(|s| s.parse().ok()),
                currency: row.get(3)?,
            })
        })
        .optional()
    }

    /// List all projects.
    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, rate, currency FROM projects ORDER BY name")?;

        let projects = stmt.query_map([], |row| {
            let rate_str: Option<String> = row.get(2)?;
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                rate: rate_str.and_then(|s| s.parse().ok()),
                currency: row.get(3)?,
            })
        })?;

        projects.collect()
    }

    /// Update project rate.
    pub fn set_project_rate(
        &self,
        name: &str,
        rate: Option<f64>,
        currency: Option<&str>,
    ) -> Result<bool> {
        self.get_or_create_project(name)?;

        let rate_str = rate.map(|r| format!("{:.2}", r));
        let rows = self.conn.execute(
            "UPDATE projects SET rate = ?1, currency = COALESCE(?2, currency) WHERE name = ?3",
            params![rate_str, currency, name],
        )?;

        Ok(rows > 0)
    }

    /// Get distinct project names from entries (for migration/sync).
    pub fn get_distinct_entry_projects(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT project FROM entries ORDER BY project")?;

        let projects = stmt.query_map([], |row| row.get(0))?;
        projects.collect()
    }

    /// Sync projects table with existing entry projects.
    pub fn sync_projects_from_entries(&self) -> Result<()> {
        let entry_projects = self.get_distinct_entry_projects()?;
        for name in entry_projects {
            self.get_or_create_project(&name)?;
        }
        Ok(())
    }
}
