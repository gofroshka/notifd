use image::{Rgba, RgbaImage};
use std::path::{Path, PathBuf};

use crate::hints::ImageData;

/// Decode the raw `image-data` hint into a PNG inside the cache directory.
/// Returns the path only when the image is well-formed and could be written.
pub fn cache(image: &ImageData, dir: &Path, name: u32) -> Option<PathBuf> {
    let (width, height) = (image.width, image.height);
    let channels = image.channels as usize;

    if image.bits_per_sample != 8 || width == 0 || height == 0 || !(3..=4).contains(&channels) {
        return None;
    }

    let rowstride = if image.rowstride == 0 {
        width as usize * channels
    } else {
        image.rowstride as usize
    };

    let needed = (height as usize - 1) * rowstride + width as usize * channels;
    if image.data.len() < needed {
        return None;
    }

    let mut buffer = RgbaImage::new(width, height);
    for y in 0..height as usize {
        let row = &image.data[y * rowstride..];
        for x in 0..width as usize {
            let offset = x * channels;
            let alpha = if channels == 4 { row[offset + 3] } else { 255 };
            buffer.put_pixel(
                x as u32,
                y as u32,
                Rgba([row[offset], row[offset + 1], row[offset + 2], alpha]),
            );
        }
    }

    std::fs::create_dir_all(dir).ok()?;
    let path = dir.join(format!("{name}.png"));
    buffer.save(&path).ok()?;
    Some(path)
}
