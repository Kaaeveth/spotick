pub mod window;

use anyhow::Result;
use image::RgbaImage;

use crate::ui::window::get_window_creation_settings;

#[macro_export]
macro_rules! callback {
    ($prop:ident, |$app_ref:ident $(,)? $( $params:ident ),*| $handler:block) => {{
        $app_ref.$prop({
            let app_weak = $app_ref.as_weak();
            move |$( $params ),*| {
                let $app_ref = app_weak.unwrap();
                $handler
            }
        });
    }};
}

pub fn init_backend() -> Result<()> {
    let window_backend = i_slint_backend_winit::Backend::builder()
        .with_window_attributes_hook(|_| get_window_creation_settings().get_settings())
        .build()?;
    slint::platform::set_platform(Box::new(window_backend))?;
    Ok(())
}

/// Rounds the corners of [img] with the given [radius].
/// This is a naive implementation running on the CPU and not quite efficient.
/// Don't call it frequently.
pub fn apply_border_radius(img: &mut RgbaImage, radius: u32) {
    let nearest_corner_distance = |coord, axis_length| {
        if coord < radius {
            radius - coord
        } else if coord >= axis_length - radius {
            coord - (axis_length - radius - 1)
        } else {
            0 // Not a corner - Ignoring
        }
    };

    for y in 0..img.height() {
        let dy = nearest_corner_distance(y, img.height());
        if dy == 0 {
            continue;
        }
        for x in 0..img.width() {
            let dx = nearest_corner_distance(x, img.width());
            if dx == 0 {
                continue;
            }

            let is_inside = dx * dx + dy * dy <= radius * radius;
            if !is_inside {
                let px = img.get_pixel_mut(x, y);
                px.0[3] = 0;
            }
        }
    }
}
