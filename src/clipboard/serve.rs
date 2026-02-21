//! Background clipboard server for X11 systems without a clipboard manager.
//!
//! Binary protocol on stdin:
//! - `[1 byte]`  type: 0x01 = text, 0x02 = image
//! - `[4 bytes]` payload_len: u32 big-endian
//! - For text:  `[payload_len bytes]` UTF-8 string
//! - For image: `[4 bytes]` width u32 BE, `[4 bytes]` height u32 BE, `[payload_len bytes]` RGBA

use std::io::Read;

use arboard::Clipboard;

#[cfg(target_os = "linux")]
use arboard::{LinuxClipboardKind, SetExtLinux};

use crate::errors::{AppError, Result};

const TYPE_TEXT: u8 = 0x01;
const TYPE_IMAGE: u8 = 0x02;

fn read_exact(r: &mut impl Read, buf: &mut [u8]) -> Result<()> {
    r.read_exact(buf)
        .map_err(|e| AppError::Clipboard(format!("stdin read: {e}")))
}

fn read_u32(r: &mut impl Read) -> Result<u32> {
    let mut buf = [0u8; 4];
    read_exact(r, &mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

pub fn run() -> Result<()> {
    let mut stdin = std::io::stdin().lock();

    let mut type_buf = [0u8; 1];
    read_exact(&mut stdin, &mut type_buf)?;

    let payload_len = read_u32(&mut stdin)? as usize;

    let mut cb = Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;

    match type_buf[0] {
        TYPE_TEXT => {
            let mut text_buf = vec![0u8; payload_len];
            read_exact(&mut stdin, &mut text_buf)?;
            let text = String::from_utf8(text_buf)
                .map_err(|e| AppError::Clipboard(format!("invalid utf-8: {e}")))?;

            #[cfg(target_os = "linux")]
            cb.set()
                .wait()
                .clipboard(LinuxClipboardKind::Clipboard)
                .text(text)
                .map_err(|e| AppError::Clipboard(e.to_string()))?;

            #[cfg(not(target_os = "linux"))]
            cb.set()
                .text(text)
                .map_err(|e| AppError::Clipboard(e.to_string()))?;
        }
        TYPE_IMAGE => {
            let width = read_u32(&mut stdin)? as usize;
            let height = read_u32(&mut stdin)? as usize;
            let rgba_len = payload_len;
            let mut rgba_buf = vec![0u8; rgba_len];
            read_exact(&mut stdin, &mut rgba_buf)?;

            let img = arboard::ImageData {
                width,
                height,
                bytes: std::borrow::Cow::Owned(rgba_buf),
            };

            #[cfg(target_os = "linux")]
            cb.set()
                .wait()
                .clipboard(LinuxClipboardKind::Clipboard)
                .image(img)
                .map_err(|e| AppError::Clipboard(e.to_string()))?;

            #[cfg(not(target_os = "linux"))]
            cb.set()
                .image(img)
                .map_err(|e| AppError::Clipboard(e.to_string()))?;
        }
        other => {
            return Err(AppError::Clipboard(format!(
                "unknown content type: 0x{other:02x}"
            )));
        }
    }

    Ok(())
}
