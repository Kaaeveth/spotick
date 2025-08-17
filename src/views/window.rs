use image::RgbaImage;
use slint::{Image, PhysicalPosition, Rgba8Pixel, SharedPixelBuffer};
use anyhow::Result;

use crate::{callback, views::{apply_border_radius, UiPlaybackInformation}};

slint::include_modules!();

pub struct MainWindow {
    ui: AppWindow
}

impl MainWindow {
    pub fn new() -> Result<Self> {
        let app = MainWindow { 
            ui: AppWindow::new()? 
        };
        app.enable_window_closing();
        app.enable_window_positioning();
        app.set_initial_thumbnail();
        Ok(app)
    }

    /// Start the main window event loop and
    /// shows the window. Blocks until the window closes.
    pub fn run_blocking(&self) -> Result<()> {
        self.ui.run()?;
        Ok(())
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

    fn enable_window_closing(&self) {
        let app = &self.ui;
        callback!(on_close, |app| {
            app.window().dispatch_event(slint::platform::WindowEvent::CloseRequested);
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
}

impl UiPlaybackInformation for MainWindow {
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
    
    fn set_title(&self, title: impl AsRef<str>) {
        self.ui.set_track_title(title.as_ref().into());
    }
    
    fn set_subtitle(&self, subtitle: impl AsRef<str>) {
        self.ui.set_track_subtitle(subtitle.as_ref().into());
    }
    
    fn set_playing(&self, playing: bool) {
        self.ui.set_playing(playing);
    }

    
}
