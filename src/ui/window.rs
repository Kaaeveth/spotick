pub mod main_window;
pub mod settings_window;
pub mod window_creation_settings;

slint::include_modules!();

use slint::{ComponentHandle, Weak};

pub use crate::ui::window::main_window::MainWindow;
pub use crate::ui::window::settings_window::SettingsWindow;
pub use crate::ui::window::window_creation_settings::get_window_creation_settings;

pub trait Window<T> 
where 
    T: ComponentHandle
{
    fn component(&self) -> &T;
    fn as_weak(&self) -> Weak<T> {
        self.component().as_weak()
    }
}
