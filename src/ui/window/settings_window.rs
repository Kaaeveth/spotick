use crate::{
    callback,
    service::BaseService,
    settings::SpotickAppSettings,
    ui::{
        get_window_creation_settings,
        window::{SlintSettingsWindow, Window},
    },
};
use anyhow::Result;
use i_slint_backend_winit::winit::window::WindowButtons;
use slint::{ComponentHandle, ToSharedString};

pub struct SettingsWindow {
    ui: SlintSettingsWindow,
    app_settings: SpotickAppSettings,
}

impl SettingsWindow {
    pub fn new(app_settings: SpotickAppSettings) -> Result<Self> {
        let _settings_guard = get_window_creation_settings()
            .change(|attr| attr.with_enabled_buttons(WindowButtons::CLOSE));
        let win = SettingsWindow {
            ui: SlintSettingsWindow::new()?,
            app_settings,
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
                let settings = settings.clone().read_owned().await;
                if let Err(_) = wui.upgrade_in_event_loop(move |ui| {
                    let settings = settings.get_settings();
                    ui.set_auto_start(settings.auto_start);
                    ui.set_always_top(settings.always_on_top);
                    ui.set_media_application_id(settings.source_app.to_shared_string());
                }) {
                    break;
                }

                let Ok(_) = setting_evs.recv().await else {
                    break;
                };
            }
        });
    }

    pub fn get_settings(&self) -> SpotickAppSettings {
        self.app_settings.clone()
    }

    fn setup_callbacks(&self) {
        let ui = &self.ui;

        let settings = self.app_settings.clone();
        callback!(on_settings_changed, |ui| {
            let settings = settings.clone();

            let auto_start = ui.get_auto_start();
            let always_on_top = ui.get_always_top();
            let source_id = ui.get_media_application_id().to_string();

            tokio::spawn(async move {
                let mut sg = settings.write().await;
                {
                    let settings = sg.get_settings_mut();
                    settings.auto_start = auto_start;
                    settings.always_on_top = always_on_top;
                    settings.source_app = source_id;
                    log::info!("{:?}", settings);
                }
                if let Err(e) = sg.save().await {
                    log::error!("Failed to save settings: {}", e);
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
