use anyhow::bail;

use crate::clipboard::{self, ClipboardContent};

pub fn run() -> anyhow::Result<()> {
    match clipboard::read_clipboard()? {
        ClipboardContent::Text(text) => {
            print!("{text}");
            Ok(())
        }
        ClipboardContent::Image {
            width,
            height,
            rgba_bytes,
        } => {
            let size_kb = rgba_bytes.len() / 1024;
            println!("Image: {width}x{height} ({size_kb} KB RGBA)");
            Ok(())
        }
        ClipboardContent::Empty => {
            bail!("clipboard is empty");
        }
    }
}
