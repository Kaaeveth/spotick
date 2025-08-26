use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

mod app_settings;

pub use crate::settings::app_settings::AppSettings;

pub type SpotickAppSettings = Arc<RwLock<AppSettings<SpotickSettings>>>;

#[derive(Serialize, Deserialize, Clone)]
pub struct SpotickSettings {
    pub auto_start: bool,
    pub source_app: String
}

impl Default for SpotickSettings {
    fn default() -> Self {
        SpotickSettings { 
            auto_start: false,
            source_app: String::from("spotify.exe")
        }
    }
}
