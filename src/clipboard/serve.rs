//! Background clipboard server for X11 systems without a clipboard manager.

use arboard::Clipboard;

#[cfg(target_os = "linux")]
use arboard::{LinuxClipboardKind, SetExtLinux};

use crate::errors::Result;

use super::ClipboardContent;

pub fn run() -> Result<()> {
    let mut stdin = std::io::stdin().lock();
    let content = super::protocol::decode(&mut stdin)?;

    let mut cb = Clipboard::new()?;

    match content {
        ClipboardContent::Text(text) => {
            #[cfg(target_os = "linux")]
            cb.set()
                .wait()
                .clipboard(LinuxClipboardKind::Clipboard)
                .text(text)
                ?;

            #[cfg(not(target_os = "linux"))]
            cb.set()
                .text(text)
                ?;
        }
        ClipboardContent::Image {
            width,
            height,
            rgba_bytes,
        } => {
            let img = arboard::ImageData {
                width: width as usize,
                height: height as usize,
                bytes: std::borrow::Cow::Owned(rgba_bytes),
            };

            #[cfg(target_os = "linux")]
            cb.set()
                .wait()
                .clipboard(LinuxClipboardKind::Clipboard)
                .image(img)
                ?;

            #[cfg(not(target_os = "linux"))]
            cb.set()
                .image(img)
                ?;
        }
        ClipboardContent::Empty => {}
    }

    Ok(())
}
