use crate::storage;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn attachments_dir() -> PathBuf {
    let p = storage::data_dir().join("attachments");
    let _ = std::fs::create_dir_all(&p);
    p
}

fn timestamp_filename(ext: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("img-{}.{}", ts, ext)
}

/// Try to get an image from the clipboard and save it. Returns the saved
/// absolute path on success.
pub fn paste_clipboard_image() -> Option<PathBuf> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let img = cb.get_image().ok()?;
    let width = img.width as u32;
    let height = img.height as u32;
    let bytes: Vec<u8> = img.bytes.into_owned();
    if width == 0 || height == 0 || bytes.len() < (width * height * 4) as usize {
        return None;
    }
    let buffer = image::RgbaImage::from_raw(width, height, bytes)?;
    let path = attachments_dir().join(timestamp_filename("png"));
    image::DynamicImage::ImageRgba8(buffer)
        .save_with_format(&path, image::ImageFormat::Png)
        .ok()?;
    Some(path)
}

/// Build a markdown image link using a relative path if possible.
pub fn markdown_link_for(path: &Path) -> String {
    // Convert to forward slashes for cross-platform markdown links.
    let s = path
        .to_string_lossy()
        .replace('\\', "/")
        .replace(' ', "%20");
    format!("![]({})", s)
}
