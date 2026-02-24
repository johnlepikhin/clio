mod actions;
mod cli;
mod clipboard;
mod config;
mod db;
mod errors;
mod models;
#[cfg(feature = "ui")]
mod ui;

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use log::debug;

use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    // Fast path: _serve-clipboard needs no config/db — handle before parsing anything heavy.
    if let Some("_serve-clipboard") = std::env::args().nth(1).as_deref() {
        return clipboard::serve::run().map_err(Into::into);
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();
    debug!("clio starting");

    let cli = Cli::parse();
    let config = config::load_config(cli.config.as_deref()).context("failed to load config")?;
    debug!("config loaded, max_history={}, watch_interval={}ms", config.max_history, config.watch_interval_ms);

    let db_path = config
        .db_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| config::data_dir().join("clio.db"));

    match cli.command {
        Commands::Show => cli::show::run(),
        Commands::Copy { ttl, mask_with } => {
            let conn = db::init_db(&db_path).context("failed to initialize database")?;
            cli::copy::run(&conn, &config, ttl, mask_with)
        }
        Commands::Watch => {
            let conn = db::init_db(&db_path).context("failed to initialize database")?;
            cli::watch::run(&conn, &config)
        }
        #[cfg(feature = "ui")]
        Commands::History => cli::history::run(&config, db_path).map_err(Into::into),
        Commands::Config { ref command } => {
            let config_path = cli
                .config
                .clone()
                .unwrap_or_else(config::default_config_path);
            cli::config::run(&config_path, command)
        }
        // Handled by early return above; unreachable via normal flow.
        Commands::ServeClipboard => clipboard::serve::run().map_err(Into::into),
    }
}
