use anyhow::Context;
use rusqlite::Connection;

use crate::db::repository;
use crate::models::entry::EntryContent;
use crate::time_fmt::format_created_at;

use super::ListFormat;

/// 300 spaces to push the ID far beyond the visible area in dmenu/rofi/wofi.
const SPACER: &str = concat!(
    "                                                  ",
    "                                                  ",
    "                                                  ",
    "                                                  ",
    "                                                  ",
    "                                                  ",
);

pub fn run(
    conn: &Connection,
    _format: &ListFormat,
    preview_length: usize,
    limit: usize,
) -> anyhow::Result<()> {
    let entries = repository::list_entries_preview(conn, limit, 0, preview_length)
        .context("failed to list entries")?;

    for entry in &entries {
        let id = entry.id.unwrap_or(0);

        let preview = if let Some(mask) = &entry.mask_text {
            mask.clone()
        } else {
            match &entry.content {
                EntryContent::Text(raw) => {
                    raw.split_whitespace().collect::<Vec<_>>().join(" ")
                }
                EntryContent::Image(_) => "[image]".to_string(),
            }
        };

        let time_ago = entry.created_at.as_ref().map(format_created_at).unwrap_or_default();
        println!("{time_ago} {preview}{SPACER}{id}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    fn collapse_whitespace(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn test_whitespace_collapse() {
        assert_eq!(collapse_whitespace("hello\n\nworld\t foo"), "hello world foo");
    }

    #[test]
    fn test_whitespace_collapse_only_spaces() {
        assert_eq!(collapse_whitespace("  a  b  "), "a b");
    }
}
