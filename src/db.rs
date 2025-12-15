use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::models::{Client, Entry, Invoice, InvoiceSettings, PomodoroConfig, Project};

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

    // === Pomodoro Methods ===

    /// Get the current Pomodoro configuration.
    pub fn get_pomodoro_config(&self) -> Result<PomodoroConfig> {
        let mut stmt = self.conn.prepare(
            "SELECT enabled, work_duration, short_break, long_break, cycles_before_long
             FROM pomodoro_config WHERE id = 1",
        )?;

        stmt.query_row([], |row| {
            Ok(PomodoroConfig {
                enabled: row.get::<_, i32>(0)? != 0,
                work_duration: row.get(1)?,
                short_break: row.get(2)?,
                long_break: row.get(3)?,
                cycles_before_long: row.get(4)?,
            })
        })
    }

    /// Update the Pomodoro configuration.
    pub fn set_pomodoro_config(&self, config: &PomodoroConfig) -> Result<()> {
        self.conn.execute(
            "UPDATE pomodoro_config SET
                enabled = ?1,
                work_duration = ?2,
                short_break = ?3,
                long_break = ?4,
                cycles_before_long = ?5
             WHERE id = 1",
            params![
                if config.enabled { 1 } else { 0 },
                config.work_duration,
                config.short_break,
                config.long_break,
                config.cycles_before_long,
            ],
        )?;
        Ok(())
    }

    /// Toggle Pomodoro enabled state.
    pub fn set_pomodoro_enabled(&self, enabled: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE pomodoro_config SET enabled = ?1 WHERE id = 1",
            params![if enabled { 1 } else { 0 }],
        )?;
        Ok(())
    }

    // === Invoice Settings Methods ===

    /// Get the current invoice settings.
    pub fn get_invoice_settings(&self) -> Result<InvoiceSettings> {
        let mut stmt = self.conn.prepare(
            "SELECT business_name, address_street, address_city, address_state,
                    address_postal, address_country, email, phone, tax_id,
                    payment_instructions, default_payment_terms, default_tax_rate
             FROM invoice_settings WHERE id = 1",
        )?;

        stmt.query_row([], |row| {
            Ok(InvoiceSettings {
                business_name: row.get(0)?,
                address_street: row.get(1)?,
                address_city: row.get(2)?,
                address_state: row.get(3)?,
                address_postal: row.get(4)?,
                address_country: row.get(5)?,
                email: row.get(6)?,
                phone: row.get(7)?,
                tax_id: row.get(8)?,
                payment_instructions: row.get(9)?,
                default_payment_terms: row.get(10)?,
                default_tax_rate: row.get(11)?,
            })
        })
    }

    /// Update invoice settings.
    pub fn set_invoice_settings(&self, settings: &InvoiceSettings) -> Result<()> {
        self.conn.execute(
            "UPDATE invoice_settings SET
                business_name = ?1,
                address_street = ?2,
                address_city = ?3,
                address_state = ?4,
                address_postal = ?5,
                address_country = ?6,
                email = ?7,
                phone = ?8,
                tax_id = ?9,
                payment_instructions = ?10,
                default_payment_terms = ?11,
                default_tax_rate = ?12
             WHERE id = 1",
            params![
                settings.business_name,
                settings.address_street,
                settings.address_city,
                settings.address_state,
                settings.address_postal,
                settings.address_country,
                settings.email,
                settings.phone,
                settings.tax_id,
                settings.payment_instructions,
                settings.default_payment_terms,
                settings.default_tax_rate,
            ],
        )?;
        Ok(())
    }

    // === Client Methods ===

    /// Add a new client.
    pub fn add_client(&self, client: &Client) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO clients (name, contact_person, address_street, address_city,
                                  address_state, address_postal, address_country, email)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                client.name,
                client.contact_person,
                client.address_street,
                client.address_city,
                client.address_state,
                client.address_postal,
                client.address_country,
                client.email,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get a client by ID.
    pub fn get_client(&self, id: i64) -> Result<Option<Client>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, contact_person, address_street, address_city,
                    address_state, address_postal, address_country, email
             FROM clients WHERE id = ?1",
        )?;

        stmt.query_row(params![id], |row| {
            Ok(Client {
                id: row.get(0)?,
                name: row.get(1)?,
                contact_person: row.get(2)?,
                address_street: row.get(3)?,
                address_city: row.get(4)?,
                address_state: row.get(5)?,
                address_postal: row.get(6)?,
                address_country: row.get(7)?,
                email: row.get(8)?,
            })
        })
        .optional()
    }

    /// List all clients.
    pub fn list_clients(&self) -> Result<Vec<Client>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, contact_person, address_street, address_city,
                    address_state, address_postal, address_country, email
             FROM clients ORDER BY name",
        )?;

        let clients = stmt.query_map([], |row| {
            Ok(Client {
                id: row.get(0)?,
                name: row.get(1)?,
                contact_person: row.get(2)?,
                address_street: row.get(3)?,
                address_city: row.get(4)?,
                address_state: row.get(5)?,
                address_postal: row.get(6)?,
                address_country: row.get(7)?,
                email: row.get(8)?,
            })
        })?;

        clients.collect()
    }

    /// Update a client.
    pub fn update_client(&self, client: &Client) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE clients SET
                name = ?1,
                contact_person = ?2,
                address_street = ?3,
                address_city = ?4,
                address_state = ?5,
                address_postal = ?6,
                address_country = ?7,
                email = ?8
             WHERE id = ?9",
            params![
                client.name,
                client.contact_person,
                client.address_street,
                client.address_city,
                client.address_state,
                client.address_postal,
                client.address_country,
                client.email,
                client.id,
            ],
        )?;
        Ok(rows > 0)
    }

    /// Delete a client.
    pub fn delete_client(&self, id: i64) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM clients WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    // === Invoice Record Methods ===

    /// Get the next invoice number.
    pub fn get_next_invoice_number(&self) -> Result<i64> {
        let mut stmt = self
            .conn
            .prepare("SELECT COALESCE(MAX(invoice_number), 0) + 1 FROM invoices")?;
        stmt.query_row([], |row| row.get(0))
    }

    /// Record a generated invoice.
    pub fn record_invoice(&self, invoice: &Invoice) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO invoices (invoice_number, client_id, date_issued, due_date,
                                   subtotal, tax_rate, tax_amount, total, file_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                invoice.invoice_number,
                invoice.client_id,
                invoice.date_issued,
                invoice.due_date,
                invoice.subtotal,
                invoice.tax_rate,
                invoice.tax_amount,
                invoice.total,
                invoice.file_path,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// List all recorded invoices.
    pub fn list_invoices(&self) -> Result<Vec<Invoice>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, invoice_number, client_id, date_issued, due_date,
                    subtotal, tax_rate, tax_amount, total, file_path
             FROM invoices ORDER BY invoice_number DESC",
        )?;

        let invoices = stmt.query_map([], |row| {
            Ok(Invoice {
                id: row.get(0)?,
                invoice_number: row.get(1)?,
                client_id: row.get(2)?,
                date_issued: row.get(3)?,
                due_date: row.get(4)?,
                subtotal: row.get(5)?,
                tax_rate: row.get(6)?,
                tax_amount: row.get(7)?,
                total: row.get(8)?,
                file_path: row.get(9)?,
            })
        })?;

        invoices.collect()
    }
}
