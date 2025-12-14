#![cfg(target_os = "macos")]

use std::env;
use std::time::Duration;

use chrono::Utc;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
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
use models::Entry;

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
/// - When running: white ring that fills based on elapsed time (cycles every hour)
fn create_icon(is_running: bool, elapsed_seconds: Option<i64>) -> Icon {
    let size = 22u32; // Standard macOS menu bar icon size
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    let center = size as f32 / 2.0;
    let outer_radius = size as f32 / 2.0 - 1.0;
    let inner_radius = outer_radius - 3.5;

    // Calculate progress (0.0 to 1.0) - cycles every hour
    let progress = if is_running {
        let secs = elapsed_seconds.unwrap_or(0);
        (secs % 3600) as f32 / 3600.0
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
                // atan2(dx, -dy) gives 0 at top, positive clockwise
                let raw_angle = dx.atan2(-dy); // -PI to PI, 0 at top
                let angle = if raw_angle < 0.0 {
                    (raw_angle + 2.0 * std::f32::consts::PI) / (2.0 * std::f32::consts::PI)
                } else {
                    raw_angle / (2.0 * std::f32::consts::PI)
                };

                if is_running {
                    if angle <= progress {
                        // Filled portion - white
                        rgba[idx] = 255; // R
                        rgba[idx + 1] = 255; // G
                        rgba[idx + 2] = 255; // B
                        rgba[idx + 3] = 255; // A
                    } else {
                        // Unfilled portion - dark gray
                        rgba[idx] = 60; // R
                        rgba[idx + 1] = 60; // G
                        rgba[idx + 2] = 60; // B
                        rgba[idx + 3] = 255; // A
                    }
                } else {
                    // Idle - gray ring
                    rgba[idx] = 128; // R
                    rgba[idx + 1] = 128; // G
                    rgba[idx + 2] = 128; // B
                    rgba[idx + 3] = 255; // A
                }
            }

            // Draw center dot
            if dist <= 3.0 {
                if is_running {
                    // White center dot
                    rgba[idx] = 255; // R
                    rgba[idx + 1] = 255; // G
                    rgba[idx + 2] = 255; // B
                    rgba[idx + 3] = 255; // A
                } else {
                    // Gray center dot
                    rgba[idx] = 128; // R
                    rgba[idx + 1] = 128; // G
                    rgba[idx + 2] = 128; // B
                    rgba[idx + 3] = 255; // A
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

    // Menu items - we'll recreate the menu on each update
    let start_i = MenuItem::with_id("start", "Start Timer...", true, None);
    let stop_i = MenuItem::with_id("stop", "Stop Timer", false, None);
    let status_i = MenuItem::with_id("status", "No active timer", false, None);
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
            &projects_submenu,
            &PredefinedMenuItem::separator(),
            &quit_i,
        ])
        .unwrap();

    let mut tray_icon = None;
    let mut current_entry: Option<Entry> = None;
    let mut recent_projects: Vec<String> = Vec::new();
    let mut last_tooltip: Option<String> = None;
    let _hotkey_manager = hotkey_manager; // Keep alive for the duration of the event loop

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(tao::event::StartCause::Init) => {
                // Hide from Dock/Cmd+Tab - must be set here AFTER tao initializes
                // (tao/winit sets activation policy to Regular on startup, overriding LSUIElement)
                set_activation_policy_accessory();

                // Check for active timer on startup
                current_entry = db.get_active_entry().unwrap_or(None);
                let is_running = current_entry.is_some();

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
                update_menu_state(&status_i, &start_i, &stop_i, &current_entry);
                update_projects_submenu(&projects_submenu, &recent_projects);

                let elapsed = current_entry
                    .as_ref()
                    .map(|e| (Utc::now() - e.start).num_seconds());
                tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_menu(Box::new(tray_menu.clone()))
                        .with_tooltip("Meter - Time Tracking")
                        .with_icon(create_icon(is_running, elapsed))
                        .build()
                        .unwrap(),
                );

                // Wake up the run loop on macOS
                #[cfg(target_os = "macos")]
                {
                    use objc2_core_foundation::CFRunLoop;
                    CFRunLoop::main().unwrap().wake_up();
                }
            }

            Event::UserEvent(UserEvent::Tick) => {
                // Refresh state from database
                let new_entry = db.get_active_entry().unwrap_or(None);
                let was_running = current_entry.is_some();
                let is_running = new_entry.is_some();

                current_entry = new_entry;

                // Update menu state
                update_menu_state(&status_i, &start_i, &stop_i, &current_entry);

                // Update icon (always update when running to show progress)
                if let Some(ref tray) = tray_icon {
                    let elapsed = current_entry
                        .as_ref()
                        .map(|e| (Utc::now() - e.start).num_seconds());
                    let _ = tray.set_icon(Some(create_icon(is_running, elapsed)));
                }

                // Update tooltip with elapsed time (only when text changes to avoid flickering)
                let new_tooltip = if let Some(entry) = &current_entry {
                    let elapsed = (Utc::now() - entry.start).num_seconds();
                    format!("Meter - {} ({})", entry.project, format_duration(elapsed))
                } else {
                    "Meter - No active timer".to_string()
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
                } else if id == "stop" {
                    if let Ok(Some(_)) = db.stop_active_timer() {
                        current_entry = None;
                        update_menu_state(&status_i, &start_i, &stop_i, &current_entry);
                        if let Some(ref tray) = tray_icon {
                            let _ = tray.set_icon(Some(create_icon(false, None)));
                            let _ = tray.set_tooltip(Some("Meter - Timer stopped"));
                        }
                    }
                } else if id.starts_with("project:") {
                    // Start timer with selected project
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
                        update_menu_state(&status_i, &start_i, &stop_i, &current_entry);
                        if let Some(ref tray) = tray_icon {
                            let _ = tray.set_icon(Some(create_icon(true, Some(0))));
                        }
                    }
                }
            }

            Event::UserEvent(UserEvent::TrayIconEvent(_event)) => {
                // Handle tray icon clicks if needed
            }

            Event::UserEvent(UserEvent::HotKey(event)) => {
                if event.id == hotkey_id {
                    // Toggle timer: if running, stop; if stopped, start with most recent project
                    if current_entry.is_some() {
                        // Stop the timer
                        if let Ok(Some(_)) = db.stop_active_timer() {
                            current_entry = None;
                            update_menu_state(&status_i, &start_i, &stop_i, &current_entry);
                            if let Some(ref tray) = tray_icon {
                                let _ = tray.set_icon(Some(create_icon(false, None)));
                                let _ = tray.set_tooltip(Some("Meter - Timer stopped via hotkey"));
                            }
                        }
                    } else {
                        // Start timer with most recent project
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
                            update_menu_state(&status_i, &start_i, &stop_i, &current_entry);
                            if let Some(ref tray) = tray_icon {
                                let _ = tray.set_icon(Some(create_icon(true, Some(0))));
                                let _ =
                                    tray.set_tooltip(Some(format!("Meter - Started: {}", project)));
                            }

                            // Refresh recent projects list
                            if let Ok(entries) = db.list(None) {
                                let mut seen = std::collections::HashSet::new();
                                recent_projects = entries
                                    .iter()
                                    .filter(|e| seen.insert(e.project.clone()))
                                    .take(5)
                                    .map(|e| e.project.clone())
                                    .collect();
                                update_projects_submenu(&projects_submenu, &recent_projects);
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
) {
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

fn update_projects_submenu(submenu: &Submenu, projects: &[String]) {
    // Clear existing items - we need to recreate the submenu content
    // Note: tray-icon doesn't have a clear() method, so items persist
    // For now, we only populate on startup
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
