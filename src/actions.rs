use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use chrono::Utc;

use crate::config::CompiledRule;
use crate::models::entry::{ClipboardEntry, EntryContent, TIMESTAMP_FORMAT};

/// Maximum bytes to read from command stdout (50 MB safety limit).
const MAX_COMMAND_OUTPUT: u64 = 50 * 1024 * 1024;

/// Result of applying action rules to a clipboard entry.
pub struct ActionResult {
    /// Transformed text content (None if unchanged or image entry).
    pub transformed_text: Option<String>,
    /// Per-entry expiration timestamp (ISO 8601).
    pub expires_at: Option<String>,
    /// Original TTL duration from the matching rule (for expiry tracking).
    pub ttl: Option<Duration>,
}

/// Evaluate all rules against an entry and apply matching actions.
/// Rules are applied in definition order. For TTL, last match wins.
/// For commands, they chain sequentially.
pub fn apply_rules(rules: &[CompiledRule], entry: &ClipboardEntry) -> ActionResult {
    let text = entry.content.text();
    let source_app = entry.source_app.as_deref();
    let source_title = entry.source_title.as_deref();
    let is_image = matches!(entry.content, EntryContent::Image(_));

    let mut ttl: Option<Duration> = None;
    let mut current_text: Option<String> = text.map(|t| t.to_owned());

    for rule in rules {
        if !rule_matches(rule, source_app, current_text.as_deref(), is_image, source_title) {
            continue;
        }

        // Apply TTL action (last match wins)
        if let Some(rule_ttl) = rule.ttl {
            ttl = Some(rule_ttl);
        }

        // Apply command action (only for text entries)
        if let Some(ref cmd) = rule.command {
            if let Some(ref input) = current_text {
                match run_command(cmd, input, rule.command_timeout) {
                    Ok(output) => current_text = Some(output),
                    Err(e) => {
                        eprintln!(
                            "warning: command failed for rule '{}': {e}; keeping original text",
                            rule.name
                        );
                    }
                }
            }
        }
    }

    let transformed_text = match (text, &current_text) {
        (Some(original), Some(transformed)) if original != transformed => {
            Some(transformed.clone())
        }
        _ => None,
    };

    let expires_at = ttl.map(|d| {
        let chrono_d = match chrono::Duration::from_std(d) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("warning: TTL duration {d:?} too large, entry will not expire");
                chrono::Duration::MAX
            }
        };
        let expires = Utc::now() + chrono_d;
        expires.format(TIMESTAMP_FORMAT).to_string()
    });

    ActionResult {
        transformed_text,
        expires_at,
        ttl,
    }
}

fn rule_matches(
    rule: &CompiledRule,
    source_app: Option<&str>,
    text: Option<&str>,
    is_image: bool,
    source_title: Option<&str>,
) -> bool {
    // Check source_app condition
    if let Some(ref expected) = rule.source_app {
        match source_app {
            Some(actual) if actual == expected => {}
            _ => return false,
        }
    }

    // Check content_regex condition (never matches images)
    if let Some(ref regex) = rule.content_regex {
        if is_image {
            return false;
        }
        match text {
            Some(t) if regex.is_match(t) => {}
            _ => return false,
        }
    }

    // Check source_title_regex condition (works for both text and images)
    if let Some(ref regex) = rule.source_title_regex {
        match source_title {
            Some(t) if regex.is_match(t) => {}
            _ => return false,
        }
    }

    true
}

