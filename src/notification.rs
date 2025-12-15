//! System notifications for Pomodoro timer events

#[cfg(target_os = "macos")]
use notify_rust::Notification;

/// Send a notification when work period is complete
pub fn notify_work_complete() {
    #[cfg(target_os = "macos")]
    {
        let _ = Notification::new()
            .summary("Meter - Pomodoro")
            .body("Work period complete! Time for a break.")
            .sound_name("Glass")
            .show();
    }
}

/// Send a notification when break is complete
pub fn notify_break_complete() {
    #[cfg(target_os = "macos")]
    {
        let _ = Notification::new()
            .summary("Meter - Pomodoro")
            .body("Break complete! Ready to resume work?")
            .sound_name("Glass")
            .show();
    }
}
