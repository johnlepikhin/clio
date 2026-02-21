pub mod source_app;

use arboard::{Clipboard, SetExtLinux};

use crate::errors::{AppError, Result};

#[derive(Debug)]
pub enum ClipboardContent {
    Text(String),
    Image {
        width: u32,
        height: u32,
        rgba_bytes: Vec<u8>,
    },
    Empty,
}

pub fn read_clipboard() -> Result<ClipboardContent> {
    let mut cb = Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;

    // Try text first
    match cb.get_text() {
        Ok(text) if !text.is_empty() => return Ok(ClipboardContent::Text(text)),
        _ => {}
    }

    // Try image
    if let Ok(img) = cb.get_image() {
        return Ok(ClipboardContent::Image {
            width: img.width as u32,
            height: img.height as u32,
            rgba_bytes: img.bytes.into_owned(),
        });
    }

    Ok(ClipboardContent::Empty)
}

pub fn write_clipboard_text(text: &str) -> Result<()> {
    let text = text.to_owned();
    std::thread::spawn(move || {
        let mut cb = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                eprintln!("clipboard error: {e}");
                return;
            }
        };
        if let Err(e) = cb.set().wait().text(text) {
            eprintln!("clipboard error: {e}");
        }
    });
    Ok(())
}

pub fn write_clipboard_image(rgba: &[u8], width: u32, height: u32) -> Result<()> {
    let rgba = rgba.to_vec();
    std::thread::spawn(move || {
        let mut cb = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                eprintln!("clipboard error: {e}");
                return;
            }
        };
        let img = arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: std::borrow::Cow::Owned(rgba),
        };
        if let Err(e) = cb.set().wait().image(img) {
            eprintln!("clipboard error: {e}");
        }
    });
    Ok(())
}
