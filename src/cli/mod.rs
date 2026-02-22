pub mod config;
pub mod copy;
#[cfg(feature = "ui")]
pub mod history;
pub mod show;
pub mod watch;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clio", version, about = "Clipboard manager with history")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show current clipboard content
    Show,
    /// Copy stdin to clipboard
    Copy,
    /// Watch clipboard for changes
    Watch,
    /// Open history window
    #[cfg(feature = "ui")]
    History,
    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Internal: serve clipboard content from stdin (used by spawn_clipboard_server)
    #[command(hide = true, name = "_serve-clipboard")]
    ServeClipboard,
}

/// Configuration management subcommands.
#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current effective configuration
    Show,
    /// Create default configuration file
    Init {
        /// Overwrite existing file
        #[arg(long)]
        force: bool,
        /// Write config to this path instead of the default location
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
    },
    /// Validate configuration file
    Validate,
    /// Print configuration file path
    Path,
}
