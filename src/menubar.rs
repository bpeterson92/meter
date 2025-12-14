#![cfg(target_os = "macos")]

use std::env;
use std::time::Duration;

use chrono::Utc;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
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

enum UserEvent {
    TrayIconEvent(tray_icon::TrayIconEvent),
    MenuEvent(tray_icon::menu::MenuEvent),
    Tick,
    HotKey(GlobalHotKeyEvent),
}

/// Create a simple timer icon (circle with play/pause indicator)
fn create_icon(is_running: bool) -> Icon {
    let size = 22u32; // Standard macOS menu bar icon size
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    // Draw a simple circle
    let center = size as f32 / 2.0;
    let radius = size as f32 / 2.0 - 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            let idx = ((y * size + x) * 4) as usize;

            if dist <= radius {
                if is_running {
                    // Green when running
                    rgba[idx] = 76; // R
                    rgba[idx + 1] = 217; // G
                    rgba[idx + 2] = 100; // B
                    rgba[idx + 3] = 255; // A
                } else {
                    // Gray when idle
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
    let home = env::var("HOME").expect("HOME not set");
    let db_path = format!("{}/.meter/db.sqlite", home);
    let db = Db::new(&db_path).expect("Failed to open DB");
    models::init_db(db.conn()).expect("Failed to init DB");
    models::init_projects_db(db.conn()).expect("Failed to init projects DB");

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // Set up global hotkey (Cmd+Shift+T)
    let hotkey_manager = GlobalHotKeyManager::new().expect("Failed to create hotkey manager");
    let hotkey = HotKey::new(Some(Modifiers::META | Modifiers::SHIFT), Code::KeyT);
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
    let _hotkey_manager = hotkey_manager; // Keep alive for the duration of the event loop

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(tao::event::StartCause::Init) => {
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

                tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_menu(Box::new(tray_menu.clone()))
                        .with_tooltip("Meter - Time Tracking")
                        .with_icon(create_icon(is_running))
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

                // Update icon if state changed
                if was_running != is_running {
                    if let Some(ref tray) = tray_icon {
                        let _ = tray.set_icon(Some(create_icon(is_running)));
                    }
                }

                // Update tooltip with elapsed time
                if let Some(entry) = &current_entry {
                    let elapsed = (Utc::now() - entry.start).num_seconds();
                    let tooltip =
                        format!("Meter - {} ({})", entry.project, format_duration(elapsed));
                    if let Some(tray) = &tray_icon {
                        let _ = tray.set_tooltip(Some(tooltip));
                    }
                } else if let Some(tray) = &tray_icon {
                    let _ = tray.set_tooltip(Some("Meter - No active timer"));
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
                            let _ = tray.set_icon(Some(create_icon(false)));
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
                            let _ = tray.set_icon(Some(create_icon(true)));
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
                                let _ = tray.set_icon(Some(create_icon(false)));
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
                                let _ = tray.set_icon(Some(create_icon(true)));
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
