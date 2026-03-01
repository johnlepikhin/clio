use std::sync::LazyLock;

use anyhow::Context;
use regex::Regex;
use rusqlite::Connection;

use crate::db::repository;
use crate::models::entry::EntryContent;
use crate::time_fmt::format_created_at;

use super::ListFormat;

static WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\r\n\t ]+").unwrap());

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
                    let collapsed = WS_RE.replace_all(raw, " ");
                    collapsed.trim().to_string()
                }
                EntryContent::Image(_) => "[image]".to_string(),
            }
        };

        let time_ago = format_created_at(entry.created_at.as_deref().unwrap_or(""));
        println!("{time_ago} {preview}{SPACER}{id}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitespace_collapse() {
        let result = WS_RE.replace_all("hello\n\nworld\t foo", " ");
        assert_eq!(result.trim(), "hello world foo");
    }

    #[test]
    fn test_whitespace_collapse_only_spaces() {
        let result = WS_RE.replace_all("  a  b  ", " ");
        assert_eq!(result.trim(), "a b");
    }
}
