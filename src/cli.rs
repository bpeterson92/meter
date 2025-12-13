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

    /// Generate a simple invoice text
    Invoice {
        #[arg(short, long)]
        month: Option<u32>,
        #[arg(short, long)]
        year: Option<i32>,
    },

    /// Launch the interactive TUI
    Tui,
}
