use std::{
    mem::MaybeUninit,
    sync::Once,
    thread::{self},
};

use i_slint_backend_winit::winit::window::WindowAttributes;

/// Retrieves the global settings defining [WindowAttributes] applied when creating a new window.
/// ### NOTE
/// **This method nor the [WindowCreationSettings] are thread-safe.
/// Calling this method from any thread but the main thread will panic!**
pub fn get_window_creation_settings() -> &'static mut WindowCreationSettings {
    static mut WINDOW_SETTINGS: MaybeUninit<WindowCreationSettings> = MaybeUninit::uninit();
    static INIT: Once = Once::new();

    if thread::current().name().unwrap_or_default() != "main" {
        panic!("Called get_window_creation_settings not from the main thread");
    }

    #[allow(static_mut_refs)]
    unsafe {
        INIT.call_once(|| {
            WINDOW_SETTINGS.write(WindowCreationSettings::new());
        });
        WINDOW_SETTINGS.assume_init_mut()
    }
}

pub struct WindowCreationSettings {
    default_settings: WindowAttributes,
    current_settings: WindowAttributes,
}

impl WindowCreationSettings {
    fn new() -> Self {
        let attr = WindowAttributes::default()
            .with_visible(false)
            .with_transparent(true);
        Self {
            default_settings: attr.clone(),
            current_settings: attr,
        }
    }

    pub fn change(
        &mut self,
        change: impl FnOnce(WindowAttributes) -> WindowAttributes + 'static,
    ) -> SettingsChangedGuard {
        let new_attr = change(self.default_settings.clone());
        let guard = SettingsChangedGuard {
            old_settings: Some(self.current_settings.clone()),
        };
        self.current_settings = new_attr;
        guard
    }

    pub fn get_settings(&self) -> WindowAttributes {
        self.current_settings.clone()
    }
}

/// A guard to revert changes made with [WindowCreationSettings::change].
/// If this gets dropped, the current window creation settings
/// will be reverted to the previous ones.
pub struct SettingsChangedGuard {
    old_settings: Option<WindowAttributes>,
}

impl Drop for SettingsChangedGuard {
    fn drop(&mut self) {
        get_window_creation_settings().current_settings = self.old_settings.take().unwrap();
    }
}
