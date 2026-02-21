pub mod serve;
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
// Write (fire-and-forget) — used by the watch daemon to serve clipboard
// in the background via `set().wait()`.
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

// ---------------------------------------------------------------------------
// Write — for short-lived processes (`clio copy`, `clio history`).
// On Linux, spawns a background `_serve-clipboard` process that holds
// selection ownership until another app takes the clipboard.
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn spawn_clipboard_server(content: &ClipboardContent) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe()
        .map_err(|e| AppError::Clipboard(format!("current_exe: {e}")))?;

    let mut child = Command::new(exe)
        .arg("_serve-clipboard")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| AppError::Clipboard(format!("spawn: {e}")))?;

    let stdin = child
        .stdin
        .as_mut()
        .ok_or_else(|| AppError::Clipboard("no stdin".into()))?;

    match content {
        ClipboardContent::Text(text) => {
            let bytes = text.as_bytes();
            stdin.write_all(&[0x01])?;
            stdin.write_all(&(bytes.len() as u32).to_be_bytes())?;
            stdin.write_all(bytes)?;
        }
        ClipboardContent::Image {
            width,
            height,
            rgba_bytes,
        } => {
            stdin.write_all(&[0x02])?;
            stdin.write_all(&(rgba_bytes.len() as u32).to_be_bytes())?;
            stdin.write_all(&width.to_be_bytes())?;
            stdin.write_all(&height.to_be_bytes())?;
            stdin.write_all(rgba_bytes)?;
        }
        ClipboardContent::Empty => {
            return Ok(());
        }
    }

    // Drop stdin so the child can finish reading, then detach.
    drop(child);
    Ok(())
}

pub fn write_clipboard_text_sync(text: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        spawn_clipboard_server(&ClipboardContent::Text(text.to_owned()))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut cb = open_clipboard()?;
        cb.set()
            .text(text.to_owned())
            .map_err(|e| AppError::Clipboard(e.to_string()))?;
        Ok(())
    }
}

pub fn write_clipboard_image_sync(width: u32, height: u32, rgba_bytes: Vec<u8>) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        spawn_clipboard_server(&ClipboardContent::Image {
            width,
            height,
            rgba_bytes,
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut cb = open_clipboard()?;
        let img = arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: std::borrow::Cow::Owned(rgba_bytes),
        };
        cb.set()
            .image(img)
            .map_err(|e| AppError::Clipboard(e.to_string()))?;
        Ok(())
    }
}
