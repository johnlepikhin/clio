pub mod entry_object;
pub mod entry_row;
pub mod window;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::clipboard;
use crate::config::Config;
use crate::errors::Result;

pub fn run_history_window(config: &Config, db_path: PathBuf) -> Result<()> {
    // Skip Vulkan/NGL GPU initialization — Cairo is sufficient for a simple list UI
    // and avoids ~2s of shader compilation + compositor round-trips.
    // Disable AT-SPI accessibility bus — saves ~600ms of D-Bus setup.
    // Users can override by setting these env vars before launching clio.
    //
    // SAFETY: called before any threads are spawned (pre-GTK init).
    unsafe {
        if std::env::var_os("GSK_RENDERER").is_none() {
            std::env::set_var("GSK_RENDERER", "cairo");
        }
        if std::env::var_os("GTK_A11Y").is_none() {
            std::env::set_var("GTK_A11Y", "none");
        }
    }

    let app = gtk4::Application::builder()
        .application_id("com.clio.history")
        .build();

    let selected: Rc<RefCell<Option<window::SelectedContent>>> = Rc::new(RefCell::new(None));

    let config = config.clone();
    let sel = selected.clone();
    app.connect_activate(move |app| {
        window::build_window(app, &config, db_path.clone(), sel.clone());
    });

    app.run_with_args::<&str>(&[]);

    // Write clipboard AFTER GTK event loop exits.
    // On Linux this spawns a background _serve-clipboard process that holds
    // selection ownership until another app takes the clipboard.
    if let Some(content) = selected.borrow_mut().take() {
        write_selected_to_clipboard(content)?;
    }

    Ok(())
}

fn write_selected_to_clipboard(content: window::SelectedContent) -> Result<()> {
    match content {
        window::SelectedContent::Text(text) => clipboard::write_clipboard_text_sync(&text),
        window::SelectedContent::Image {
            rgba,
            width,
            height,
        } => clipboard::write_clipboard_image_sync(width, height, rgba),
    }
}
