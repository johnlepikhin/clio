pub mod source_app;

use arboard::Clipboard;

#[cfg(target_os = "linux")]
use arboard::{GetExtLinux, LinuxClipboardKind, SetExtLinux};

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

fn open_clipboard() -> Result<Clipboard> {
    Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))
}

// ---------------------------------------------------------------------------
// Read
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
pub fn read_selection(kind: LinuxClipboardKind) -> Result<ClipboardContent> {
    let mut cb = open_clipboard()?;

    match cb.get().clipboard(kind).text() {
        Ok(text) if !text.is_empty() => return Ok(ClipboardContent::Text(text)),
        _ => {}
    }

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
        let mut cb = open_clipboard()?;
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

// ---------------------------------------------------------------------------
// Write (fire-and-forget)
//
// These functions spawn a background thread that calls `set().wait()`,
// which on Wayland keeps serving the clipboard until another app copies.
// Errors inside the thread are logged to stderr but NOT propagated to the
// caller — the returned `Ok(())` only means the thread was spawned.
// For short-lived processes (e.g. `clio copy`) use `write_clipboard_text_sync`.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Write (synchronous) — for short-lived processes like `clio copy`.
// ---------------------------------------------------------------------------

pub fn write_clipboard_text_sync(text: &str) -> Result<()> {
    let mut cb = open_clipboard()?;
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
