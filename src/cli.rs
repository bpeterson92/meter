use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "meter")]
#[command(about = "Track consulting hours and generate invoices", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a new timer for a project
    Start {
        #[arg(short, long)]
        project: String,
        #[arg(short, long, default_value = "Work session")]
        desc: String,
    },

    /// Stop the currently running timer
    Stop,

    /// Add a manual time entry
    Add {
        #[arg(short, long)]
        project: String,
        #[arg(short, long)]
        desc: String,
        #[arg(short, long, help = "Duration in hours (e.g. 1.5)")]
        duration: f64,
    },

    /// List all entries (filtered by status)
    List {
        #[arg(short, long)]
        billed: bool,
    },

    /// Mark entries as billed
    Bill {
        #[arg(short, long)]
        id: Option<i64>,
    },

    /// Mark entries as unbilled
    Unbill {
        #[arg(short, long)]
        id: Option<i64>,
    },

    /// Generate a PDF invoice
    Invoice {
        /// Month (1-12). Defaults to current month.
        #[arg(short, long)]
        month: Option<u32>,

        /// Year. Defaults to current year.
        #[arg(short, long)]
        year: Option<i32>,

        /// Client ID to invoice
        #[arg(short, long)]
        client: Option<i64>,

        /// Override tax rate for this invoice
        #[arg(short, long)]
        tax_rate: Option<f64>,
    },

    /// Launch the interactive TUI
    Tui,

    /// Set or view project hourly rate
    Rate {
        /// Project name
        #[arg(short, long)]
        project: String,

        /// Hourly rate (e.g., 150.00). Omit to view current rate.
        #[arg(short, long)]
        rate: Option<f64>,

        /// Currency symbol (default: $)
        #[arg(short, long, default_value = "$")]
        currency: String,
    },

    /// List all projects with their rates
    Projects,

    /// Configure Pomodoro timer settings
    Pomodoro {
        /// Enable Pomodoro mode
        #[arg(short, long)]
        enable: bool,

        /// Disable Pomodoro mode
        #[arg(short, long, conflicts_with = "enable")]
        disable: bool,

        /// Work period duration in minutes (default: 45)
        #[arg(short, long)]
        work: Option<i32>,

        /// Short break duration in minutes (default: 15)
        #[arg(short, long)]
        short_break: Option<i32>,

        /// Long break duration in minutes (default: 60)
        #[arg(short, long)]
        long_break: Option<i32>,

        /// Number of work cycles before a long break (default: 4)
        #[arg(short, long)]
        cycles: Option<i32>,
    },

    /// Configure invoice settings (your business info)
    #[command(name = "invoice-settings")]
    InvoiceSettings {
        /// Business or personal name
        #[arg(long)]
        business_name: Option<String>,

        /// Street address
        #[arg(long)]
        street: Option<String>,

        /// City
        #[arg(long)]
        city: Option<String>,

        /// State or province
        #[arg(long)]
        state: Option<String>,

        /// Postal/ZIP code
        #[arg(long)]
        postal: Option<String>,

        /// Country
        #[arg(long)]
        country: Option<String>,

        /// Email address
        #[arg(long)]
        email: Option<String>,

        /// Phone number
        #[arg(long)]
        phone: Option<String>,

        /// Tax ID / VAT number
        #[arg(long)]
        tax_id: Option<String>,

        /// Payment instructions (bank details, PayPal, etc.)
        #[arg(long)]
        payment_instructions: Option<String>,

        /// Default payment terms (e.g., "Net 30", "Due on receipt")
        #[arg(long)]
        payment_terms: Option<String>,

        /// Default tax rate percentage (e.g., 8.5 for 8.5%)
        #[arg(long)]
        tax_rate: Option<f64>,
    },

    /// Manage clients
    #[command(subcommand)]
    Client(ClientCommands),
}

#[derive(Subcommand)]
pub enum ClientCommands {
    /// Add a new client
    Add {
        /// Client or company name
        #[arg(long)]
        name: String,

        /// Contact person name
        #[arg(long)]
        contact: Option<String>,

        /// Street address
        #[arg(long)]
        street: Option<String>,

        /// City
        #[arg(long)]
        city: Option<String>,

        /// State or province
        #[arg(long)]
        state: Option<String>,

        /// Postal/ZIP code
        #[arg(long)]
        postal: Option<String>,

        /// Country
        #[arg(long)]
        country: Option<String>,

        /// Email address
        #[arg(long)]
        email: Option<String>,
    },

    /// List all clients
    List,

    /// Edit a client
    Edit {
        /// Client ID
        id: i64,

        /// Client or company name
        #[arg(long)]
        name: Option<String>,

        /// Contact person name
        #[arg(long)]
        contact: Option<String>,

        /// Street address
        #[arg(long)]
        street: Option<String>,

        /// City
        #[arg(long)]
        city: Option<String>,

        /// State or province
        #[arg(long)]
        state: Option<String>,

        /// Postal/ZIP code
        #[arg(long)]
        postal: Option<String>,

        /// Country
        #[arg(long)]
        country: Option<String>,

        /// Email address
        #[arg(long)]
        email: Option<String>,
    },

    /// Delete a client
    Delete {
        /// Client ID
        id: i64,
    },
}
