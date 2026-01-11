use std::{cell::RefCell, rc::Rc};

use anyhow::Result;
use i_slint_backend_winit::{
    winit::{
        platform::windows::{WindowAttributesExtWindows, WindowExtWindows, HWND},
        raw_window_handle::{HasWindowHandle, RawWindowHandle},
        window::WindowAttributes,
    },
    WinitWindowAccessor,
};
use slint::ComponentHandle;

use crate::ui::window::get_window_creation_settings;

pub struct DialogWindow<T, R, P>
where
    T: ComponentHandle + 'static,
    P: ComponentHandle + 'static,
{
    /// The actual slint window we want to show as a dialog
    window: T,
    parent_window: P,
    /// The result (e.g. user selection) of the dialog
    result: Rc<RefCell<Option<R>>>,
}

impl<T, R, P> DialogWindow<T, R, P>
where
    T: ComponentHandle + 'static,
    P: ComponentHandle + 'static,
    R: 'static,
{
    /// Creates a new dialog on top of [parent_window].
    /// The dialog itself is just a slint window returned by [create_win].
    /// NOTE: You have to setup the slint window to use [close_dialog!] for closing
    /// the dialog! Simply calling hide() won't work!
    /// You also need to wire any callbacks that save the dialog result in [create_win].
    /// The native close button, however, does work.
    pub fn new(
        parent_window: P,
        create_win: impl FnOnce(Rc<RefCell<Option<R>>>) -> Result<T>,
        win_attr: impl FnOnce(WindowAttributes) -> WindowAttributes + 'static,
    ) -> Result<Self> {
        // Create (but not show) the actual dialog window
        // This window is owned by the parent
        let parent_handle = parent_window
            .window()
            .window_handle()
            .window_handle()?
            .as_raw();
        let _sg = get_window_creation_settings().change(move |mut attr| {
            if let RawWindowHandle::Win32(hwnd) = parent_handle {
                attr = attr.with_owner_window(hwnd.hwnd.get() as HWND);
            }
            win_attr(attr)
        });
        let result = Rc::new(RefCell::new(None));
        let window = create_win(result.clone())?;

        Ok(DialogWindow {
            parent_window,
            window,
            result,
        })
    }

    /// Displays the dialog and returns the result via [on_close].
    /// This method returns immediately after opening the dialog!
    /// You must use [close_dialog!] inside the actual slint window
    /// to close dialog again - Simply calling hide() won't work!
    /// Make sure you have setup the slint window accordingly!
    pub fn show_dialog<F>(self, on_close: F) -> Result<()>
    where
        F: FnOnce(Option<R>) -> () + 'static,
    {
        // Disable parent window
        // This is needed in addition to setting owner window
        // to get the full dialog behavior
        self.parent_window
            .window()
            .with_winit_window(|win| win.set_enable(false));

        // Setup close handling
        // We just reenable the parent window here
        // NOTE: This works only when the dialog is properly closed
        // by using [close_dialog!] and not just calling hide()
        self.window.window().on_close_requested({
            let parent = self.parent_window.as_weak();
            let mut on_close = Some(on_close);
            let result = self.result.clone();
            move || {
                let _ = parent.upgrade_in_event_loop(|parent| {
                    parent.window().with_winit_window(|win| {
                        win.set_enable(true);
                        win.focus_window();
                    });
                });
                on_close.take().unwrap()(result.take());
                slint::CloseRequestResponse::HideWindow
            }
        });

        // Open the dialog
        self.window.show()?;
        Ok(())
    }
}

#[macro_export]
/// Must be used by the slint dialog window to properly close.
/// The dialog won't close by just calling hide() on the slint window.
macro_rules! close_dialog {
    ($window:ident|weak) => {
        let _ = $window.upgrade_in_event_loop(|win| {
            win.window()
                .dispatch_event(slint::platform::WindowEvent::CloseRequested);
        });
    };
    ($window:ident) => {
        $window
            .window()
            .dispatch_event(slint::platform::WindowEvent::CloseRequested);
    };
}
