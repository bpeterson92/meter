# Meter

A simple time tracking CLI and TUI application for consultants to track billable hours and generate PDF invoices.

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

Launch the full-featured terminal interface:

```bash
meter tui
```

**Screens:**
- **Timer** (`1`) - start/stop timers with live elapsed time display
- **Entries** (`2`) - view, edit, delete, and bill time entries
- **Invoice** (`3`) - generate PDF invoices by month or custom selection
- **Projects** (`4`) - manage project hourly rates
- **Pomodoro** (`5`) - configure Pomodoro timer settings
- **Clients** (`6`) - view configured clients
- **Settings** (`7`) - view invoice/business settings

**Key Bindings:**
| Key | Action |
|-----|--------|
| `q` | Quit |
| `1-7` | Switch screens |
| `?` | Toggle help |
| `s` | Start/stop timer (Timer screen) |
| `p` | Toggle Pomodoro mode (Timer screen) |
| `Space` | Acknowledge Pomodoro transition |
| `j/k` | Navigate up/down |
| `e` | Edit entry (Entries screen) |
| `d` | Delete entry (Entries screen) |
| `b` | Mark as billed (Entries screen) |
| `u` | Unbill entry (Entries screen) |
| `f` | Toggle filter (Entries screen) |
| `c` | Cycle client selection (Invoice screen) |
| `Enter` | Confirm/generate |
| `Esc` | Cancel/back |

### CLI Commands

#### Time Tracking

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

# Unbill a specific entry (or all if no id)
meter unbill --id 5
meter unbill
```

#### Project Rate Management

```bash
# Set a project's hourly rate
meter rate --project "Acme Corp" --rate 150.00

# View a project's current rate
meter rate --project "Acme Corp"

# List all projects with their rates
meter projects
```

#### Invoice Generation

Meter generates professional PDF invoices with your business information, client details, line items, and payment instructions.

```bash
# Generate a PDF invoice for the current month
meter invoice

# Generate an invoice for a specific month/year
meter invoice --month 11 --year 2025

# Generate an invoice for a specific client
meter invoice --client 1

# Generate an invoice with a custom tax rate
meter invoice --tax-rate 8.5
```

**Invoice Features:**
- Professional PDF format with proper layout
- Your business name, address, and contact info
- Client billing information
- Auto-incrementing invoice numbers
- Itemized time entries with hourly rates
- Tax calculation
- Payment terms and due date
- Payment instructions

#### Invoice Settings (Your Business Info)

Configure your business information that appears on invoices:

```bash
# View current settings
meter invoice-settings

# Set business information
meter invoice-settings \
  --business-name "Your Consulting LLC" \
  --street "123 Main St" \
  --city "San Francisco" \
  --state "CA" \
  --postal "94102" \
  --country "USA" \
  --email "billing@yourconsulting.com" \
  --phone "(555) 123-4567" \
  --tax-id "12-3456789"

# Set payment details
meter invoice-settings \
  --payment-terms "Net 30" \
  --default-tax-rate 0 \
  --payment-instructions "Pay via ACH to Account #12345"
```

#### Client Management

Manage clients for invoicing:

```bash
# Add a new client
meter client add \
  --name "Acme Corporation" \
  --contact "John Smith" \
  --street "456 Corporate Blvd" \
  --city "New York" \
  --state "NY" \
  --postal "10001" \
  --country "USA" \
  --email "ap@acmecorp.com"

# List all clients
meter client list

# Edit a client
meter client edit --id 1 --email "newemail@acmecorp.com"

# Delete a client
meter client delete --id 1
```

#### Pomodoro Timer

Configure the Pomodoro timer mode for focused work sessions:

```bash
# View Pomodoro settings
meter pomodoro

# Enable Pomodoro mode
meter pomodoro --enable

# Disable Pomodoro mode
meter pomodoro --disable

# Configure durations (in minutes)
meter pomodoro --work 25 --short-break 5 --long-break 15 --cycles 4
```

**Settings:**
- `--work` - Work period duration in minutes (default: 45)
- `--short-break` - Short break duration in minutes (default: 15)
- `--long-break` - Long break duration in minutes (default: 60)
- `--cycles` - Number of work cycles before a long break (default: 4)

**Behavior:**
- Work period ends -> timer pauses -> notification
- Press Space (TUI) or hotkey (menubar) to start break
- Break ends -> notification -> press to resume work
- Break time is NOT included in billable hours

## Data Storage

All data is stored in a SQLite database located at `~/.meter/db.sqlite`.

**Database Tables:**
- `entries` - Time tracking records
- `projects` - Project names and hourly rates
- `pomodoro_config` - Pomodoro timer settings
- `invoice_settings` - Your business information
- `clients` - Client billing information
- `invoices` - Invoice history and numbering

**Output Files:**
- PDF invoices: `~/.meter/invoices/invoice_NNNN_YYYY_MM.pdf`

## License

MIT
