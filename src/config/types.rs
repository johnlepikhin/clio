use std::fmt;
use std::time::Duration;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMode {
    ToClipboard,
    ToPrimary,
    Both,
    Disabled,
}

impl Default for SyncMode {
    fn default() -> Self {
        Self::Both
    }
}

impl fmt::Display for SyncMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ToClipboard => write!(f, "to-clipboard"),
            Self::ToPrimary => write!(f, "to-primary"),
            Self::Both => write!(f, "both"),
            Self::Disabled => write!(f, "disabled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConditions {
    pub source_app: Option<String>,
    pub content_regex: Option<String>,
    pub source_title_regex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleActions {
    #[serde(with = "humantime_serde::option", default)]
    pub ttl: Option<Duration>,
    pub command: Option<Vec<String>>,
    #[serde(with = "humantime_serde::option", default)]
    pub command_timeout: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRule {
    pub name: String,
    pub conditions: RuleConditions,
    pub actions: RuleActions,
}

/// Validated version of ActionRule with compiled regex.
#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub name: String,
    pub source_app: Option<String>,
    pub content_regex: Option<Regex>,
    pub source_title_regex: Option<Regex>,
    pub ttl: Option<Duration>,
    pub command: Option<Vec<String>>,
    pub command_timeout: Duration,
}

const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

const DEFAULT_PRUNE_INTERVAL: Duration = Duration::from_secs(3);

fn default_prune_interval() -> Duration {
    DEFAULT_PRUNE_INTERVAL
}

impl ActionRule {
    /// Validate and compile this rule. Returns error messages for invalid rules.
    pub fn compile(&self) -> Result<CompiledRule, String> {
        if self.conditions.source_app.is_none()
            && self.conditions.content_regex.is_none()
            && self.conditions.source_title_regex.is_none()
        {
            return Err(format!(
                "rule '{}': at least one condition (source_app, content_regex, or source_title_regex) is required",
                self.name
            ));
        }

        let content_regex = match &self.conditions.content_regex {
            Some(pattern) => match Regex::new(pattern) {
                Ok(re) => Some(re),
                Err(e) => {
                    return Err(format!("rule '{}': invalid regex '{}': {}", self.name, pattern, e));
                }
            },
            None => None,
        };

        let source_title_regex = match &self.conditions.source_title_regex {
            Some(pattern) => match Regex::new(pattern) {
                Ok(re) => Some(re),
                Err(e) => {
                    return Err(format!(
                        "rule '{}': invalid source_title_regex '{}': {}",
                        self.name, pattern, e
                    ));
                }
            },
            None => None,
        };

        if let Some(ref cmd) = self.actions.command {
            if cmd.is_empty() {
                return Err(format!("rule '{}': command must not be empty", self.name));
            }
        }

        Ok(CompiledRule {
            name: self.name.clone(),
            source_app: self.conditions.source_app.clone(),
            content_regex,
            source_title_regex,
            ttl: self.actions.ttl,
            command: self.actions.command.clone(),
            command_timeout: self.actions.command_timeout.unwrap_or(DEFAULT_COMMAND_TIMEOUT),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub max_history: usize,
    pub watch_interval_ms: u64,
    pub db_path: Option<String>,
    pub max_entry_size_kb: u64,
    pub window_width: i32,
    pub window_height: i32,
    pub sync_mode: SyncMode,
    pub preview_text_chars: usize,
    pub history_page_size: usize,
    pub image_preview_max_px: i32,
    #[serde(with = "humantime_serde::option", default)]
    pub max_age: Option<Duration>,
    #[serde(with = "humantime_serde", default = "default_prune_interval")]
    pub prune_interval: Duration,
    #[serde(default)]
    pub actions: Vec<ActionRule>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_history: 500,
            watch_interval_ms: 500,
            db_path: None,
            max_entry_size_kb: 51200,
            window_width: 600,
            window_height: 400,
            sync_mode: SyncMode::default(),
            preview_text_chars: 4096,
            history_page_size: 50,
            image_preview_max_px: 640,
            max_age: None,
            prune_interval: DEFAULT_PRUNE_INTERVAL,
            actions: Vec::new(),
        }
    }
}

impl Config {
    /// Generate a default configuration file with explanatory comments.
    #[must_use]
    pub fn default_yaml() -> String {
        r#"# Clio clipboard manager configuration
# See `clio config show` for current effective values.

# Maximum number of clipboard entries to keep in history.
max_history: 500

# Polling interval in milliseconds for `clio watch`.
watch_interval_ms: 500

# Custom database path (omit to use XDG default).
# db_path: /path/to/custom.db

# Maximum clipboard entry size in kilobytes (default 50 MB).
max_entry_size_kb: 51200

# GTK history window dimensions.
window_width: 600
window_height: 400

# Synchronization between PRIMARY selection (mouse) and CLIPBOARD (Ctrl+C/V).
# Values: both (default), to-clipboard, to-primary, disabled
sync_mode: both

# Maximum characters of text shown in history list preview (default 4096).
preview_text_chars: 4096

# Number of entries loaded per page in the history window (default 50).
history_page_size: 50

# Maximum image preview size in pixels (longest side, default 640).
image_preview_max_px: 640

# Delete entries older than this duration (e.g. 30d, 12h, 90m).
# Omit or leave empty to keep entries forever.
# max_age: 30d

# How often to prune expired entries during `clio watch` (e.g. 3s, 1m).
prune_interval: 3s

# Action rules: conditions → actions applied to matching clipboard entries.
# actions:
#   - name: "Expire passwords quickly"
#     conditions:
#       source_app: "KeePassXC"
#     actions:
#       ttl: "30s"
#
#   - name: "Expire API keys"
#     conditions:
#       content_regex: "^(sk-|ghp_|gho_|ghs_|AKIA|xox[bpas]-|glpat-)[A-Za-z0-9_\\-]+"
#     actions:
#       ttl: "1m"
#
#   - name: "Expire private keys & certificates"
#     conditions:
#       content_regex: "^-----BEGIN (RSA |EC |OPENSSH |PGP )?PRIVATE KEY-----"
#     actions:
#       ttl: "30s"
#
#   - name: "Expire connection strings"
#     conditions:
#       content_regex: "^(postgres|mysql|mongodb|redis)://[^\\s]*@"
#     actions:
#       ttl: "2m"
#
#   - name: "Expire env secrets"
#     conditions:
#       content_regex: "(?i)(PASSWORD|SECRET|TOKEN|API_KEY)\\s*[=:]\\s*\\S+"
#     actions:
#       ttl: "2m"
#
#   - name: "Strip tracking params"
#     conditions:
#       content_regex: "^https?://.*[?&](utm_|fbclid|gclid|msclkid|yclid|_ga|_gl|mc_eid|igshid|ref_)"
#     actions:
#       command: ["sed", "s/[?&]\\(utm_[^&]*\\|fbclid=[^&]*\\|gclid=[^&]*\\|msclkid=[^&]*\\|yclid=[^&]*\\|_ga=[^&]*\\|_gl=[^&]*\\|mc_eid=[^&]*\\|igshid=[^&]*\\|ref_=[^&]*\\)//g"]
#       command_timeout: "5s"
#
#   - name: "Clean trailing whitespace"
#     conditions:
#       content_regex: "[ \\t]+$"
#     actions:
#       command: ["sed", "s/[[:space:]]*$//"]
#       command_timeout: "5s"
#
#   - name: "Expire banking site copies"
#     conditions:
#       source_title_regex: "(?i)(bank|banking|chase\\.com|wells\\s*fargo)"
#     actions:
#       ttl: "1m"
"#
        .to_owned()
    }

    /// Validate configuration values.
    /// Returns `Ok(())` if valid, or `Err` with a list of error messages.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.max_history == 0 {
            errors.push("max_history must be greater than 0".to_owned());
        }
        if self.watch_interval_ms == 0 {
            errors.push("watch_interval_ms must be greater than 0".to_owned());
        }
        if self.max_entry_size_kb == 0 {
            errors.push("max_entry_size_kb must be greater than 0".to_owned());
        }
        if self.window_width <= 0 {
            errors.push("window_width must be greater than 0".to_owned());
        }
        if self.window_height <= 0 {
            errors.push("window_height must be greater than 0".to_owned());
        }
        if self.preview_text_chars == 0 {
            errors.push("preview_text_chars must be greater than 0".to_owned());
        }
        if self.history_page_size == 0 {
            errors.push("history_page_size must be greater than 0".to_owned());
        }
        if self.image_preview_max_px <= 0 {
            errors.push("image_preview_max_px must be greater than 0".to_owned());
        }
        if self.prune_interval.is_zero() {
            errors.push("prune_interval must be greater than 0".to_owned());
        }

        for rule in &self.actions {
            if let Err(e) = rule.compile() {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Compile all action rules into ready-to-use form.
    /// Invalid rules are skipped with warnings printed to stderr.
    pub fn compile_rules(&self) -> Vec<CompiledRule> {
        let mut compiled = Vec::new();
        for rule in &self.actions {
            match rule.compile() {
                Ok(r) => {
                    if r.ttl.is_none() && r.command.is_none() {
                        eprintln!(
                            "warning: skipping rule '{}': no actions (no ttl or command)",
                            r.name
                        );
                        continue;
                    }
                    compiled.push(r);
                }
                Err(e) => eprintln!("warning: skipping action rule: {e}"),
            }
        }
        compiled
    }
}
