//! Binary protocol for clipboard content transfer via stdin/stdout.
//!
//! Wire format:
//! - `[1 byte]`  type: 0x01 = text, 0x02 = image
//! - `[4 bytes]` payload_len: u32 big-endian
//! - For text:  `[payload_len bytes]` UTF-8 string
//! - For image: `[4 bytes]` width u32 BE, `[4 bytes]` height u32 BE, `[payload_len bytes]` RGBA

use std::io::{Read, Write};

use crate::errors::{AppError, Result};

use super::ClipboardContent;

const TYPE_TEXT: u8 = 0x01;
const TYPE_IMAGE: u8 = 0x02;
const MAX_PAYLOAD_SIZE: usize = 256 * 1024 * 1024;

fn read_exact(r: &mut impl Read, buf: &mut [u8]) -> Result<()> {
    r.read_exact(buf)
        .map_err(|e| AppError::Clipboard(format!("stdin read: {e}")))
}

fn read_u32(r: &mut impl Read) -> Result<u32> {
    let mut buf = [0u8; 4];
    read_exact(r, &mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

pub fn encode(content: &ClipboardContent, w: &mut impl Write) -> Result<()> {
    match content {
        ClipboardContent::Text(text) => {
            let bytes = text.as_bytes();
            let len: u32 = bytes.len().try_into().map_err(|_| {
                AppError::Clipboard(format!("text too large: {} bytes", bytes.len()))
            })?;
            w.write_all(&[TYPE_TEXT])?;
            w.write_all(&len.to_be_bytes())?;
            w.write_all(bytes)?;
        }
        ClipboardContent::Image {
            width,
            height,
            rgba_bytes,
        } => {
            let len: u32 = rgba_bytes.len().try_into().map_err(|_| {
                AppError::Clipboard(format!("image too large: {} bytes", rgba_bytes.len()))
            })?;
            w.write_all(&[TYPE_IMAGE])?;
            w.write_all(&len.to_be_bytes())?;
            w.write_all(&width.to_be_bytes())?;
            w.write_all(&height.to_be_bytes())?;
            w.write_all(rgba_bytes)?;
        }
        ClipboardContent::Empty => {}
    }
    Ok(())
}

pub fn decode(r: &mut impl Read) -> Result<ClipboardContent> {
    let mut type_buf = [0u8; 1];
    read_exact(r, &mut type_buf)?;

    let payload_len = read_u32(r)? as usize;
    if payload_len > MAX_PAYLOAD_SIZE {
        return Err(AppError::Clipboard(format!(
            "payload too large: {payload_len} bytes (max {MAX_PAYLOAD_SIZE})"
        )));
    }

    match type_buf[0] {
        TYPE_TEXT => {
            let mut text_buf = vec![0u8; payload_len];
            read_exact(r, &mut text_buf)?;
            let text = String::from_utf8(text_buf)
                .map_err(|e| AppError::Clipboard(format!("invalid utf-8: {e}")))?;
            Ok(ClipboardContent::Text(text))
        }
        TYPE_IMAGE => {
            let width = read_u32(r)?;
            let height = read_u32(r)?;
            let expected_len = (width as usize)
                .checked_mul(height as usize)
                .and_then(|n| n.checked_mul(4));
            if expected_len != Some(payload_len) {
                return Err(AppError::Clipboard(format!(
                    "image dimension mismatch: {width}x{height}x4 != {payload_len} bytes"
                )));
            }
            let mut rgba_buf = vec![0u8; payload_len];
            read_exact(r, &mut rgba_buf)?;
            Ok(ClipboardContent::Image {
                width,
                height,
                rgba_bytes: rgba_buf,
            })
        }
        other => Err(AppError::Clipboard(format!(
            "unknown content type: 0x{other:02x}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_text() {
        let content = ClipboardContent::Text("hello world".into());
        let mut buf = Vec::new();
        encode(&content, &mut buf).unwrap();
        let decoded = decode(&mut &buf[..]).unwrap();
        match decoded {
            ClipboardContent::Text(t) => assert_eq!(t, "hello world"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn roundtrip_image() {
        let content = ClipboardContent::Image {
            width: 2,
            height: 3,
            rgba_bytes: vec![0xAA; 2 * 3 * 4],
        };
        let mut buf = Vec::new();
        encode(&content, &mut buf).unwrap();
        let decoded = decode(&mut &buf[..]).unwrap();
        match decoded {
            ClipboardContent::Image {
                width,
                height,
                rgba_bytes,
            } => {
                assert_eq!(width, 2);
                assert_eq!(height, 3);
                assert_eq!(rgba_bytes, vec![0xAA; 24]);
            }
            other => panic!("expected Image, got {other:?}"),
        }
    }

    #[test]
    fn decode_image_dimension_mismatch() {
        // Encode a 2x3 image but tamper with dimensions to 2x2 (expects 16 bytes, got 24)
        let content = ClipboardContent::Image {
            width: 2,
            height: 3,
            rgba_bytes: vec![0xAA; 2 * 3 * 4],
        };
        let mut buf = Vec::new();
        encode(&content, &mut buf).unwrap();

        // Tamper: change height from 3 to 2 (bytes at offset 9..13)
        buf[9..13].copy_from_slice(&2u32.to_be_bytes());

        let err = decode(&mut &buf[..]).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("dimension mismatch"),
            "expected dimension mismatch error, got: {msg}"
        );
    }

    #[test]
    fn encode_empty_is_noop() {
        let mut buf = Vec::new();
        encode(&ClipboardContent::Empty, &mut buf).unwrap();
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_payload_too_large() {
        // Craft a header with payload_len > MAX_PAYLOAD_SIZE
        let mut buf = Vec::new();
        buf.push(TYPE_TEXT); // type = text
        let huge_len: u32 = (MAX_PAYLOAD_SIZE as u32) + 1;
        buf.extend_from_slice(&huge_len.to_be_bytes());
        let result = decode(&mut &buf[..]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("too large"), "expected 'too large' error, got: {err}");
    }

    #[test]
    fn decode_unknown_type() {
        let mut buf = Vec::new();
        buf.push(0xFF); // unknown type
        buf.extend_from_slice(&4u32.to_be_bytes()); // payload_len = 4
        buf.extend_from_slice(b"test");
        let result = decode(&mut &buf[..]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown content type"), "expected 'unknown content type' error, got: {err}");
    }

    #[test]
    fn decode_truncated_input() {
        // Only type byte, no payload length
        let buf = vec![TYPE_TEXT];
        let result = decode(&mut &buf[..]);
        assert!(result.is_err());
    }
}
