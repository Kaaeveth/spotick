use crate::{
    callback, save_changes_in_settings,
    service::{BaseService, SharedMediaService},
    settings::SpotickAppSettings,
    ui::{
        get_window_creation_settings,
        window::{MsgType, SlintSettingsWindow, Window},
    },
};
use anyhow::Result;
use i_slint_backend_winit::winit::window::WindowButtons;
use slint::{ComponentHandle, SharedString, ToSharedString, Weak};
use std::{sync::Arc, time::Duration};
use tokio::sync::watch::{channel, Receiver, Sender};

pub struct SettingsWindow {
    ui: SlintSettingsWindow,
    app_settings: SpotickAppSettings,
    media_service: SharedMediaService,
    scale_changed_tx: Sender<f32>,
}

impl SettingsWindow {
    pub fn new(
        app_settings: SpotickAppSettings,
        media_service: SharedMediaService,
    ) -> Result<Self> {
        let _settings_guard = get_window_creation_settings()
            .change(|attr| attr.with_enabled_buttons(WindowButtons::CLOSE));
        let win = SettingsWindow {
            ui: SlintSettingsWindow::new()?,
            media_service,
            app_settings,
            scale_changed_tx: channel(1f32).0,
        };

        win.connect_settings();
        win.connect_window_scale();
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
                    ui.set_window_scale(settings.main_window_scale);
                }) {
                    break;
                }

                let Ok(_) = setting_evs.recv().await else {
                    break;
                };
            }
        });
    }

    pub fn subscribe_scale_changed(&self) -> Receiver<f32> {
        self.scale_changed_tx.subscribe()
    }

    fn connect_window_scale(&self) {
        let ui = &self.ui;
        let scale_sender = self.scale_changed_tx.clone();

        callback!(on_scale_changed, |ui| {
            let scale = ui.get_window_scale();
            let _ = scale_sender.send_replace(scale);
        });

        let mut scale_rv = self.subscribe_scale_changed();
        save_changes_in_settings!(scale_rv, self.app_settings, |sg| {
            sg.get_settings_mut().main_window_scale = scale_rv.borrow().clone();
        });
    }

    pub fn get_settings(&self) -> SpotickAppSettings {
        self.app_settings.clone()
    }

    fn setup_callbacks(&self) {
        let ui = &self.ui;

        let settings = self.app_settings.clone();
        let media_service = Arc::downgrade(&self.media_service);
        callback!(on_settings_changed, |ui| {
            let settings = settings.clone();
            let media_service = media_service.clone();

            let auto_start = ui.get_auto_start();
            let always_on_top = ui.get_always_top();
            let source_id = ui.get_media_application_id().to_string();
            let scale_factor = ui.get_window_scale();

            let ui = ui.as_weak();
            tokio::spawn(async move {
                let mut sg = settings.write().await;
                {
                    let settings = sg.get_settings_mut();
                    settings.auto_start = auto_start;
                    settings.always_on_top = always_on_top;
                    settings.source_app = source_id;
                    settings.main_window_scale = scale_factor;
                    log::info!("{:?}", settings);
                }

                // Save settings
                show_msg(&ui, "Saving...", MsgType::Info);
                if let Err(e) = sg.save().await {
                    let msg = format!("Failed to save settings: {}", e);
                    log::error!("{msg}");
                    show_msg(&ui, msg, MsgType::Error);
                } else {
                    show_msg(&ui, "Settings saved", MsgType::Success);
                }

                // Apply possible changes to the media service
                if let Some(media_service) = media_service.upgrade() {
                    let mut mg = media_service.write().await;
                    let new_source_app = &sg.get_settings().source_app;

                    if new_source_app != mg.get_source_app_id() {
                        if let Err(e) = mg.set_source_app_id(new_source_app.clone()) {
                            log::error!("Could not set source app: {}", e);
                        }
                    }
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

fn show_msg(ui: &Weak<SlintSettingsWindow>, msg: impl Into<SharedString>, success: MsgType) {
    let msg = msg.into();
    let _ = ui.upgrade_in_event_loop(move |ui| {
        ui.invoke_show_msg(msg, success);
    });
}
