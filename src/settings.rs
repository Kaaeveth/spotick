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
}

impl Default for SpotickSettings {
    fn default() -> Self {
        SpotickSettings {
            auto_start: false,
            always_on_top: false,
            source_app: String::from("spotify.exe"),
            main_window_pos: PhysicalPosition::default(),
        }
    }
}
