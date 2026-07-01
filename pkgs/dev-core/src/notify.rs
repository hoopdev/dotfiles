//! Telegram notifications — port of the bash `_dev_notify`.
//!
//! Token comes from `$TELEGRAM_BOT_TOKEN` (or `~/.op-secrets`, sourced by the
//! shell), chat id from `$TELEGRAM_CHAT_ID`. We shell out to `curl` to avoid an
//! HTTP-client dependency, matching the bash behaviour.

use std::process::{Command, Stdio};

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

/// Read `KEY=value` (optionally `export KEY=value`, quoted) from a secrets file.
fn read_secret_file(path: &str, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let line = line.trim().strip_prefix("export ").unwrap_or(line.trim());
        if let Some(rest) = line.strip_prefix(&format!("{key}=")) {
            let v = rest.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn bot_token() -> Option<String> {
    env_nonempty("TELEGRAM_BOT_TOKEN").or_else(|| {
        let home = std::env::var("HOME").ok()?;
        read_secret_file(&format!("{home}/.op-secrets"), "TELEGRAM_BOT_TOKEN")
    })
}

/// Send `msg` to the configured Telegram chat. Returns `false` if unconfigured
/// or the request failed (best-effort, like the bash version).
pub fn send(msg: &str) -> bool {
    let (Some(token), Some(chat)) = (bot_token(), env_nonempty("TELEGRAM_CHAT_ID")) else {
        return false;
    };
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    Command::new("curl")
        .args(["-sS", "-o", "/dev/null", "-X", "POST", &url])
        .args(["--data-urlencode", &format!("chat_id={chat}")])
        .args(["--data-urlencode", &format!("text={msg}")])
        .stdin(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
