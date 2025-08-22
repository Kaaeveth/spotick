use serde::{Deserialize, Serialize};

mod app_settings;

pub use crate::settings::app_settings::AppSettings;

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct SpotifySettings {
    pub auth_token: String,
    pub refresh_token: String
}

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct SpotickSettings {
    pub auto_start: bool,
    pub spotify: Option<SpotifySettings>
}
