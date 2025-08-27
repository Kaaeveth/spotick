use std::sync::Arc;

use i_slint_backend_winit::winit::platform::windows::WindowAttributesExtWindows;
use image::RgbaImage;
use slint::{ComponentHandle, Image, PhysicalPosition, Rgba8Pixel, SharedPixelBuffer, ToSharedString, Weak};
use anyhow::Result;

use crate::{
    callback,
    service::{BaseService, PlaybackChangedEvent, SharedMediaService, AlbumCover}, 
    ui::{
        apply_border_radius, get_window_creation_settings,
        window::{SettingsWindow, SlintMainWindow, Window}
    }
};

pub struct MainWindow {
    ui: SlintMainWindow,
    settings_window: SettingsWindow,
    media_service: SharedMediaService
}

impl MainWindow {
    pub fn new(media_service: SharedMediaService, settings: SettingsWindow) -> Result<Self> {
        let _guard_settings = get_window_creation_settings().change(|attr| {
            attr.with_skip_taskbar(true)
        });
        let app = MainWindow {
            ui: SlintMainWindow::new()?,
            settings_window: settings,
            media_service
        };

        app.ui.set_initial_thumbnail();
        app.connect_settings();
        app.connect_media_info();
        app.enable_app_quit();
        app.enable_window_positioning();
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

        macro_rules! connect_to_media_service {
            ($srv:expr, $media_method:ident, $ui_callback:ident) => {
                let srv = Arc::downgrade($srv);
                callback!($ui_callback, |_app| {
                    tokio::spawn({
                        let srv = srv.clone();
                        async move {
                            if let Some(srv) = srv.upgrade() {
                                if let Err(e) = srv.write().await.$media_method().await {
                                    log::error!("Error in {}: {}", stringify!($media_method), e);
                                }
                            }
                        }
                    });
                });
            };
        }

        connect_to_media_service!(&self.media_service, toggle_playback, on_toggle_play);
        connect_to_media_service!(&self.media_service, next_track, on_next_track);
        connect_to_media_service!(&self.media_service, previous_track, on_previous_track);
    }

    async fn update_track(srv: &SharedMediaService, wui: &Weak<SlintMainWindow>) {
        let srv_lock = srv.clone().read_owned().await;
        let _ = wui.upgrade_in_event_loop(move |ui| {
            if let Some(current_media_track) = srv_lock.current_track() {
                ui.set_track_title(current_media_track.title.to_shared_string());
                ui.set_track_subtitle(current_media_track.artist.to_shared_string());
                if let AlbumCover::Image(img) = &current_media_track.album_cover {
                    ui.set_thumbnail(img.clone());
                }
            } else {
                ui.set_track_title("No Title".into());
                ui.set_track_subtitle("...".into());
                ui.set_initial_thumbnail();
            }
        });
    }

    async fn update_playback(srv: &SharedMediaService, wui: &Weak<SlintMainWindow>) {
        let srv_lock = srv.clone().read_owned().await;
        let _ = wui.upgrade_in_event_loop(move |ui| {
            let playback_state = srv_lock.current_playback_state();
            ui.set_playing(playback_state.is_playing);
        });
    }

    fn connect_media_info(&self) {
        let srv = self.media_service.clone();
        let wui = self.ui.as_weak();
        tokio::spawn(async move {
            let mut media_events = srv.read().await.subscribe();

            MainWindow::update_track(&srv, &wui).await;
            MainWindow::update_playback(&srv, &wui).await;

            loop {
                let Ok(e) = media_events.recv().await else {
                    break;
                };

                match e {
                    PlaybackChangedEvent::TrackChanged => {
                        MainWindow::update_track(&srv, &wui).await;
                    },
                    PlaybackChangedEvent::Play | PlaybackChangedEvent::Pause => {
                        MainWindow::update_playback(&srv, &wui).await;
                    },
                    _ => {}
                }
            }
        });
    }

    fn connect_settings(&self) {
        let settings = self.settings_window.get_settings();
        let wui = self.as_weak();
        tokio::spawn(async move {
            let settings = settings.clone();
            let mut settings_recv = settings.read().await.subscribe();
            loop {
                let always_on_top = settings.read().await.get_settings().always_on_top;

                let _ = wui.upgrade_in_event_loop(move |ui| {
                    ui.set_on_top(always_on_top);
                });
                if let Err(_) = settings_recv.recv().await {
                    break;
                }
            }
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
}

impl SlintMainWindow {
    fn set_thumbnail(&self, mut img: RgbaImage) {
        // Apply image decorations
        apply_border_radius(&mut img, self.get_thumbnail_border_radius() as u32);

        let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            img.as_raw(),
            img.width(),
            img.height(),
        );
        let image = Image::from_rgba8(buffer);
        self.set_thumbnail_img(image);
    }

    /// Sets the initial (empty) album cover image
    /// using the placeholder (thumbnail-placeholder)
    /// defined in the Slint file of the [AppWindow].
    /// This is necessary for image decorations (border-radius,...)
    /// to be applied to the initial cover image.
    fn set_initial_thumbnail(&self) {
        let img = self.get_thumbnail_placeholder();
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

impl Window<SlintMainWindow> for MainWindow {
    fn component(&self) -> &SlintMainWindow {
        &self.ui
    }
}
