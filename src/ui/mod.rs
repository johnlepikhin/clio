pub mod entry_object;
pub mod entry_row;
pub mod window;

use std::path::PathBuf;

use gtk4::prelude::*;

use crate::config::Config;
use crate::errors::Result;

pub fn run_history_window(config: &Config, db_path: PathBuf) -> Result<()> {
    let app = gtk4::Application::builder()
        .application_id("com.clio.history")
        .build();

    let config = config.clone();
    app.connect_activate(move |app| {
        window::build_window(app, &config, db_path.clone());
    });

    app.run_with_args::<&str>(&[]);
    Ok(())
}
