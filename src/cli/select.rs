use std::io::BufRead;

use anyhow::Context;
use log::debug;
use rusqlite::Connection;

use crate::clipboard;
use crate::db::repository;
use crate::models::entry::EntryContent;

use super::SelectSource;

pub fn run(conn: &Connection, source: &SelectSource) -> anyhow::Result<()> {
    let id = match source {
        SelectSource::Id { id } => *id,
        SelectSource::Stdin => parse_id_from_stdin()?,
    };

    debug!("selecting entry id={id}");

    let entry = repository::get_entry_content(conn, id)
        .context("failed to read entry")?
        .ok_or_else(|| anyhow::anyhow!("entry {id} not found"))?;

    match &entry.content {
        EntryContent::Text(text) => {
            clipboard::write_clipboard_text_sync(text)?;
            #[cfg(target_os = "linux")]
            clipboard::write_selection_text(arboard::LinuxClipboardKind::Primary, text);
            debug!("text copied to clipboard");
        }
        EntryContent::Image(png_bytes) => {
            let img = image::load_from_memory_with_format(png_bytes, image::ImageFormat::Png)
                .context("failed to decode PNG")?
                .to_rgba8();
            let (w, h) = img.dimensions();
            clipboard::write_clipboard_image_sync(w, h, img.into_raw())?;
            debug!("image copied to clipboard");
        }
    }

    Ok(())
}

fn parse_id_from_stdin() -> anyhow::Result<i64> {
    let line = std::io::stdin()
        .lock()
        .lines()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no input on stdin"))?
        .context("failed to read stdin")?;

    parse_id_from_line(&line)
}

fn parse_id_from_line(line: &str) -> anyhow::Result<i64> {
    let trimmed = line.trim();
    let token = trimmed
        .rsplit_once(' ')
        .map(|(_, id)| id)
        .unwrap_or(trimmed);
    token
        .parse::<i64>()
        .context("failed to parse entry ID from stdin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_id_from_dmenu_format() {
        let spacer = " ".repeat(300);
        let line = format!("5m ago some clipboard text{spacer}42");
        assert_eq!(parse_id_from_line(&line).unwrap(), 42);
    }

    #[test]
    fn test_parse_id_only() {
        assert_eq!(parse_id_from_line("7").unwrap(), 7);
    }

    #[test]
    fn test_parse_id_with_trailing_whitespace() {
        let spacer = " ".repeat(300);
        let line = format!("just now hello{spacer}15 ");
        assert_eq!(parse_id_from_line(&line).unwrap(), 15);
    }

    #[test]
    fn test_parse_id_empty() {
        assert!(parse_id_from_line("").is_err());
    }

    #[test]
    fn test_parse_id_invalid() {
        let spacer = " ".repeat(300);
        let line = format!("5m ago text{spacer}abc");
        assert!(parse_id_from_line(&line).is_err());
    }
}
