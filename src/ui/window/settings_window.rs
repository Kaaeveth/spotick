use anyhow::Result;
use i_slint_backend_winit::winit::window::WindowButtons;
use crate::ui::{get_window_creation_settings, window::{SlintSettingsWindow, Window}};

pub struct SettingsWindow {
    ui: SlintSettingsWindow
}

impl SettingsWindow {
    pub fn new() -> Result<Self> {
        get_window_creation_settings().change(|attr| {
            attr.with_enabled_buttons(WindowButtons::CLOSE)
        });
        Ok(SettingsWindow { 
            ui: SlintSettingsWindow::new()?
        })
    }
}

impl Window<SlintSettingsWindow> for SettingsWindow {
    fn component(&self) -> &SlintSettingsWindow {
        &self.ui
    }
}
