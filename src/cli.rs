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

    /// Generate a simple invoice text
    Invoice {
        #[arg(short, long)]
        month: Option<u32>,
        #[arg(short, long)]
        year: Option<i32>,
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
}
