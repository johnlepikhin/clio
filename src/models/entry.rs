use std::fmt;
use std::io::Cursor;
use std::time::Duration;

use chrono::{NaiveDateTime, Utc};
use image::{ImageFormat, RgbaImage};

use crate::errors::{AppError, Result};

/// ISO 8601 timestamp format matching SQLite's strftime('%Y-%m-%dT%H:%M:%f').
/// Use this constant for all chrono format calls to ensure consistency.
pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f";

/// ISO 8601 timestamp. Guaranteed to be in `TIMESTAMP_FORMAT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timestamp(String);

impl Timestamp {
    /// Create from a pre-validated string (e.g. from DB or strftime).
    /// Does NOT re-parse — caller guarantees format correctness.
    pub(crate) fn from_raw(s: String) -> Self {
        Self(s)
    }

    /// Parse and validate a timestamp string.
    pub fn parse(s: &str) -> std::result::Result<Self, chrono::ParseError> {
        NaiveDateTime::parse_from_str(s, TIMESTAMP_FORMAT)?;
        Ok(Self(s.to_owned()))
    }

    /// Current UTC time.
    pub fn now() -> Self {
        Self(Utc::now().format(TIMESTAMP_FORMAT).to_string())
    }

    /// Current UTC time + duration (for TTL).
    pub fn after(ttl: Duration) -> Self {
        let chrono_d = match chrono::Duration::from_std(ttl) {
            Ok(d) => d,
            Err(_) => {
                log::warn!("TTL duration {ttl:?} too large, using max duration");
                chrono::Duration::MAX
            }
        };
        let expires = Utc::now() + chrono_d;
        Self(expires.format(TIMESTAMP_FORMAT).to_string())
    }

    /// Parse into `NaiveDateTime` for calculations.
    /// Infallible — we validated at construction.
    pub fn to_naive(&self) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&self.0, TIMESTAMP_FORMAT)
            .expect("Timestamp invariant broken: invalid format")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Timestamp {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl rusqlite::types::FromSql for Timestamp {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = String::column_result(value)?;
        Timestamp::parse(&s).map_err(|e| {
            rusqlite::types::FromSqlError::Other(
                format!("invalid timestamp '{s}': {e}").into(),
            )
        })
    }
}

impl rusqlite::types::ToSql for Timestamp {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

/// Fixed-size blake3 hash (32 bytes). Replaces `Vec<u8>` to avoid heap allocation.
pub type ContentHash = [u8; 32];

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ContentType {
    Text,
    Image,
    Unknown,
}

impl ContentType {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn from_db_str(s: &str) -> Self {
        match s {
            "text" => Self::Text,
            "image" => Self::Image,
            other => {
                log::warn!("unknown content_type in database: {other:?}, treating as text");
                Self::Unknown
            }
        }
    }
}

/// Content payload of a clipboard entry.
#[derive(Debug, Clone)]
pub enum EntryContent {
    Text(String),
    Image(Vec<u8>), // PNG bytes
}

impl EntryContent {
    pub(crate) fn content_type(&self) -> ContentType {
        match self {
            Self::Text(_) => ContentType::Text,
            Self::Image(_) => ContentType::Image,
        }
    }

    /// Content type as a string slice (public API for external crates).
    pub fn content_type_str(&self) -> &'static str {
        self.content_type().as_str()
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn blob(&self) -> Option<&[u8]> {
        match self {
            Self::Image(b) => Some(b),
            _ => None,
        }
    }

