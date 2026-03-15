use anyhow::Context;
use clap::Parser;
use log::debug;

use clio::cli::{Cli, Commands};
use clio::{clipboard, config, db};

fn main() -> anyhow::Result<()> {
    // Fast path: _serve-clipboard needs no config/db — handle before parsing anything heavy.
    if std::env::args().nth(1).as_deref() == Some(clio::cli::SERVE_CLIPBOARD_CMD) {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();
        return clipboard::serve::run().map_err(Into::into);
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    debug!("clio starting");

    let cli = Cli::parse();
    let config = config::load_config(cli.config.as_deref()).context("failed to load config")?;
    debug!("config loaded, max_history={}, watch_interval={}ms", config.max_history, config.watch_interval.as_millis());

    let db_path = config::resolve_db_path(&config);

    match cli.command {
        Commands::Show => clio::cli::show::run(),
        Commands::Copy { ttl, mask_with } => {
            let conn = db::init_db(&db_path).context("failed to initialize database")?;
            clio::cli::copy::run(&conn, &config, ttl, mask_with)
        }
        Commands::Watch => {
            let conn = db::init_db(&db_path).context("failed to initialize database")?;
            clio::cli::watch::run(&conn, &config)
        }
        Commands::History => clio::cli::history::run(cli.config.as_deref(), db_path),
        Commands::List {
            ref format,
            preview_length,
            limit,
        } => {
            let conn = db::init_db(&db_path).context("failed to initialize database")?;
            clio::cli::list::run(&conn, format, preview_length, limit)
        }
        Commands::Select { ref source } => {
            let conn = db::init_db(&db_path).context("failed to initialize database")?;
            clio::cli::select::run(&conn, source)
        }
        Commands::Config { ref command } => {
            let config_path = cli
                .config
                .clone()
                .unwrap_or_else(config::default_config_path);
            clio::cli::config::run(&config_path, command)
        }
        // Handled by early return above; unreachable via normal flow.
        Commands::ServeClipboard => clipboard::serve::run().map_err(Into::into),
    }
}
