#![cfg(target_os = "macos")]

use std::env;
use std::time::Duration;

use chrono::Utc;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
use notify_rust::Notification;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{
    Icon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
};

mod db;
mod models;

use db::Db;
use models::{Entry, PomodoroConfig};

/// Pomodoro state for menubar
#[derive(Debug, Clone, PartialEq)]
enum PomodoroState {
    Idle,
    Working,
    WorkComplete,
    OnBreak,
    BreakComplete,
}

impl Default for PomodoroState {
    fn default() -> Self {
        PomodoroState::Idle
    }
}

/// Hide the app from the Dock and app switcher (menu bar only)
fn set_activation_policy_accessory() {
    // Safety: This is called from the main thread at app startup
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
}

enum UserEvent {
    TrayIconEvent(tray_icon::TrayIconEvent),
    MenuEvent(tray_icon::menu::MenuEvent),
    Tick,
    HotKey(GlobalHotKeyEvent),
}

/// Create a timer icon with progress ring
/// - When idle: gray ring outline
/// - When running: white ring that fills based on elapsed time
/// - Pomodoro enabled: "P" in center instead of dot
/// - On break: blue ring color
fn create_icon(
    is_running: bool,
    elapsed_seconds: Option<i64>,
    pomodoro_enabled: bool,
    pomodoro_state: &PomodoroState,
    pomodoro_total_secs: Option<i64>,
) -> Icon {
    let size = 22u32; // Standard macOS menu bar icon size
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    let center = size as f32 / 2.0;
    let outer_radius = size as f32 / 2.0 - 1.0;
    let inner_radius = outer_radius - 3.5;

    // Determine ring color based on state
    let (ring_r, ring_g, ring_b) = match pomodoro_state {
        PomodoroState::OnBreak => (100, 149, 237), // Cornflower blue for break
        PomodoroState::WorkComplete | PomodoroState::BreakComplete => (255, 200, 0), // Yellow for prompts
        _ => (255, 255, 255), // White for working
    };

    // Calculate progress (0.0 to 1.0)
    let progress = if is_running || *pomodoro_state == PomodoroState::OnBreak {
        if let (Some(elapsed), Some(total)) = (elapsed_seconds, pomodoro_total_secs) {
            // Pomodoro mode: progress based on interval
            (elapsed as f32 / total as f32).min(1.0)
        } else if let Some(secs) = elapsed_seconds {
            // Normal mode: cycles every hour
            (secs % 3600) as f32 / 3600.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            let idx = ((y * size + x) * 4) as usize;

            // Check if pixel is in the ring area
            if dist <= outer_radius && dist >= inner_radius {
                // Calculate angle (0 at 12 o'clock, clockwise)
                let raw_angle = dx.atan2(-dy);
                let angle = if raw_angle < 0.0 {
                    (raw_angle + 2.0 * std::f32::consts::PI) / (2.0 * std::f32::consts::PI)
                } else {
                    raw_angle / (2.0 * std::f32::consts::PI)
                };

                if is_running || *pomodoro_state == PomodoroState::OnBreak {
                    if angle <= progress {
                        // Filled portion
                        rgba[idx] = ring_r;
                        rgba[idx + 1] = ring_g;
                        rgba[idx + 2] = ring_b;
                        rgba[idx + 3] = 255;
                    } else {
                        // Unfilled portion - dark gray
                        rgba[idx] = 60;
                        rgba[idx + 1] = 60;
                        rgba[idx + 2] = 60;
                        rgba[idx + 3] = 255;
                    }
                } else {
                    // Idle - gray ring
                    rgba[idx] = 128;
                    rgba[idx + 1] = 128;
                    rgba[idx + 2] = 128;
                    rgba[idx + 3] = 255;
                }
            }

            // Draw center: "P" if Pomodoro enabled, otherwise dot
            if pomodoro_enabled {
                // Draw a simple "P" shape in the center
                // P is roughly 5x7 pixels, centered
                let px = (x as i32) - (center as i32);
                let py = (y as i32) - (center as i32);

                // Define "P" shape (relative to center, scaled down)
                let is_p =
                    // Vertical stem
                    (px >= -2 && px <= -1 && py >= -3 && py <= 3) ||
                    // Top horizontal of P
                    (px >= -1 && px <= 2 && py >= -3 && py <= -2) ||
                    // Right curve of P (top)
                    (px >= 2 && px <= 3 && py >= -2 && py <= 0) ||
                    // Middle horizontal of P
                    (px >= -1 && px <= 2 && py >= 0 && py <= 1);

                if is_p {
                    if is_running || *pomodoro_state == PomodoroState::OnBreak {
                        rgba[idx] = ring_r;
                        rgba[idx + 1] = ring_g;
                        rgba[idx + 2] = ring_b;
                        rgba[idx + 3] = 255;
                    } else {
                        rgba[idx] = 128;
                        rgba[idx + 1] = 128;
                        rgba[idx + 2] = 128;
                        rgba[idx + 3] = 255;
                    }
                }
            } else {
                // Draw center dot
                if dist <= 3.0 {
                    if is_running {
                        rgba[idx] = 255;
                        rgba[idx + 1] = 255;
                        rgba[idx + 2] = 255;
                        rgba[idx + 3] = 255;
                    } else {
                        rgba[idx] = 128;
                        rgba[idx + 1] = 128;
                        rgba[idx + 2] = 128;
                        rgba[idx + 3] = 255;
                    }
                }
            }
        }
    }

    Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}

fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, secs)
}

fn format_remaining(seconds: i64) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", minutes, secs)
}

fn notify_work_complete() {
    let _ = Notification::new()
        .summary("Meter - Pomodoro")
        .body("Work period complete! Time for a break.")
        .sound_name("Glass")
        .show();
}

fn notify_break_complete() {
    let _ = Notification::new()
        .summary("Meter - Pomodoro")
        .body("Break complete! Ready to resume work?")
        .sound_name("Glass")
        .show();
}

fn main() {
    // Daemonize: fork and detach from terminal
    // Pass --no-fork to skip (useful for debugging)
    if !std::env::args().any(|arg| arg == "--no-fork") {
        unsafe {
            let pid = libc::fork();
            if pid < 0 {
                eprintln!("Failed to fork");
                std::process::exit(1);
            }
            if pid > 0 {
                // Parent process exits immediately
                println!("Meter menubar started (pid: {})", pid);
                std::process::exit(0);
            }
            // Child process continues
            // Create new session to detach from terminal
            libc::setsid();
        }
    }

    // Hide from Dock and app switcher - MUST be set before event loop is created
    set_activation_policy_accessory();

    let home = env::var("HOME").expect("HOME not set");
    let db_path = format!("{}/.meter/db.sqlite", home);
    let db = Db::new(&db_path).expect("Failed to open DB");
    models::init_db(db.conn()).expect("Failed to init DB");
    models::init_projects_db(db.conn()).expect("Failed to init projects DB");
    models::init_pomodoro_db(db.conn()).expect("Failed to init Pomodoro DB");

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // Re-apply activation policy after event loop creation (tao may reset it)
    set_activation_policy_accessory();

    // Set up global hotkey (Cmd+Control+T)
    let hotkey_manager = GlobalHotKeyManager::new().expect("Failed to create hotkey manager");
    let hotkey = HotKey::new(Some(Modifiers::META | Modifiers::CONTROL), Code::KeyT);
    let hotkey_id = hotkey.id();
    hotkey_manager
        .register(hotkey)
        .expect("Failed to register hotkey");

    // Set up hotkey event handler
    let proxy = event_loop.create_proxy();
    GlobalHotKeyEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::HotKey(event));
    }));

    // Set up event handlers
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::TrayIconEvent(event));
    }));

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
    }));

    // Set up a timer for periodic updates
    let proxy = event_loop.create_proxy();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let _ = proxy.send_event(UserEvent::Tick);
        }
    });

    // Menu items
    let start_i = MenuItem::with_id("start", "Start Timer...", true, None);
    let stop_i = MenuItem::with_id("stop", "Stop Timer", false, None);
    let status_i = MenuItem::with_id("status", "No active timer", false, None);
    let pomodoro_i = MenuItem::with_id("pomodoro", "Pomodoro: OFF", true, None);
    let separator = PredefinedMenuItem::separator();
    let quit_i = MenuItem::with_id("quit", "Quit Meter", true, None);

    // Recent projects submenu
    let projects_submenu = Submenu::new("Recent Projects", true);

    let tray_menu = Menu::new();
    tray_menu
        .append_items(&[
            &status_i,
            &separator,
            &start_i,
            &stop_i,
            &PredefinedMenuItem::separator(),
            &pomodoro_i,
            &PredefinedMenuItem::separator(),
            &projects_submenu,
            &PredefinedMenuItem::separator(),
            &quit_i,
        ])
        .unwrap();

    let mut tray_icon = None;
    let mut current_entry: Option<Entry> = None;
    let mut recent_projects: Vec<String> = Vec::new();
    let mut last_tooltip: Option<String> = None;
    let _hotkey_manager = hotkey_manager;

    // Pomodoro state
    let mut pomodoro_config: PomodoroConfig = db.get_pomodoro_config().unwrap_or_default();
    let mut pomodoro_state = PomodoroState::Idle;
    let mut pomodoro_interval_start: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut pomodoro_cycles_completed: u32 = 0;
    let mut pomodoro_last_project: Option<String> = None;
    let mut pomodoro_last_description: Option<String> = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(tao::event::StartCause::Init) => {
                set_activation_policy_accessory();

                // Check for active timer on startup
                current_entry = db.get_active_entry().unwrap_or(None);
                let is_running = current_entry.is_some();

                // Load Pomodoro config
                pomodoro_config = db.get_pomodoro_config().unwrap_or_default();

                // If timer is running and Pomodoro is enabled, set state to Working
                if is_running && pomodoro_config.enabled {
                    pomodoro_state = PomodoroState::Working;
                    pomodoro_interval_start = current_entry.as_ref().map(|e| e.start);
                }

                // Load recent projects
                if let Ok(entries) = db.list(None) {
                    let mut seen = std::collections::HashSet::new();
                    recent_projects = entries
                        .iter()
                        .filter(|e| seen.insert(e.project.clone()))
                        .take(5)
                        .map(|e| e.project.clone())
                        .collect();
                }

                // Update menu state
                update_menu_state(
                    &status_i,
                    &start_i,
                    &stop_i,
                    &current_entry,
                    &pomodoro_state,
                );
                update_pomodoro_menu(&pomodoro_i, &pomodoro_config);
                update_projects_submenu(&projects_submenu, &recent_projects);

                let elapsed = current_entry
                    .as_ref()
                    .map(|e| (Utc::now() - e.start).num_seconds());
                let total_secs = if pomodoro_config.enabled {
                    Some(pomodoro_config.work_duration as i64 * 60)
                } else {
                    None
                };

                tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_menu(Box::new(tray_menu.clone()))
                        .with_tooltip("Meter - Time Tracking")
                        .with_icon(create_icon(
                            is_running,
                            elapsed,
                            pomodoro_config.enabled,
                            &pomodoro_state,
                            total_secs,
                        ))
                        .build()
                        .unwrap(),
                );

                #[cfg(target_os = "macos")]
                {
                    use objc2_core_foundation::CFRunLoop;
                    CFRunLoop::main().unwrap().wake_up();
                }
            }

            Event::UserEvent(UserEvent::Tick) => {
                // Refresh Pomodoro config from DB
                pomodoro_config = db.get_pomodoro_config().unwrap_or_default();
                update_pomodoro_menu(&pomodoro_i, &pomodoro_config);

                // Refresh state from database
                let new_entry = db.get_active_entry().unwrap_or(None);
                let is_running = new_entry.is_some();

                // Detect external timer changes
                let timer_changed = match (&current_entry, &new_entry) {
                    (Some(old), Some(new)) => old.id != new.id,
                    (Some(_), None) => true,
                    (None, Some(_)) => true,
                    (None, None) => false,
                };

                if timer_changed {
                    if new_entry.is_some() && pomodoro_config.enabled {
                        // Timer started externally
                        pomodoro_state = PomodoroState::Working;
                        pomodoro_interval_start = Some(Utc::now());
                    } else if new_entry.is_none() && pomodoro_state == PomodoroState::Working {
                        // Timer stopped externally
                        pomodoro_state = PomodoroState::Idle;
                        pomodoro_interval_start = None;
                    }
                }

                current_entry = new_entry;

                // Pomodoro state machine
                if pomodoro_config.enabled {
                    if let Some(interval_start) = pomodoro_interval_start {
                        let elapsed_secs = (Utc::now() - interval_start).num_seconds();

                        match pomodoro_state {
                            PomodoroState::Working => {
                                let work_secs = pomodoro_config.work_duration as i64 * 60;
                                if elapsed_secs >= work_secs {
                                    // Work period complete
                                    if let Some(ref entry) = current_entry {
                                        pomodoro_last_project = Some(entry.project.clone());
                                        pomodoro_last_description = Some(entry.description.clone());
                                    }
                                    let _ = db.stop_active_timer();
                                    current_entry = None;
                                    pomodoro_state = PomodoroState::WorkComplete;
                                    pomodoro_interval_start = None;
                                    notify_work_complete();
                                }
                            }
                            PomodoroState::OnBreak => {
                                let is_long = (pomodoro_cycles_completed + 1)
                                    >= pomodoro_config.cycles_before_long as u32;
                                let break_secs = if is_long {
                                    pomodoro_config.long_break as i64 * 60
                                } else {
                                    pomodoro_config.short_break as i64 * 60
                                };

                                if elapsed_secs >= break_secs {
                                    // Break complete
                                    pomodoro_state = PomodoroState::BreakComplete;
                                    pomodoro_interval_start = None;
                                    notify_break_complete();
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Update menu state
                update_menu_state(
                    &status_i,
                    &start_i,
                    &stop_i,
                    &current_entry,
                    &pomodoro_state,
                );

                // Calculate values for icon
                let elapsed = match pomodoro_state {
                    PomodoroState::Working => {
                        pomodoro_interval_start.map(|s| (Utc::now() - s).num_seconds())
                    }
                    PomodoroState::OnBreak => {
                        pomodoro_interval_start.map(|s| (Utc::now() - s).num_seconds())
                    }
                    _ => current_entry
                        .as_ref()
                        .map(|e| (Utc::now() - e.start).num_seconds()),
                };

                let total_secs = match pomodoro_state {
                    PomodoroState::Working => Some(pomodoro_config.work_duration as i64 * 60),
                    PomodoroState::OnBreak => {
                        let is_long = (pomodoro_cycles_completed + 1)
                            >= pomodoro_config.cycles_before_long as u32;
                        if is_long {
                            Some(pomodoro_config.long_break as i64 * 60)
                        } else {
                            Some(pomodoro_config.short_break as i64 * 60)
                        }
                    }
                    _ => None,
                };

                // Update icon
                if let Some(ref tray) = tray_icon {
                    let _ = tray.set_icon(Some(create_icon(
                        is_running,
                        elapsed,
                        pomodoro_config.enabled,
                        &pomodoro_state,
                        total_secs,
                    )));
                }

                // Update tooltip
                let new_tooltip = match pomodoro_state {
                    PomodoroState::Working => {
                        if let Some(remaining) = total_secs.and_then(|t| elapsed.map(|e| t - e)) {
                            format!(
                                "Meter - Working ({} remaining)",
                                format_remaining(remaining.max(0))
                            )
                        } else {
                            "Meter - Working".to_string()
                        }
                    }
                    PomodoroState::OnBreak => {
                        if let Some(remaining) = total_secs.and_then(|t| elapsed.map(|e| t - e)) {
                            format!(
                                "Meter - Break ({} remaining)",
                                format_remaining(remaining.max(0))
                            )
                        } else {
                            "Meter - Break".to_string()
                        }
                    }
                    PomodoroState::WorkComplete => {
                        "Meter - Work complete! Start break?".to_string()
                    }
                    PomodoroState::BreakComplete => {
                        "Meter - Break complete! Resume work?".to_string()
                    }
                    PomodoroState::Idle => {
                        if let Some(entry) = &current_entry {
                            let elapsed = (Utc::now() - entry.start).num_seconds();
                            format!("Meter - {} ({})", entry.project, format_duration(elapsed))
                        } else {
                            "Meter - No active timer".to_string()
                        }
                    }
                };

                if last_tooltip.as_ref() != Some(&new_tooltip) {
                    if let Some(tray) = &tray_icon {
                        let _ = tray.set_tooltip(Some(&new_tooltip));
                    }
                    last_tooltip = Some(new_tooltip);
                }
            }

            Event::UserEvent(UserEvent::MenuEvent(event)) => {
                let id = event.id.0.as_str();

                if id == "quit" {
                    tray_icon.take();
                    *control_flow = ControlFlow::Exit;
                } else if id == "pomodoro" {
                    // Toggle Pomodoro mode
                    pomodoro_config.enabled = !pomodoro_config.enabled;
                    let _ = db.set_pomodoro_enabled(pomodoro_config.enabled);
                    update_pomodoro_menu(&pomodoro_i, &pomodoro_config);

                    if pomodoro_config.enabled {
                        if current_entry.is_some() {
                            pomodoro_state = PomodoroState::Working;
                            pomodoro_interval_start = Some(Utc::now());
                        }
                    } else {
                        pomodoro_state = PomodoroState::Idle;
                        pomodoro_interval_start = None;
                        pomodoro_cycles_completed = 0;
                    }
                } else if id == "stop" {
                    if let Ok(Some(_)) = db.stop_active_timer() {
                        current_entry = None;
                        pomodoro_state = PomodoroState::Idle;
                        pomodoro_interval_start = None;
                        pomodoro_cycles_completed = 0;
                        update_menu_state(
                            &status_i,
                            &start_i,
                            &stop_i,
                            &current_entry,
                            &pomodoro_state,
                        );
                        if let Some(ref tray) = tray_icon {
                            let _ = tray.set_icon(Some(create_icon(
                                false,
                                None,
                                pomodoro_config.enabled,
                                &pomodoro_state,
                                None,
                            )));
                            let _ = tray.set_tooltip(Some("Meter - Timer stopped"));
                        }
                    }
                } else if id == "start_break" {
                    // Start break (from WorkComplete state)
                    if pomodoro_state == PomodoroState::WorkComplete {
                        pomodoro_state = PomodoroState::OnBreak;
                        pomodoro_interval_start = Some(Utc::now());
                    }
                } else if id == "resume_work" {
                    // Resume work (from BreakComplete state)
                    if pomodoro_state == PomodoroState::BreakComplete {
                        pomodoro_cycles_completed += 1;
                        if pomodoro_cycles_completed >= pomodoro_config.cycles_before_long as u32 {
                            pomodoro_cycles_completed = 0;
                        }
                        pomodoro_state = PomodoroState::Idle;
                        pomodoro_interval_start = None;

                        // Start new timer with last project
                        let project = pomodoro_last_project
                            .clone()
                            .unwrap_or_else(|| "Work".to_string());
                        let description = pomodoro_last_description
                            .clone()
                            .unwrap_or_else(|| "Work session".to_string());
                        let entry = Entry {
                            id: 0,
                            project: project.clone(),
                            description,
                            start: Utc::now(),
                            end: None,
                            billed: false,
                        };
                        if db.insert(&entry).is_ok() {
                            current_entry = db.get_active_entry().unwrap_or(None);
                            pomodoro_state = PomodoroState::Working;
                            pomodoro_interval_start = Some(Utc::now());
                        }
                    }
                } else if id.starts_with("project:") {
                    let project = id.strip_prefix("project:").unwrap_or("Work");
                    let entry = Entry {
                        id: 0,
                        project: project.to_string(),
                        description: "Work session".to_string(),
                        start: Utc::now(),
                        end: None,
                        billed: false,
                    };
                    if db.insert(&entry).is_ok() {
                        current_entry = db.get_active_entry().unwrap_or(None);
                        pomodoro_last_project = Some(project.to_string());
                        pomodoro_last_description = Some("Work session".to_string());

                        if pomodoro_config.enabled {
                            pomodoro_state = PomodoroState::Working;
                            pomodoro_interval_start = Some(Utc::now());
                        }

                        update_menu_state(
                            &status_i,
                            &start_i,
                            &stop_i,
                            &current_entry,
                            &pomodoro_state,
                        );
                        if let Some(ref tray) = tray_icon {
                            let total = if pomodoro_config.enabled {
                                Some(pomodoro_config.work_duration as i64 * 60)
                            } else {
                                None
                            };
                            let _ = tray.set_icon(Some(create_icon(
                                true,
                                Some(0),
                                pomodoro_config.enabled,
                                &pomodoro_state,
                                total,
                            )));
                        }
                    }
                }
            }

            Event::UserEvent(UserEvent::TrayIconEvent(_event)) => {
                // Handle tray icon clicks if needed
            }

            Event::UserEvent(UserEvent::HotKey(event)) => {
                if event.id == hotkey_id {
                    // Handle based on Pomodoro state
                    match pomodoro_state {
                        PomodoroState::WorkComplete => {
                            // Start break
                            pomodoro_state = PomodoroState::OnBreak;
                            pomodoro_interval_start = Some(Utc::now());
                        }
                        PomodoroState::BreakComplete => {
                            // Resume work
                            pomodoro_cycles_completed += 1;
                            if pomodoro_cycles_completed
                                >= pomodoro_config.cycles_before_long as u32
                            {
                                pomodoro_cycles_completed = 0;
                            }

                            let project = pomodoro_last_project
                                .clone()
                                .unwrap_or_else(|| "Work".to_string());
                            let description = pomodoro_last_description
                                .clone()
                                .unwrap_or_else(|| "Work session".to_string());
                            let entry = Entry {
                                id: 0,
                                project,
                                description,
                                start: Utc::now(),
                                end: None,
                                billed: false,
                            };
                            if db.insert(&entry).is_ok() {
                                current_entry = db.get_active_entry().unwrap_or(None);
                                pomodoro_state = PomodoroState::Working;
                                pomodoro_interval_start = Some(Utc::now());
                            }
                        }
                        PomodoroState::OnBreak => {
                            // During break, hotkey does nothing
                        }
                        _ => {
                            // Normal toggle behavior
                            if current_entry.is_some() {
                                if let Ok(Some(_)) = db.stop_active_timer() {
                                    current_entry = None;
                                    pomodoro_state = PomodoroState::Idle;
                                    pomodoro_interval_start = None;
                                    pomodoro_cycles_completed = 0;
                                    update_menu_state(
                                        &status_i,
                                        &start_i,
                                        &stop_i,
                                        &current_entry,
                                        &pomodoro_state,
                                    );
                                    if let Some(ref tray) = tray_icon {
                                        let _ = tray.set_icon(Some(create_icon(
                                            false,
                                            None,
                                            pomodoro_config.enabled,
                                            &pomodoro_state,
                                            None,
                                        )));
                                        let _ = tray
                                            .set_tooltip(Some("Meter - Timer stopped via hotkey"));
                                    }
                                }
                            } else {
                                let project = recent_projects
                                    .first()
                                    .cloned()
                                    .unwrap_or_else(|| "Work".to_string());
                                let entry = Entry {
                                    id: 0,
                                    project: project.clone(),
                                    description: "Work session".to_string(),
                                    start: Utc::now(),
                                    end: None,
                                    billed: false,
                                };
                                if db.insert(&entry).is_ok() {
                                    current_entry = db.get_active_entry().unwrap_or(None);
                                    pomodoro_last_project = Some(project.clone());
                                    pomodoro_last_description = Some("Work session".to_string());

                                    if pomodoro_config.enabled {
                                        pomodoro_state = PomodoroState::Working;
                                        pomodoro_interval_start = Some(Utc::now());
                                    }

                                    update_menu_state(
                                        &status_i,
                                        &start_i,
                                        &stop_i,
                                        &current_entry,
                                        &pomodoro_state,
                                    );
                                    if let Some(ref tray) = tray_icon {
                                        let total = if pomodoro_config.enabled {
                                            Some(pomodoro_config.work_duration as i64 * 60)
                                        } else {
                                            None
                                        };
                                        let _ = tray.set_icon(Some(create_icon(
                                            true,
                                            Some(0),
                                            pomodoro_config.enabled,
                                            &pomodoro_state,
                                            total,
                                        )));
                                        let _ = tray.set_tooltip(Some(format!(
                                            "Meter - Started: {}",
                                            project
                                        )));
                                    }

                                    if let Ok(entries) = db.list(None) {
                                        let mut seen = std::collections::HashSet::new();
                                        recent_projects = entries
                                            .iter()
                                            .filter(|e| seen.insert(e.project.clone()))
                                            .take(5)
                                            .map(|e| e.project.clone())
                                            .collect();
                                        update_projects_submenu(
                                            &projects_submenu,
                                            &recent_projects,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            _ => {}
        }
    });
}

fn update_menu_state(
    status_i: &MenuItem,
    start_i: &MenuItem,
    stop_i: &MenuItem,
    current_entry: &Option<Entry>,
    pomodoro_state: &PomodoroState,
) {
    match pomodoro_state {
        PomodoroState::WorkComplete => {
            status_i.set_text("Work complete! Click to start break");
            start_i.set_enabled(false);
            stop_i.set_enabled(false);
        }
        PomodoroState::BreakComplete => {
            status_i.set_text("Break complete! Click to resume");
            start_i.set_enabled(false);
            stop_i.set_enabled(false);
        }
        PomodoroState::OnBreak => {
            status_i.set_text("On break...");
            start_i.set_enabled(false);
            stop_i.set_enabled(false);
        }
        _ => {
            if let Some(entry) = current_entry {
                let elapsed = (Utc::now() - entry.start).num_seconds();
                status_i.set_text(format!("{} - {}", entry.project, format_duration(elapsed)));
                start_i.set_enabled(false);
                stop_i.set_enabled(true);
            } else {
                status_i.set_text("No active timer");
                start_i.set_enabled(true);
                stop_i.set_enabled(false);
            }
        }
    }
}

fn update_pomodoro_menu(pomodoro_i: &MenuItem, config: &PomodoroConfig) {
    if config.enabled {
        pomodoro_i.set_text(format!("Pomodoro: ON ({}m)", config.work_duration));
    } else {
        pomodoro_i.set_text("Pomodoro: OFF");
    }
}

fn update_projects_submenu(submenu: &Submenu, projects: &[String]) {
    for project in projects {
        let item = MenuItem::with_id(
            format!("project:{}", project),
            format!("Start: {}", project),
            true,
            None,
        );
        let _ = submenu.append(&item);
    }
}
