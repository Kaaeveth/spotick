pub mod window;

use image::RgbaImage;

pub trait UiPlaybackInformation {
    fn set_thumbnail(&self, img: RgbaImage);
    fn set_title(&self, title: impl AsRef<str>);
    fn set_subtitle(&self, subtitle: impl AsRef<str>);
    fn set_playing(&self, playing: bool);
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
            0 // Not a border - Ignoring
        }
    };

    for y in 0..img.height() {
        for x in 0..img.width() {
            let dx = nearest_corner_distance(x, img.width());
            let dy = nearest_corner_distance(y, img.height());

            let is_inside = dx*dx + dy*dy <= radius*radius;
            let alpha = if is_inside || (dx == 0 && dy == 0) {255} else {0};
            
            let px = img.get_pixel_mut(x, y);
            px.0[3] = alpha;
        }
    }
}
