pub mod dialog_window;
pub mod main_window;
pub mod settings_window;
pub mod window_creation_settings;

slint::include_modules!();

use slint::{ComponentHandle, Weak};

pub use crate::ui::window::dialog_window::DialogWindow;
pub use crate::ui::window::main_window::MainWindow;
pub use crate::ui::window::settings_window::SettingsWindow;
pub use crate::ui::window::window_creation_settings::get_window_creation_settings;

pub trait Window<T>
where
    T: ComponentHandle,
{
    fn component(&self) -> &T;
    fn as_weak(&self) -> Weak<T> {
        self.component().as_weak()
    }
}

#[macro_export]
macro_rules! save_changes_in_settings {
    ($watch_rv:ident, $settings:expr, |$sg:ident|$handler:block) => {
        let settings = Arc::downgrade(&$settings);
        tokio::spawn(async move {
            loop {
                if let Err(_) = $watch_rv.changed().await {
                    break;
                }
                // Wait a bit for any other potential changes before saving
                tokio::time::sleep(Duration::from_millis(500)).await;

                if let Some(settings) = settings.upgrade() {
                    let mut $sg = settings.write().await;
                    $handler;
                    $watch_rv.mark_unchanged();
                    match $sg.save().await {
                        Ok(()) => {
                            log::info!("Saved observed settings from {}", stringify!($watch_rv))
                        }
                        Err(e) => log::error!(
                            "Could not save observed settings from {}: {:?}",
                            stringify!($watch_rv),
                            e
                        ),
                    };
                } else {
                    break;
                }
            }
        });
    };
}
