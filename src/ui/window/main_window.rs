use i_slint_backend_winit::winit::platform::windows::WindowAttributesExtWindows;
use image::RgbaImage;
use slint::{ComponentHandle, Image, PhysicalPosition, Rgba8Pixel, SharedPixelBuffer};
use anyhow::Result;

use crate::{callback, ui::{apply_border_radius, get_window_creation_settings, window::{SettingsWindow, SlintMainWindow, Window}}};

pub struct MainWindow {
    ui: SlintMainWindow,
    settings_window: SettingsWindow
}

impl MainWindow {
    pub fn new(settings: SettingsWindow) -> Result<Self> {
        let _guard_settings = get_window_creation_settings().change(|attr| {
            attr.with_skip_taskbar(true)
        });
        let app = MainWindow {
            ui: SlintMainWindow::new()?,
            settings_window: settings
        };

        app.enable_app_quit();
        app.enable_window_positioning();
        app.set_initial_thumbnail();
        app.setup_ui_callbacks();
        
        Ok(app)
    }

    /// Start the main window event loop and
    /// shows the window. Blocks until the window closes.
    pub fn run_blocking(&self) -> Result<()> {
        self.ui.show()?;
        tokio::task::block_in_place(slint::run_event_loop)?;
        self.ui.hide()?;
        Ok(())
    }

    fn setup_ui_callbacks(&self) {
        let _app = &self.ui;
        let settings_window = self.settings_window.as_weak();

        callback!(on_show_options, |_app| {
            let settings_window = settings_window.unwrap();
            let _ = settings_window.show();
        });
    }

    fn enable_window_positioning(&self) {
        let app = &self.ui;
        let window_pos = app.window().position();
        app.set_window_x(window_pos.x as f32);
        app.set_window_y(window_pos.y as f32);

        callback!(on_position_window, |app, x, y| {
            let pos = PhysicalPosition::new(x as i32, y as i32);
            app.window().set_position(pos);
        });
    }

    fn enable_app_quit(&self) {
        let _app = &self.ui;
        callback!(on_quit, |_app| {
            let _ = slint::quit_event_loop();
        });
    }

    /// Sets the initial (empty) album cover image
    /// using the placeholder (thumbnail-placeholder)
    /// defined in the Slint file of the [AppWindow].
    /// This is necessary for image decorations (border-radius,...)
    /// to be applied to the initial cover image.
    fn set_initial_thumbnail(&self) {
        let app = &self.ui;
        let img = app.get_thumbnail_placeholder();
        let img_size = img.size();
        let img = img.to_rgba8().expect("Expected RGBA");
        let buffer = RgbaImage::from_raw(
            img_size.width,
            img_size.height,
            img.as_bytes().to_vec()
        ).expect("Invalid placeholder image format");

        self.set_thumbnail(buffer);
    }

    fn set_thumbnail(&self, mut img: RgbaImage) {
        // Apply image decorations
        apply_border_radius(&mut img, self.ui.get_thumbnail_border_radius() as u32);

        let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            img.as_raw(),
            img.width(),
            img.height(),
        );
        let image = Image::from_rgba8(buffer);
        self.ui.set_thumbnail_img(image);
    }
}

impl Window<SlintMainWindow> for MainWindow {
    fn component(&self) -> &SlintMainWindow {
        &self.ui
    }
}
