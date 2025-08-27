// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;

use crate::{
    service::WindowsMediaService,
    settings::{AppSettings, SpotickSettings},
    ui::{
        init_backend,
        window::{MainWindow, SettingsWindow},
    },
};

mod service;
mod settings;
mod ui;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    env_logger::init();
    init_backend()?;

    let settings = AppSettings::<SpotickSettings>::default()?;
    settings.write().await.load().await?;

    let win_media_service =
        WindowsMediaService::new(settings.read().await.get_settings().source_app.clone());
    win_media_service.write().await.begin_monitor_sessions()?;

    let settings_window = SettingsWindow::new(settings.clone())?;
    let main_window = MainWindow::new(win_media_service, settings_window).await?;

    main_window.run_blocking()?;
    Ok(())
}