fn run_command(cmd: &[String], input: &str, timeout: Duration) -> Result<String, String> {
    let mut child = Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn '{}': {e}", cmd[0]))?;

    // Write stdin in a separate thread to avoid deadlock:
    // if stdout buffer fills before stdin is fully written, both sides block.
    let stdin_handle = child.stdin.take();
    let input_owned = input.to_owned();
    let writer = std::thread::spawn(move || -> Result<(), String> {
        if let Some(mut stdin) = stdin_handle {
            stdin
                .write_all(input_owned.as_bytes())
                .map_err(|e| format!("failed to write to stdin: {e}"))?;
        }
        Ok(())
    });

    let output = match wait_with_timeout(&mut child, timeout) {
        Ok(output) => output,
        Err(e) => {
            let _ = child.kill();
            // Ensure writer thread finishes after kill
            let _ = writer.join();
            return Err(e);
        }
    };

    // Check if stdin write failed
    match writer.join() {
        Ok(result) => result?,
        Err(_) => return Err("stdin writer thread panicked".to_owned()),
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "command exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("command output is not valid UTF-8: {e}"))
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = child
                    .stdout
                    .take()
                    .map(|s| {
                        let mut buf = Vec::new();
                        s.take(MAX_COMMAND_OUTPUT).read_to_end(&mut buf).ok();
                        buf
                    })
                    .unwrap_or_default();
                let stderr = child
                    .stderr
                    .take()
                    .map(|s| {
                        let mut buf = Vec::new();
                        s.take(1024 * 1024).read_to_end(&mut buf).ok();
                        buf
                    })
                    .unwrap_or_default();
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    return Err(format!("command timed out after {}s", timeout.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(format!("failed to wait for command: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ActionRule, RuleActions, RuleConditions};

    fn compile_rule(rule: &ActionRule) -> CompiledRule {
        rule.compile().unwrap()
    }

    fn text_entry(text: &str, source_app: Option<&str>) -> ClipboardEntry {
        ClipboardEntry::from_text(text.to_owned(), source_app.map(|s| s.to_owned()))
    }

    #[test]
    fn test_no_rules_no_effect() {
        let entry = text_entry("hello", None);
        let result = apply_rules(&[], &entry);
        assert!(result.transformed_text.is_none());
        assert!(result.expires_at.is_none());
        assert!(result.ttl.is_none());
    }

    #[test]
    fn test_source_app_match_ttl() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: Some("KeePassXC".into()),
                content_regex: None,
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(30)),
                command: None,
                command_timeout: None,
            },
        });

        let entry = text_entry("password123", Some("KeePassXC"));
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_some());
        assert_eq!(result.ttl, Some(Duration::from_secs(30)));
        assert!(result.transformed_text.is_none());
    }

    #[test]
    fn test_source_app_no_match() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: Some("KeePassXC".into()),
                content_regex: None,
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(30)),
                command: None,
                command_timeout: None,
            },
        });

        let entry = text_entry("hello", Some("Firefox"));
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_none());
    }

    #[test]
    fn test_content_regex_match_ttl() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: Some("^sk-".into()),
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(60)),
                command: None,
                command_timeout: None,
            },
        });

        let entry = text_entry("sk-abc123xyz", None);
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_some());
    }

    #[test]
    fn test_content_regex_no_match() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: Some("^sk-".into()),
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(60)),
                command: None,
                command_timeout: None,
            },
        });

        let entry = text_entry("Hello world", None);
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_none());
    }

    #[test]
    fn test_and_logic_both_match() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: Some("Firefox".into()),
                content_regex: Some("^password:".into()),
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(15)),
                command: None,
                command_timeout: None,
            },
        });

        let entry = text_entry("password:secret", Some("Firefox"));
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_some());
    }

    #[test]
    fn test_and_logic_partial_match() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: Some("Firefox".into()),
                content_regex: Some("^password:".into()),
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(15)),
                command: None,
                command_timeout: None,
            },
        });

        // source_app matches but regex doesn't
        let entry = text_entry("hello", Some("Firefox"));
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_none());
    }

    #[test]
    fn test_command_transforms_text() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: Some(".*".into()),
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: None,
                command: Some(vec!["tr".into(), "a-z".into(), "A-Z".into()]),
                command_timeout: None,
            },
        });

        let entry = text_entry("hello", None);
        let result = apply_rules(&[rule], &entry);
        assert_eq!(result.transformed_text.as_deref(), Some("HELLO"));
    }

    #[test]
    fn test_command_failure_preserves_original() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: Some(".*".into()),
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: None,
                command: Some(vec!["false".into()]),
                command_timeout: None,
            },
        });

        let entry = text_entry("hello", None);
        let result = apply_rules(&[rule], &entry);
        assert!(result.transformed_text.is_none());
    }

    #[test]
    fn test_missing_binary_preserves_original() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: Some(".*".into()),
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: None,
                command: Some(vec!["nonexistent_binary_xyz".into()]),
                command_timeout: None,
            },
        });

        let entry = text_entry("hello", None);
        let result = apply_rules(&[rule], &entry);
        assert!(result.transformed_text.is_none());
    }

    #[test]
    fn test_last_ttl_wins() {
        let rules: Vec<CompiledRule> = vec![
            compile_rule(&ActionRule {
                name: "rule1".into(),
                conditions: RuleConditions {
                    source_app: None,
                    content_regex: Some(".*".into()),
                    source_title_regex: None,
                },
                actions: RuleActions {
                    ttl: Some(Duration::from_secs(30)),
                    command: None,
                    command_timeout: None,
                },
            }),
            compile_rule(&ActionRule {
                name: "rule2".into(),
                conditions: RuleConditions {
                    source_app: None,
                    content_regex: Some(".*".into()),
                    source_title_regex: None,
                },
                actions: RuleActions {
                    ttl: Some(Duration::from_secs(60)),
                    command: None,
                    command_timeout: None,
                },
            }),
        ];

        let entry = text_entry("hello", None);
        let before = Utc::now();
        let result = apply_rules(&rules, &entry);
        // Both matched, last TTL (60s) wins
        let expires = result.expires_at.unwrap();
        let parsed = chrono::NaiveDateTime::parse_from_str(&expires, TIMESTAMP_FORMAT)
            .unwrap_or_else(|e| panic!("failed to parse '{expires}': {e}"));
        let diff = parsed - before.naive_utc();
        assert!(
            diff.num_seconds() >= 55 && diff.num_seconds() <= 65,
            "expected ~60s TTL, got {}s",
            diff.num_seconds()
        );
    }

    #[test]
    fn test_commands_chain() {
        let rules: Vec<CompiledRule> = vec![
            compile_rule(&ActionRule {
                name: "upper".into(),
                conditions: RuleConditions {
                    source_app: None,
                    content_regex: Some(".*".into()),
                    source_title_regex: None,
                },
                actions: RuleActions {
                    ttl: None,
                    command: Some(vec!["tr".into(), "a-z".into(), "A-Z".into()]),
                    command_timeout: None,
                },
            }),
            compile_rule(&ActionRule {
                name: "reverse".into(),
                conditions: RuleConditions {
                    source_app: None,
                    content_regex: Some(".*".into()),
                    source_title_regex: None,
                },
                actions: RuleActions {
                    ttl: None,
                    command: Some(vec!["rev".into()]),
                    command_timeout: None,
                },
            }),
        ];

        let entry = text_entry("hello", None);
        let result = apply_rules(&rules, &entry);
        assert_eq!(result.transformed_text.as_deref(), Some("OLLEH"));
    }

    #[test]
    fn test_source_app_none_skips_source_app_rules() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: Some("KeePassXC".into()),
                content_regex: None,
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(30)),
                command: None,
                command_timeout: None,
            },
        });

        let entry = text_entry("password", None);
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_none());
    }

    #[test]
    fn test_image_entry_skips_content_regex() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: Some(".*".into()),
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(30)),
                command: None,
                command_timeout: None,
            },
        });

        let rgba = vec![255u8; 4 * 2 * 2];
        let entry = ClipboardEntry::from_image(2, 2, rgba, None).unwrap();
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_none());
    }

    #[test]
    fn test_image_entry_matches_source_app_only() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: Some("GIMP".into()),
                content_regex: None,
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(30)),
                command: None,
                command_timeout: None,
            },
        });

        let rgba = vec![255u8; 4 * 2 * 2];
        let entry = ClipboardEntry::from_image(2, 2, rgba, Some("GIMP".into())).unwrap();
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_some());
        // Command should not apply to images (but here there's no command, just TTL)
        assert!(result.transformed_text.is_none());
    }

    #[test]
    fn test_rule_validation_no_conditions() {
        let rule = ActionRule {
            name: "bad".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: None,
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: None,
                command: None,
                command_timeout: None,
            },
        };
        assert!(rule.compile().is_err());
    }

    #[test]
    fn test_rule_validation_invalid_regex() {
        let rule = ActionRule {
            name: "bad".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: Some("[invalid".into()),
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: None,
                command: None,
                command_timeout: None,
            },
        };
        assert!(rule.compile().is_err());
    }

    #[test]
    fn test_rule_validation_empty_command() {
        let rule = ActionRule {
            name: "bad".into(),
            conditions: RuleConditions {
                source_app: Some("App".into()),
                content_regex: None,
                source_title_regex: None,
            },
            actions: RuleActions {
                ttl: None,
                command: Some(vec![]),
                command_timeout: None,
            },
        };
        assert!(rule.compile().is_err());
    }

    #[test]
    fn test_source_title_regex_match() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: None,
                source_title_regex: Some("KeePass".into()),
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(30)),
                command: None,
                command_timeout: None,
            },
        });

        let mut entry = text_entry("password123", None);
        entry.source_title = Some("KeePassXC – Passwords".into());
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_some());
        assert_eq!(result.ttl, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_source_title_regex_no_match() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: None,
                source_title_regex: Some("KeePass".into()),
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(30)),
                command: None,
                command_timeout: None,
            },
        });

        let mut entry = text_entry("hello", None);
        entry.source_title = Some("Mozilla Firefox".into());
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_none());
    }

    #[test]
    fn test_source_title_none_skips() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: None,
                source_title_regex: Some("KeePass".into()),
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(30)),
                command: None,
                command_timeout: None,
            },
        });

        let entry = text_entry("password", None);
        assert!(entry.source_title.is_none());
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_none());
    }

    #[test]
    fn test_source_title_images_match() {
        let rule = compile_rule(&ActionRule {
            name: "test".into(),
            conditions: RuleConditions {
                source_app: None,
                content_regex: None,
                source_title_regex: Some("GIMP".into()),
            },
            actions: RuleActions {
                ttl: Some(Duration::from_secs(60)),
                command: None,
                command_timeout: None,
            },
        });

        let rgba = vec![255u8; 4 * 2 * 2];
        let mut entry = ClipboardEntry::from_image(2, 2, rgba, None).unwrap();
        entry.source_title = Some("GIMP – image.png".into());
        let result = apply_rules(&[rule], &entry);
        assert!(result.expires_at.is_some());
        assert_eq!(result.ttl, Some(Duration::from_secs(60)));
    }
}