    pub fn size_bytes(&self) -> usize {
        match self {
            Self::Text(s) => s.len(),
            Self::Image(b) => b.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub(crate) id: Option<i64>,
    pub(crate) content: EntryContent,
    pub(crate) content_hash: ContentHash,
    pub(crate) source_app: Option<String>,
    pub(crate) source_title: Option<String>,
    pub(crate) created_at: Option<Timestamp>,
    pub(crate) metadata: Option<String>,
    pub(crate) expires_at: Option<Timestamp>,
    pub(crate) mask_text: Option<String>,
}

impl ClipboardEntry {
    pub fn from_text(text: String, source_app: Option<String>) -> Self {
        let hash = compute_hash(text.as_bytes());
        Self {
            id: None,
            content: EntryContent::Text(text),
            content_hash: hash,
            source_app,
            source_title: None,
            created_at: None,
            metadata: None,
            expires_at: None,
            mask_text: None,
        }
    }

    pub fn from_image(
        width: u32,
        height: u32,
        rgba_bytes: Vec<u8>,
        source_app: Option<String>,
    ) -> Result<Self> {
        let png_bytes = encode_rgba_to_png(width, height, rgba_bytes)?;
        let hash = compute_hash(&png_bytes);
        Ok(Self {
            id: None,
            content: EntryContent::Image(png_bytes),
            content_hash: hash,
            source_app,
            source_title: None,
            created_at: None,
            metadata: None,
            expires_at: None,
            mask_text: None,
        })
    }

    /// Read-only access to content.
    pub fn content(&self) -> &EntryContent { &self.content }

    /// Read-only access to content hash.
    pub fn content_hash(&self) -> &ContentHash { &self.content_hash }

    /// Consume entry and return its content (for move semantics).
    pub fn into_content(self) -> EntryContent { self.content }

    pub fn content_size_bytes(&self) -> usize {
        self.content.size_bytes()
    }

    pub fn id(&self) -> Option<i64> { self.id }
    pub fn source_app(&self) -> Option<&str> { self.source_app.as_deref() }
    pub fn source_title(&self) -> Option<&str> { self.source_title.as_deref() }
    pub fn created_at(&self) -> Option<&Timestamp> { self.created_at.as_ref() }
    pub fn metadata(&self) -> Option<&str> { self.metadata.as_deref() }
    pub fn expires_at(&self) -> Option<&Timestamp> { self.expires_at.as_ref() }
    pub fn mask_text(&self) -> Option<&str> { self.mask_text.as_deref() }

    pub fn set_source_title(&mut self, title: Option<String>) { self.source_title = title; }
    pub fn set_expires_at(&mut self, ts: Option<Timestamp>) { self.expires_at = ts; }
    pub fn set_mask_text(&mut self, mask: Option<String>) { self.mask_text = mask; }

    /// Replace content and recompute hash atomically, preserving the invariant.
    pub fn set_content(&mut self, content: EntryContent) {
        let hash = match &content {
            EntryContent::Text(t) => compute_hash(t.as_bytes()),
            EntryContent::Image(b) => compute_hash(b),
        };
        self.content = content;
        self.content_hash = hash;
    }
}

pub fn compute_hash(data: &[u8]) -> ContentHash {
    *blake3::hash(data).as_bytes()
}

pub fn encode_rgba_to_png(width: u32, height: u32, rgba_bytes: Vec<u8>) -> Result<Vec<u8>> {
    let img = RgbaImage::from_raw(width, height, rgba_bytes).ok_or_else(|| {
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
        assert_eq!(entry.content.content_type(), ContentType::Text);
        assert_eq!(entry.content.text(), Some("test"));
        assert!(entry.content.blob().is_none());
        assert_eq!(entry.content_hash.len(), 32);
    }

    #[test]
    fn test_from_image() {
        let rgba = vec![255u8; 4 * 2 * 2]; // 2x2 white image
        let entry = ClipboardEntry::from_image(2, 2, rgba, None).unwrap();
        assert_eq!(entry.content.content_type(), ContentType::Image);
        assert!(entry.content.text().is_none());
        let blob = entry.content.blob().unwrap();
        assert_eq!(&blob[1..4], b"PNG");
    }

    #[test]
    fn test_content_type_roundtrip() {
        assert_eq!(ContentType::from_db_str("text"), ContentType::Text);
        assert_eq!(ContentType::from_db_str("image"), ContentType::Image);
        assert_eq!(ContentType::from_db_str("unknown"), ContentType::Unknown);
        assert_eq!(ContentType::from_db_str("other"), ContentType::Unknown);
    }

    #[test]
    fn test_content_size_bytes() {
        let entry = ClipboardEntry::from_text("hello".to_string(), None);
        assert_eq!(entry.content_size_bytes(), 5);
    }
}
