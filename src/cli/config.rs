use std::path::Path;

use anyhow::Context;

use super::ConfigCommands;

pub fn run(config_path: &Path, command: &ConfigCommands) -> anyhow::Result<()> {
    match command {
        ConfigCommands::Show => cmd_show(config_path),
        ConfigCommands::Init { force, output } => {
            let target = output.as_deref().unwrap_or(config_path);
            cmd_init(target, *force)
        }
        ConfigCommands::Validate => cmd_validate(config_path),
        ConfigCommands::Path => cmd_path(config_path),
    }
}

fn cmd_show(config_path: &Path) -> anyhow::Result<()> {
    let config = crate::config::load_config(Some(config_path))
        .context("failed to load config")?;
    let yaml = serde_yaml::to_string(&config).context("failed to serialize config")?;
    print!("{yaml}");
    Ok(())
}

fn cmd_init(config_path: &Path, force: bool) -> anyhow::Result<()> {
    if config_path.exists() && !force {
        anyhow::bail!(
            "Config file already exists at {}. Use --force to overwrite.",
            config_path.display()
        );
    }
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    std::fs::write(config_path, crate::config::Config::default_yaml())
        .with_context(|| format!("failed to write config to {}", config_path.display()))?;
    println!("Config written to {}", config_path.display());
    Ok(())
}

fn cmd_validate(config_path: &Path) -> anyhow::Result<()> {
    let config = if config_path.exists() {
        crate::config::load_config(Some(config_path)).context("failed to load config")?
    } else {
        println!(
            "No config file found at {}. Using defaults.",
            config_path.display()
        );
        crate::config::Config::default()
    };

    if let Err(errors) = config.validate() {
        anyhow::bail!("invalid configuration:\n  {}", errors.join("\n  "));
    }

    println!("Configuration is valid.");
    Ok(())
}

fn cmd_path(config_path: &Path) -> anyhow::Result<()> {
    println!("{}", config_path.display());
    Ok(())
}
