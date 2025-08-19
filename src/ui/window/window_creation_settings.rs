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

pub struct InnerCreationSettings {
    pub(crate) default_settings: WindowAttributes,
    pub(crate) current_settings: WindowAttributes
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

    pub fn change(&self, change: impl Fn(WindowAttributes) -> WindowAttributes + 'static) {
        let mut attr = self.inner.borrow_mut();
        let new_attr = change(attr.default_settings.clone());
        (*attr).current_settings = new_attr;
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
