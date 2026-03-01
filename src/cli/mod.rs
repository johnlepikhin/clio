pub mod config;
pub mod copy;
#[cfg(feature = "ui")]
pub mod history;
pub mod list;
pub mod select;
pub mod show;
pub mod watch;

use std::path::PathBuf;
use std::time::Duration;

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
    Copy {
        /// Entry time-to-live (e.g. "30s", "5m", "1h")
        #[arg(long, value_parser = parse_duration)]
        ttl: Option<Duration>,
        /// Mask text shown in history UI instead of real content
        #[arg(long)]
        mask_with: Option<String>,
    },
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
    /// List clipboard entries (for dmenu/rofi/wofi integration)
    List {
        /// Output format
        #[arg(long, default_value = "dmenu")]
        format: ListFormat,
        /// Max characters per entry preview
        #[arg(long, default_value_t = 50)]
        preview_length: usize,
        /// Max entries to show
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Select entry by ID and copy to clipboard
    Select {
        #[command(subcommand)]
        source: SelectSource,
    },
    /// Internal: serve clipboard content from stdin (used by spawn_clipboard_server)
    #[command(hide = true, name = "_serve-clipboard")]
    ServeClipboard,
}

#[derive(Clone, clap::ValueEnum)]
pub enum ListFormat {
    Dmenu,
}

#[derive(Subcommand)]
pub enum SelectSource {
    /// Read entry ID from stdin (for piping from dmenu)
    Stdin,
    /// Select entry by numeric ID
    Id {
        /// Entry ID
        id: i64,
    },
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

fn parse_duration(s: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(s)
}
