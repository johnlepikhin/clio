pub(crate) mod protocol;
pub mod serve;
pub mod source_app;

use arboard::Clipboard;
use image::ImageFormat;
use log::error;

#[cfg(target_os = "linux")]
use arboard::{GetExtLinux, LinuxClipboardKind, SetExtLinux};

use crate::errors::{AppError, Result};
use crate::models::entry::{ContentHash, EntryContent};

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

impl ClipboardContent {
    /// Compute content hash for **change detection** in the watch loop.
    ///
    /// INVARIANT: For images, this hashes raw RGBA bytes (from arboard),
    /// while `ClipboardEntry::from_image()` hashes the PNG-encoded bytes.
    /// These hashes are NOT comparable across the two types. Change detection
    /// (comparing successive `ClipboardContent` hashes) and DB dedup
    /// (comparing `ClipboardEntry` hashes) operate in separate hash spaces.
    pub fn content_hash(&self) -> Option<ContentHash> {
        use crate::models::entry::compute_hash;
        match self {
            Self::Text(t) => Some(compute_hash(t.as_bytes())),
            Self::Image { rgba_bytes, .. } => Some(compute_hash(rgba_bytes)),
            Self::Empty => None,
        }
    }
}

pub fn open_clipboard() -> Result<Clipboard> {
    Ok(Clipboard::new()?)
}

// ---------------------------------------------------------------------------
// Read
// ---------------------------------------------------------------------------

/// Read a selection using an existing Clipboard instance (avoids reconnecting).
#[cfg(target_os = "linux")]
pub fn read_selection_with(
    cb: &mut Clipboard,
    kind: LinuxClipboardKind,
) -> Result<ClipboardContent> {
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

/// Read CLIPBOARD using an existing Clipboard instance.
pub fn read_clipboard_with(cb: &mut Clipboard) -> Result<ClipboardContent> {
    #[cfg(target_os = "linux")]
    {
        read_selection_with(cb, LinuxClipboardKind::Clipboard)
    }
    #[cfg(not(target_os = "linux"))]
    {
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

pub fn read_clipboard() -> Result<ClipboardContent> {
    let mut cb = open_clipboard()?;
    read_clipboard_with(&mut cb)
}

// ---------------------------------------------------------------------------
// Write (fire-and-forget) — used by the watch daemon to serve clipboard
// in the background via `set().wait()`.
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
pub fn write_selection_text(
    kind: LinuxClipboardKind,
    text: &str,
) -> Option<std::thread::JoinHandle<()>> {
    const CLIPBOARD_THREAD_STACK_SIZE: usize = 128 * 1024;
    let text = text.to_owned();
    std::thread::Builder::new()
        .stack_size(CLIPBOARD_THREAD_STACK_SIZE)
        .spawn(move || {
            let mut cb = match Clipboard::new() {
                Ok(cb) => cb,
                Err(e) => {
                    error!("clipboard write error: {e}");
                    return;
                }
            };
            if let Err(e) = cb.set().wait().clipboard(kind).text(text) {
                error!("clipboard write error: {e}");
            }
        })
        .ok()
}

// ---------------------------------------------------------------------------
// Write — for short-lived processes (`clio copy`, `clio history`).
// On Linux, spawns a background `_serve-clipboard` process that holds
// selection ownership until another app takes the clipboard.
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn spawn_clipboard_server(content: &ClipboardContent) -> Result<()> {
    use std::process::{Command, Stdio};

    if matches!(content, ClipboardContent::Empty) {
        return Ok(());
    }

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

    protocol::encode(content, stdin)?;

    // Register PID for targeted reaping, then detach.
    crate::platform::register_child_pid(child.id());
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
            ?;
        Ok(())
    }
}

/// Write an entry's content to the clipboard. Decodes PNG for images.
pub fn write_entry_to_clipboard(content: &EntryContent) -> Result<()> {
    match content {
        EntryContent::Text(text) => write_clipboard_text_sync(text),
        EntryContent::Image(png_bytes) => {
            let img = image::load_from_memory_with_format(png_bytes, ImageFormat::Png)?
                .to_rgba8();
            let (w, h) = img.dimensions();
            write_clipboard_image_sync(w, h, img.into_raw())
        }
    }
}

/// Restore the previous active clipboard entry, or clear clipboard if none exists.
pub fn restore_or_clear_clipboard(conn: &rusqlite::Connection) -> Result<()> {
    use crate::db::repository;
    match repository::get_latest_active(conn)? {
        Some(entry) => write_entry_to_clipboard(entry.content())?,
        None => write_clipboard_text_sync("")?,
    }
    Ok(())
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
            ?;
        Ok(())
    }
}
