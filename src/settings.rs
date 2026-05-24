use std::sync::Arc;

use serde::{Deserialize, Serialize};
use slint::PhysicalPosition;
use tokio::sync::RwLock;

mod app_settings;

pub use crate::settings::app_settings::AppSettings;

pub type SpotickAppSettings = Arc<RwLock<AppSettings<SpotickSettings>>>;

/// Spotick specific settings.
/// NOTE: Make sure every change is made optional using [Option<T>]
/// for backwards compatibility - Or add some migration logic in [AppSettings].
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpotickSettings {
    pub auto_start: bool,
    pub always_on_top: bool,
    pub source_app: String,
    pub main_window_pos: PhysicalPosition,
    pub main_window_scale: f32,
    pub phantom_transparency: Option<f32>,
}

impl SpotickSettings {
    pub const DEFAULT_PHANTOM_TRANSPARENCY: f32 = 0.6;
}

impl Default for SpotickSettings {
    fn default() -> Self {
        SpotickSettings {
            auto_start: false,
            always_on_top: false,
            main_window_scale: 1.0,
            source_app: String::from("spotify.exe"),
            main_window_pos: PhysicalPosition::default(),
            phantom_transparency: Some(SpotickSettings::DEFAULT_PHANTOM_TRANSPARENCY),
        }
    }
}

#[macro_export]
macro_rules! on_settings_changed {
    ($settings:expr, |$spotick_settings:ident|$handler:block) => {
        let mut settings_rv = $settings.read().await.subscribe();
        let settings = Arc::downgrade(&$settings);

        tokio::spawn(async move {
            loop {
                if let Some(settings) = settings.upgrade() {
                    let sg = settings.read().await;
                    let $spotick_settings = sg.get_settings();
                    $handler
                } else {
                    break;
                }

                if let Err(_) = settings_rv.recv().await {
                    break;
                }
            }
        });
    };
}
