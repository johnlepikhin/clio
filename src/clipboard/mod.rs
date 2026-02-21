pub mod source_app;

use arboard::{Clipboard, SetExtLinux};

#[cfg(target_os = "linux")]
use arboard::{GetExtLinux, LinuxClipboardKind};

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

#[cfg(target_os = "linux")]
pub fn read_selection(kind: LinuxClipboardKind) -> Result<ClipboardContent> {
    let mut cb = Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;

    // Try text first
    match cb.get().clipboard(kind).text() {
        Ok(text) if !text.is_empty() => return Ok(ClipboardContent::Text(text)),
        _ => {}
    }

    // Try image (only for Clipboard selection, PRIMARY rarely has images)
    if matches!(kind, LinuxClipboardKind::Clipboard) {
        if let Ok(img) = cb.get().clipboard(kind).image() {
            return Ok(ClipboardContent::Image {
                width: img.width as u32,
                height: img.height as u32,
                rgba_bytes: img.bytes.into_owned(),
            });
        }
    }

    Ok(ClipboardContent::Empty)
}

pub fn read_clipboard() -> Result<ClipboardContent> {
    #[cfg(target_os = "linux")]
    {
        read_selection(LinuxClipboardKind::Clipboard)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut cb = Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;
        match cb.get_text() {
            Ok(text) if !text.is_empty() => return Ok(ClipboardContent::Text(text)),
            _ => {}
        }
        if let Ok(img) = cb.get_image() {
            return Ok(ClipboardContent::Image {
                width: img.width as u32,
                height: img.height as u32,
                rgba_bytes: img.bytes.into_owned(),
            });
        }
        Ok(ClipboardContent::Empty)
    }
}

#[cfg(target_os = "linux")]
pub fn write_selection_text(kind: LinuxClipboardKind, text: &str) -> Result<()> {
    let text = text.to_owned();
    std::thread::spawn(move || {
        let mut cb = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                eprintln!("clipboard error: {e}");
                return;
            }
        };
        if let Err(e) = cb.set().wait().clipboard(kind).text(text) {
            eprintln!("clipboard error: {e}");
        }
    });
    Ok(())
}

pub fn write_clipboard_text(text: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        write_selection_text(LinuxClipboardKind::Clipboard, text)
    }
    #[cfg(not(target_os = "linux"))]
    {
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
}

/// Write text to clipboard synchronously (blocks until clipboard is set).
/// Use this for short-lived processes like `clio copy` that would exit
/// before a background thread finishes.
pub fn write_clipboard_text_sync(text: &str) -> Result<()> {
    let mut cb = Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;
    #[cfg(target_os = "linux")]
    {
        cb.set()
            .clipboard(LinuxClipboardKind::Clipboard)
            .text(text.to_owned())
            .map_err(|e| AppError::Clipboard(e.to_string()))?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        cb.set()
            .text(text.to_owned())
            .map_err(|e| AppError::Clipboard(e.to_string()))?;
    }
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
