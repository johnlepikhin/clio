use crate::clipboard::{self, ClipboardContent};
use crate::errors::Result;

pub fn run() -> Result<()> {
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
            println!("Image: {width}x{height} PNG ({size_kb} KB)");
            Ok(())
        }
        ClipboardContent::Empty => {
            eprintln!("error: clipboard is empty");
            std::process::exit(1);
        }
    }
}
