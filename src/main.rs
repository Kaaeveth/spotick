// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

use anyhow::Result;

use crate::{service::{BaseService, MediaService, PlaybackChangedEvent, WindowsMediaService}, settings::{AppSettings, SpotickSettings}, ui::{init_backend, window::{MainWindow, SettingsWindow}}};

mod ui;
mod settings;
mod service;


#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    env_logger::init();
    init_backend()?;

    let settings = AppSettings::<SpotickSettings>::default()?;
    settings.write().await.load().await?;

    let win_media_service = WindowsMediaService::new("spotify.exe");
    win_media_service.write().await.begin_monitor_sessions()?;

    // Print media events when in debug mode
    #[cfg(debug_assertions)]
    {
        let mut ev = win_media_service.read().await.subscribe();
        let srv = win_media_service.clone();
        tokio::spawn(async move {
            loop {
                let Ok(e) = ev.recv().await else {
                    break;
                };

                match e {
                    PlaybackChangedEvent::Play | PlaybackChangedEvent::Pause => {
                        log::info!("{:?}", srv.read().await.current_playback_state());
                    },
                    PlaybackChangedEvent::TrackChanged => {
                        log::info!("{:?}", srv.read().await.current_track());
                    },
                    _ => {}
                }
            }
        });
    }

    let settings_window = SettingsWindow::new(settings.clone())?;
    let main_window = MainWindow::new(settings_window)?;

    main_window.run_blocking()?;
    Ok(())
}
