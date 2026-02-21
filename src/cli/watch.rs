use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rusqlite::Connection;

use crate::clipboard::source_app;
use crate::clipboard::{self, ClipboardContent};
use crate::config::Config;
use crate::db::repository;
use crate::errors::Result;
use crate::models::entry::{compute_hash, ClipboardEntry};

pub fn run(conn: &Connection, config: &Config) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("failed to set Ctrl+C handler");

    eprintln!(
        "watching clipboard (interval: {}ms)...",
        config.watch_interval_ms
    );

    let mut last_hash: Option<Vec<u8>> = None;
    let interval = Duration::from_millis(config.watch_interval_ms);
    let max_size = config.max_entry_size_kb * 1024;

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(interval);

        let content = match clipboard::read_clipboard() {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (hash, entry) = match content {
            ClipboardContent::Text(text) => {
                let h = compute_hash(text.as_bytes());
                let e = ClipboardEntry::from_text(text, source_app::detect_source_app());
                (h, e)
            }
            ClipboardContent::Image {
                width,
                height,
                rgba_bytes,
            } => {
                match ClipboardEntry::from_image(
                    width,
                    height,
                    &rgba_bytes,
                    source_app::detect_source_app(),
                ) {
                    Ok(e) => {
                        let h = e.content_hash.clone();
                        (h, e)
                    }
                    Err(_) => continue,
                }
            }
            ClipboardContent::Empty => continue,
        };

        if last_hash.as_ref() == Some(&hash) {
            continue;
        }

        if entry.content_size_bytes() as u64 > max_size {
            eprintln!(
                "skipping entry: size {} KB exceeds limit {} KB",
                entry.content_size_bytes() / 1024,
                config.max_entry_size_kb
            );
            last_hash = Some(hash);
            continue;
        }

        match repository::save_or_update(conn, &entry, config.max_history) {
            Ok(_) => {
                last_hash = Some(hash);
            }
            Err(e) => {
                eprintln!("error saving entry: {e}");
            }
        }
    }

    eprintln!("watch stopped");
    Ok(())
}
