use crate::host::lock_error;
use std::error::Error;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewWindow};

/// The count the application icon's badge shows, which windows that open later show too.
#[derive(Default)]
pub struct Badge(Mutex<Option<i64>>);

impl Badge {
    /// Show a count on every window, none clearing the badge.
    pub fn set(&self, app: &AppHandle, count: Option<i64>) -> Result<(), Box<dyn Error>> {
        *self.0.lock().map_err(|_| lock_error())? = count;
        for window in app.webview_windows().values() {
            window.set_badge_count(count)?;
        }

        Ok(())
    }

    /// Show the current count on a window that opened.
    pub fn show(&self, window: &WebviewWindow) -> Result<(), Box<dyn Error>> {
        let count = *self.0.lock().map_err(|_| lock_error())?;
        window.set_badge_count(count)?;

        Ok(())
    }
}
