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

macOS
```bash
sudo ln -s $PWD/target/release/meter /usr/local/bin/
```

### macOS Menu Bar App

Build and install the menu bar companion app:

```bash
./scripts/bundle-menubar.sh
cp -r ./target/release/Meter.app /Applications/
```

The menu bar app:
- Shows a progress ring icon that fills as time passes (cycles every hour)
- Global hotkey `Cmd+Control+T` to toggle timer from anywhere
- Pomodoro mode support with visual indicators
- Runs in the background (no Dock icon, no Cmd+Tab)
- Start on login: System Settings > General > Login Items > add Meter

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

Launch the full‑featured terminal interface:

```bash
meter tui
```

**Screens:**
- **Timer** (`1`) – start/stop timers with live elapsed time display
- **Entries** (`2`) – view, delete, and bill time entries
- **Invoice** (`3`) – generate invoices by month or custom selection
- **Projects** (`4`) – manage project rates
- **Pomodoro** (`5`) – configure Pomodoro timer settings

**Key Bindings:**
| Key | Action |
|-----|--------|
| `q` | Quit |
| `1-5` | Switch screens |
| `?` | Toggle help |
| `s` | Start/stop timer (Timer screen) |
| `p` | Toggle Pomodoro mode (Timer screen) |
| `Space` | Acknowledge Pomodoro transition |
| `j/k` | Navigate up/down |
| `d` | Delete entry (Entries screen) |
| `b` | Mark as billed (Entries screen) |
| `f` | Toggle filter (Entries screen) |
| `Enter` | Confirm/generate |
| `Esc` | Cancel/back |

### CLI Commands

```bash
# Start timing a project
meter start --project "Acme Corp" --desc "Initial kick‑off"

# Stop the current timer
meter stop

# Add a manual 1.5‑hour entry
meter add --project "Beta Inc" --desc "Fixed bug #42" --duration 1.5

# List all pending (unbilled) entries
meter list --billed false

# List all billed entries
meter list --billed true

# Mark a specific entry as billed
meter bill --id 3

# Mark all pending entries as billed
meter bill

# Unbill a specific entry (or all if no id)
meter unbill --id 5
meter unbill   # unbills all pending entries

# Generate a text invoice for the current month
meter invoice

# Generate an invoice for a specific month/year
meter invoice --month 11 --year 2025

# Set or view a project's hourly rate
meter rate --project "Acme Corp" --rate 150.00   # set rate
meter rate --project "Acme Corp"                  # view rate

# List all projects with their current rates
meter projects

# View Pomodoro settings
meter pomodoro

# Enable Pomodoro mode
meter pomodoro --enable

# Disable Pomodoro mode
meter pomodoro --disable

# Configure Pomodoro durations (in minutes)
meter pomodoro --work 25 --short-break 5 --long-break 15 --cycles 4
```

#### Project Rate Management

`meter rate` lets you store a per‑project hourly rate and currency symbol.  
Examples:
- `meter rate --project "Acme Corp" --rate 150` → set rate to $150/hr.  
- `meter rate --project "Acme Corp"`      → show current rate for that project.

#### List Projects

`meter projects` prints a table of all known projects and the rate that has been assigned to each (or “—” if none).

#### Unbill Entries

`meter unbill` reverts the billed status of entries.  
- `meter unbill --id 10` unbills entry 10.  
- `meter unbill` unbills every entry that is currently marked as billed.

#### Pomodoro Timer

`meter pomodoro` configures the Pomodoro timer mode. When enabled, work periods automatically pause after the configured duration, prompting you to take a break.

**Settings:**
- `--work` – Work period duration in minutes (default: 45)
- `--short-break` – Short break duration in minutes (default: 15)
- `--long-break` – Long break duration in minutes (default: 60)
- `--cycles` – Number of work cycles before a long break (default: 4)

**Behavior:**
- Work period ends → timer pauses → notification
- Press Space (TUI) or hotkey (menubar) to start break
- Break ends → notification → press to resume work
- Break time is NOT included in billable hours

## Data Storage

All data is stored in a SQLite database located at `~/.meter/db.sqlite`.  
The database holds two tables: `entries` (time records) and `projects` (project names and rates).  
Invoices are written as plain-text files to `~/.meter/invoices/invoice_YYYY_MM.txt`.

## License

MIT