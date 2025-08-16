// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use slint::PhysicalPosition;

slint::include_modules!();

mod viewmodel;

fn main() -> Result<()> {
    let app = AppWindow::new()?;

    enable_window_closing(&app);
    enable_window_positioning(&app);

    app.run()?;
    Ok(())
}

fn enable_window_positioning(app: &AppWindow) {
    let window_pos = app.window().position();
    app.set_window_x(window_pos.x as f32);
    app.set_window_y(window_pos.y as f32);

    callback!(on_position_window, |app|(x,y) {
        let pos = PhysicalPosition::new(x as i32, y as i32);
        app.window().set_position(pos);
    });
}

fn enable_window_closing(app: &AppWindow) {
    callback!(on_close, |app|() {
        app.window().dispatch_event(slint::platform::WindowEvent::CloseRequested);
    });
}
