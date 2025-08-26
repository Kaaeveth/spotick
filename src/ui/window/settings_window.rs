use anyhow::Result;
use slint::{ComponentHandle, ToSharedString};
use i_slint_backend_winit::winit::window::WindowButtons;
use crate::{callback, service::BaseService, settings::SpotickAppSettings, ui::{get_window_creation_settings, window::{SlintSettingsWindow, Window}}};

pub struct SettingsWindow {
    ui: SlintSettingsWindow,
    app_settings: SpotickAppSettings
}

impl SettingsWindow {
    pub fn new(app_settings: SpotickAppSettings) -> Result<Self> {
        let _settings_guard = get_window_creation_settings().change(|attr| {
            attr.with_enabled_buttons(WindowButtons::CLOSE)
        });
        let win = SettingsWindow { 
            ui: SlintSettingsWindow::new()?,
            app_settings
        };

        win.connect_settings();
        win.setup_callbacks();

        Ok(win)
    }

    fn connect_settings(&self) {
        let settings = self.app_settings.clone();
        let wui = self.ui.as_weak();
        tokio::spawn(async move {
            let wui = wui;
            let mut setting_evs = settings.read().await.subscribe();
            loop {
                let settings = settings.read().await.get_settings().clone();
                if let Err(_) = wui.upgrade_in_event_loop(move |ui| {
                    ui.set_auto_start(settings.auto_start);
                    ui.set_media_application_id(settings.source_app.to_shared_string());
                }) 
                {
                    break;
                }

                let Ok(_) = setting_evs.recv().await else {
                    break;
                };
            }
        });
    }

    #[allow(unused_variables)]
    fn setup_callbacks(&self) {
        let ui = &self.ui;
        
        let settings = self.app_settings.clone();
        callback!(on_auto_start_changed, |ui|{
            let auto_start = ui.get_auto_start();
            let settings = settings.clone();
            tokio::spawn(async move {
                let mut sw = settings.write().await;
                sw.get_settings_mut().auto_start = auto_start;
                if let Err(e) = sw.save().await {
                    log::error!("Failed to set auto_start: {}", e);
                }
            });
        });

        let settings = self.app_settings.clone();
        callback!(on_set_media_application, |ui, source_app|{
            let source_app = source_app.to_string();
            let settings = settings.clone();
            tokio::spawn(async move {
                let mut sw = settings.write().await;
                sw.get_settings_mut().source_app = source_app;
                if let Err(e) = sw.save().await {
                    log::error!("Failed to set media_application: {}", e);
                }
            });
        });
    }
}

impl Window<SlintSettingsWindow> for SettingsWindow {
    fn component(&self) -> &SlintSettingsWindow {
        &self.ui
    }
}
