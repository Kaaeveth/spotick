use serde::{Deserialize, Serialize};

mod app_settings;

pub use crate::settings::app_settings::AppSettings;

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct SpotickSettings {
    pub auto_start: bool,
    pub source_app: String
}
