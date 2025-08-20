use std::{cell::{OnceCell, RefCell}, rc::Rc};

use i_slint_backend_winit::winit::window::WindowAttributes;

thread_local! {
    pub(crate) static WINDOW_SETTINGS: OnceCell<WindowCreationSettings> = OnceCell::new();
}

pub fn get_window_creation_settings() -> WindowCreationSettings {
    WINDOW_SETTINGS.with(|s| s.get_or_init(|| WindowCreationSettings::new()).clone())
}

pub struct WindowCreationSettings {
    inner: Rc<RefCell<InnerCreationSettings>>
}

struct InnerCreationSettings {
    default_settings: WindowAttributes,
    current_settings: WindowAttributes
}

impl WindowCreationSettings {
    pub fn new() -> Self {
        let attr = WindowAttributes::default()
            .with_visible(false)
            .with_transparent(true);
        Self {
            inner: Rc::new(RefCell::new(InnerCreationSettings {
                default_settings: attr.clone(),
                current_settings: attr
            }))
        }
    }

    pub fn change(&self, change: impl Fn(WindowAttributes) -> WindowAttributes + 'static) -> SettingsChangedGuard {
        let mut attr = self.inner.borrow_mut();
        let new_attr = change(attr.default_settings.clone());
        let guard = SettingsChangedGuard { 
            settings: self.clone(),
            old_settings: Some(attr.current_settings.clone())
        };
        (*attr).current_settings = new_attr;
        guard
    }

    pub fn get_settings(&self) -> WindowAttributes {
        self.inner.borrow().current_settings.clone()
    }
}

impl Clone for WindowCreationSettings {
    fn clone(&self) -> Self {
        WindowCreationSettings { inner: self.inner.clone() }
    }
}

/// A guard to revert changes made with [WindowCreationSettings::change].
/// If this gets dropped, the current window creation settings
/// will be reverted to the previous ones.
pub struct SettingsChangedGuard {
    settings: WindowCreationSettings,
    old_settings: Option<WindowAttributes>
}

impl Drop for SettingsChangedGuard {
    fn drop(&mut self) {
        // Borrowing may only fail if the guard is being dropped inside of a WindowCreationSettings::change
        // callback, which should not happen anyway.
        if let Ok(mut inner) = self.settings.inner.try_borrow_mut() {
            inner.current_settings = self.old_settings.take().unwrap();
        }
    }
}
