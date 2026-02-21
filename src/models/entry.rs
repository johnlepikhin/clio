use std::io::Cursor;

use image::{ImageFormat, RgbaImage};

use crate::errors::{AppError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    Text,
    Image,
    Unknown,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "text" => Self::Text,
            "image" => Self::Image,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub id: Option<i64>,
    pub content_type: ContentType,
    pub text_content: Option<String>,
    pub blob_content: Option<Vec<u8>>,
    pub content_hash: Vec<u8>,
    pub source_app: Option<String>,
    pub created_at: Option<String>,
    pub metadata: Option<String>,
}

impl ClipboardEntry {
    pub fn from_text(text: String, source_app: Option<String>) -> Self {
        let hash = compute_hash(text.as_bytes());
        Self {
            id: None,
            content_type: ContentType::Text,
            text_content: Some(text),
            blob_content: None,
            content_hash: hash,
            source_app,
            created_at: None,
            metadata: None,
        }
    }

    pub fn from_image(
        width: u32,
        height: u32,
        rgba_bytes: &[u8],
        source_app: Option<String>,
    ) -> Result<Self> {
        let png_bytes = encode_rgba_to_png(width, height, rgba_bytes)?;
        let hash = compute_hash(&png_bytes);
        Ok(Self {
            id: None,
            content_type: ContentType::Image,
            text_content: None,
            blob_content: Some(png_bytes),
            content_hash: hash,
            source_app,
            created_at: None,
            metadata: None,
        })
    }

    #[allow(dead_code)]
    pub fn from_unknown(raw_bytes: Vec<u8>, source_app: Option<String>) -> Self {
        let hash = compute_hash(&raw_bytes);
        Self {
            id: None,
            content_type: ContentType::Unknown,
            text_content: None,
            blob_content: Some(raw_bytes),
            content_hash: hash,
            source_app,
            created_at: None,
            metadata: None,
        }
    }

    pub fn content_size_bytes(&self) -> usize {
        if let Some(ref text) = self.text_content {
            text.len()
        } else if let Some(ref blob) = self.blob_content {
            blob.len()
        } else {
            0
        }
    }
}

pub fn compute_hash(data: &[u8]) -> Vec<u8> {
    blake3::hash(data).as_bytes().to_vec()
}

pub fn encode_rgba_to_png(width: u32, height: u32, rgba_bytes: &[u8]) -> Result<Vec<u8>> {
    let img = RgbaImage::from_raw(width, height, rgba_bytes.to_vec()).ok_or_else(|| {
        AppError::Image(image::ImageError::Parameter(
            image::error::ParameterError::from_kind(
                image::error::ParameterErrorKind::DimensionMismatch,
            ),
        ))
    })?;
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Png)?;
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_deterministic() {
        let h1 = compute_hash(b"hello");
        let h2 = compute_hash(b"hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
    }

    #[test]
    fn test_compute_hash_different_input() {
        let h1 = compute_hash(b"hello");
        let h2 = compute_hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_from_text() {
        let entry = ClipboardEntry::from_text("test".to_string(), None);
        assert_eq!(entry.content_type, ContentType::Text);
        assert_eq!(entry.text_content.as_deref(), Some("test"));
        assert!(entry.blob_content.is_none());
        assert_eq!(entry.content_hash.len(), 32);
    }

    #[test]
    fn test_from_image() {
        let rgba = vec![255u8; 4 * 2 * 2]; // 2x2 white image
        let entry = ClipboardEntry::from_image(2, 2, &rgba, None).unwrap();
        assert_eq!(entry.content_type, ContentType::Image);
        assert!(entry.text_content.is_none());
        assert!(entry.blob_content.is_some());
        // PNG bytes should start with PNG magic
        let blob = entry.blob_content.as_ref().unwrap();
        assert_eq!(&blob[1..4], b"PNG");
    }

    #[test]
    fn test_content_type_roundtrip() {
        assert_eq!(ContentType::from_str("text"), ContentType::Text);
        assert_eq!(ContentType::from_str("image"), ContentType::Image);
        assert_eq!(ContentType::from_str("unknown"), ContentType::Unknown);
        assert_eq!(ContentType::from_str("other"), ContentType::Unknown);
    }

    #[test]
    fn test_content_size_bytes() {
        let entry = ClipboardEntry::from_text("hello".to_string(), None);
        assert_eq!(entry.content_size_bytes(), 5);
    }
}
