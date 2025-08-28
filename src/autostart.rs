use std::sync::Arc;

use anyhow::{Context, Result};
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS},
    RegKey,
};

use crate::{service::BaseService, settings::SpotickAppSettings};

const AUTO_START_KEY: &'static str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const AUTO_START_VALUE_NAME: &'static str = env!("CARGO_PKG_NAME");

pub fn enable_autostart() -> Result<()> {
    if is_autostart_enabled()? {
        return Ok(());
    }

    let app_path = std::env::current_exe()?; //.canonicalize()?;
    let auto_start_key = get_autostart_key()?;
    auto_start_key
        .set_value(AUTO_START_VALUE_NAME, &app_path.as_os_str())
        .context("Could not set autostart key")?;

    log::info!("Enabled autostart with path: {:?}", &app_path);
    Ok(())
}

fn get_autostart_key() -> Result<RegKey> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let auto_start_key = hkcu
        .open_subkey_with_flags(AUTO_START_KEY, KEY_ALL_ACCESS)
        .context("Could not open subkey")?;
    Ok(auto_start_key)
}

pub fn disable_autostart() -> Result<()> {
    if !is_autostart_enabled()? {
        return Ok(());
    }

    let auto_start_key = get_autostart_key()?;
    auto_start_key
        .delete_value(AUTO_START_VALUE_NAME)
        .context("Could not delete autostart key")?;
    log::info!("Disabled autostart");
    Ok(())
}

pub fn is_autostart_enabled() -> Result<bool> {
    let auto_start_key = get_autostart_key()?;
    let ok = auto_start_key.enum_values().any(|val| {
        if let Ok((val, _)) = val {
            val == AUTO_START_VALUE_NAME
        } else {
            false
        }
    });
    Ok(ok)
}

pub async fn register_autostart_changed(settings: SpotickAppSettings) {
    let mut settings_rv = settings.read().await.subscribe();
    let settings = Arc::downgrade(&settings);

    tokio::spawn(async move {
        loop {
            if let Some(settings) = settings.upgrade() {
                let auto_start_set = settings.read().await.get_settings().auto_start;
                let res = if auto_start_set {
                    enable_autostart()
                } else {
                    disable_autostart()
                };

                if let Err(e) = res {
                    log::error!("Could not toggle autostart: {}", e);
                }
            } else {
                break;
            }

            if let Err(_) = settings_rv.recv().await {
                break;
            }
        }
    });
}
