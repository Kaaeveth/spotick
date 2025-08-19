// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;

use crate::ui::{init_backend, window::{MainWindow, SettingsWindow}};

mod ui;

fn main() -> Result<()> {
    init_backend()?;

    let settings = SettingsWindow::new()?;
    let app = MainWindow::new(settings)?;
    app.run_blocking()?;
    Ok(())
}
