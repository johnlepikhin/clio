mod ui;

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

#[derive(Parser)]
#[command(name = "clio-history", version, about = "GTK4 history viewer for clio")]
struct Args {
    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Path to database file
    #[arg(long)]
    db_path: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    // Skip Vulkan/NGL GPU initialization — Cairo is sufficient for a simple list UI
    // and avoids ~2s of shader compilation + compositor round-trips.
    // Disable AT-SPI accessibility bus — saves ~600ms of D-Bus setup.
    // Users can override by setting these env vars before launching clio.
    //
    // SAFETY: called before any threads are spawned (pre-GTK init, pre-env_logger).
    unsafe {
        if std::env::var_os("GSK_RENDERER").is_none() {
            std::env::set_var("GSK_RENDERER", "cairo");
        }
        if std::env::var_os("GTK_A11Y").is_none() {
            std::env::set_var("GTK_A11Y", "none");
        }
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();

    let args = Args::parse();
    let config =
        clio::config::load_config(args.config.as_deref()).context("failed to load config")?;

    let db_path = args
        .db_path
        .unwrap_or_else(|| clio::config::resolve_db_path(&config));

    ui::run_history_window(&config, db_path)
}
