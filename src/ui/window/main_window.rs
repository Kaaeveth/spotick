use std::{sync::Arc, time::Duration};

use anyhow::Result;
use i_slint_backend_winit::winit::platform::windows::WindowAttributesExtWindows;
use image::RgbaImage;
use slint::{
    ComponentHandle, Image, LogicalSize, PhysicalPosition, Rgba8Pixel, SharedPixelBuffer,
    ToSharedString, Weak,
};
use tokio::sync::watch::channel;

use crate::{
    callback, save_changes_in_settings,
    service::{AlbumCover, BaseService, PlaybackChangedEvent, SharedMediaService},
    ui::{
        apply_border_radius, get_window_creation_settings,
        window::{SettingsWindow, SlintMainWindow, Window},
    },
};

pub struct MainWindow {
    ui: SlintMainWindow,
    settings_window: SettingsWindow,
    media_service: SharedMediaService,
}

impl MainWindow {
    pub async fn new(media_service: SharedMediaService, settings: SettingsWindow) -> Result<Self> {
        let _guard_settings =
            get_window_creation_settings().change(|attr| attr.with_skip_taskbar(true));
        let app = MainWindow {
            ui: SlintMainWindow::new()?,
            settings_window: settings,
            media_service,
        };

        app.ui.set_initial_thumbnail();
        app.connect_settings();
        app.connect_media_info().await;
        app.enable_app_quit();
        app.enable_window_positioning().await;
        app.enable_window_scaling().await;
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

    async fn connect_media_info(&self) {
        let srv = self.media_service.clone();
        let wui = self.ui.as_weak();
        MainWindow::update_track(&srv, &wui).await;
        MainWindow::update_playback(&srv, &wui).await;

        tokio::spawn(async move {
            let mut media_events = srv.read().await.subscribe();
            loop {
                let Ok(e) = media_events.recv().await else {
                    break;
                };

                match e {
                    PlaybackChangedEvent::TrackChanged => {
                        MainWindow::update_track(&srv, &wui).await;
                    }
                    PlaybackChangedEvent::Play | PlaybackChangedEvent::Pause => {
                        MainWindow::update_playback(&srv, &wui).await;
                    }
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

    async fn enable_window_scaling(&self) {
        let app = &self.ui;
        let mut scale_change_rv = self.settings_window.subscribe_scale_changed();
        let settings = self.settings_window.get_settings();

        // Set initial scale
        // Since the window is not yet created, we have to queue the rescale onto the event loop
        // which should start shortly after this call.
        {
            let spotick_settings = settings.read().await;
            let initial_scale = spotick_settings.get_settings().main_window_scale;
            app.as_weak()
                .upgrade_in_event_loop(move |app| {
                    app.rescale(initial_scale);
                })
                .unwrap();
        }

        let app = app.as_weak();
        tokio::spawn(async move {
            loop {
                if let Err(_) = scale_change_rv.changed().await {
                    break;
                }

                let scale = scale_change_rv.borrow_and_update().clone();
                let _ = app.upgrade_in_event_loop(move |app| {
                    app.rescale(scale);
                });
            }
        });
    }

    async fn enable_window_positioning(&self) {
        let app = &self.ui;
        let settings = self.settings_window.get_settings();

        // Set initial position
        {
            let spotick_settings = settings.read().await;
            let initial_pos = spotick_settings.get_settings().main_window_pos.clone();
            app.set_window_x(initial_pos.x as f32);
            app.set_window_y(initial_pos.y as f32);
            app.window().set_position(initial_pos);
        }

        // Channel for sending notifications of window position changes
        let (pos_tx, mut pos_rv) = channel(PhysicalPosition::new(-1, -1));
        pos_rv.mark_unchanged();

        callback!(on_position_window, |app, x, y| {
            let pos = PhysicalPosition::new(x as i32, y as i32);
            app.window().set_position(pos);
            let _ = pos_tx.send_replace(pos);
        });

        save_changes_in_settings!(pos_rv, settings, |sg| {
            let spotick_settings = sg.get_settings_mut();
            spotick_settings.main_window_pos = pos_rv.borrow().clone();
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
        let buffer = RgbaImage::from_raw(img_size.width, img_size.height, img.as_bytes().to_vec())
            .expect("Invalid placeholder image format");

        self.set_thumbnail(buffer);
    }

    fn rescale(&self, scale: f32) {
        let width = self.get_original_window_width() as f32 * scale;
        let height = self.get_original_window_height() as f32 * scale;

        // We set the window size through a platform event
        // instead of using [Window::set_size] since this method
        // doesn't work when setting the initial scale at app startup.
        // idk why, probably because the window doesn't exist yet
        // but events (using dispatch_event) are still getting queued
        self.window()
            .dispatch_event(slint::platform::WindowEvent::Resized {
                size: LogicalSize::new(width, height),
            });

        self.window()
            .dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged {
                scale_factor: scale,
            });
    }
}

impl Window<SlintMainWindow> for MainWindow {
    fn component(&self) -> &SlintMainWindow {
        &self.ui
    }
}
