# Meter

A simple time tracking CLI and TUI application for consultants to track billable hours and generate invoices.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.70+)

### Build from source

```bash
git clone https://github.com/bpeterson92/meter.git
cd meter
cargo build --release
```

The binary will be at `./target/release/meter`. Optionally, copy it to your PATH:

Linux
```bash
ln -s ./target/release/meter ~/.local/bin/
# or
sudo ln -s ./target/release/meter /usr/local/bin/
```

MacOS
```bash
sudo ln -s $PWD/target/release/meter /usr/local/bin/
```

## Quick Start

```bash
# Start a timer
meter start --project "Acme Corp" --desc "Working on feature X"

# Stop the timer
meter stop

# Launch the interactive TUI
meter tui
```

## Usage

### Interactive TUI

Launch the full-featured terminal interface:

```bash
meter tui
```

**Screens:**
- **Timer** (`1`) - Start/stop timers with live elapsed time display
- **Entries** (`2`) - View, delete, and bill time entries
- **Invoice** (`3`) - Generate invoices by month or custom selection

**Key Bindings:**
| Key | Action |
|-----|--------|
| `q` | Quit |
| `1/2/3` | Switch screens |
| `?` | Toggle help |
| `s` | Start/stop timer (Timer screen) |
| `j/k` | Navigate up/down |
| `d` | Delete entry (Entries screen) |
| `b` | Mark as billed (Entries screen) |
| `f` | Toggle filter (Entries screen) |
| `Enter` | Confirm/generate |
| `Esc` | Cancel/back |

### CLI Commands

```bash
# Start timing a project
meter start --project "Acme Corp" --desc "Initial kick-off"

# Stop the current timer
meter stop

# Add a manual 1.5-hour entry
meter add --project "Beta Inc" --desc "Fixed bug #42" --duration 1.5

# List all pending (unbilled) entries
meter list --billed false

# List all billed entries
meter list --billed true

# Mark a specific entry as billed
meter bill --id 3

# Mark all pending entries as billed
meter bill

# Generate a text invoice for the current month
meter invoice

# Generate an invoice for a specific month/year
meter invoice --month 11 --year 2025
```

## Data Storage

All data is stored in a SQLite database at `~/.meter.sqlite`.

Invoices are generated as text files in your home directory: `~/invoice_YYYY_MM.txt`

## License

MIT
