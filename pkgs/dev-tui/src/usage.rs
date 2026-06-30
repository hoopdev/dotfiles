use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

#[derive(Clone)]
pub struct ClaudeUsage {
    pub five_hour_pct: Option<u32>,
    #[allow(dead_code)]
    pub five_hour_resets_at: Option<i64>,
    pub seven_day_pct: Option<u32>,
    #[allow(dead_code)]
    pub seven_day_resets_at: Option<i64>,
    #[allow(dead_code)]
    pub updated_at: Option<f64>,
}

/// Primary: fetch from the Anthropic OAuth usage API.
/// Falls back to the local usage.json cache on failure.
pub fn fetch_claude_usage() -> Option<ClaudeUsage> {
    if let Some(u) = fetch_via_api() {
        return Some(u);
    }
    fetch_from_file()
}

fn fetch_via_api() -> Option<ClaudeUsage> {
    let token = read_oauth_token()?;
    let version = claude_version();

    let out = Command::new("curl")
        .args([
            "-s",
            "--max-time", "8",
            "-H", &format!("Authorization: Bearer {token}"),
            "-H", "anthropic-beta: oauth-2025-04-20",
            "-H", &format!("User-Agent: claude-code/{version}"),
            "-H", "Content-Type: application/json",
            "https://api.anthropic.com/api/oauth/usage",
        ])
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    let v: Value = serde_json::from_slice(&out.stdout).ok()?;

    // API returns utilization as f64 percentage (0–100); resets_at as ISO 8601 string.
    let five_hour_pct = v.pointer("/five_hour/utilization")
        .and_then(|x| x.as_f64())
        .map(|f| f.round() as u32);

    let seven_day_pct = v.pointer("/seven_day/utilization")
        .and_then(|x| x.as_f64())
        .map(|f| f.round() as u32);

    let five_hour_resets_at = v.pointer("/five_hour/resets_at")
        .and_then(|x| x.as_str())
        .and_then(parse_iso8601);

    let seven_day_resets_at = v.pointer("/seven_day/resets_at")
        .and_then(|x| x.as_str())
        .and_then(parse_iso8601);

    Some(ClaudeUsage {
        five_hour_pct,
        five_hour_resets_at,
        seven_day_pct,
        seven_day_resets_at,
        updated_at: unix_now_secs().map(|s| s as f64),
    })
}

/// Read the OAuth access token.
/// Order: macOS Keychain → ~/.claude/.credentials.json → CLAUDE_CODE_OAUTH_TOKEN env var.
fn read_oauth_token() -> Option<String> {
    // macOS Keychain
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("security")
            .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
            .output()
            .ok()?;
        if out.status.success() {
            let s = String::from_utf8(out.stdout).ok()?;
            if let Some(tok) = parse_credentials_json(s.trim()) {
                return Some(tok);
            }
        }
    }

    // ~/.claude/.credentials.json
    if let Ok(home) = std::env::var("HOME") {
        let path = format!("{home}/.claude/.credentials.json");
        if let Ok(data) = std::fs::read(&path) {
            let s = String::from_utf8(data).unwrap_or_default();
            if let Some(tok) = parse_credentials_json(&s) {
                return Some(tok);
            }
        }
    }

    // Env var fallback (CI / setup-token scenarios)
    std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok()
}

fn parse_credentials_json(s: &str) -> Option<String> {
    let v: Value = serde_json::from_str(s).ok()?;
    let oauth = v.get("claudeAiOauth")?;
    let token = oauth.get("accessToken")?.as_str()?.to_string();
    // Check expiry (epoch ms, 60s buffer)
    if let Some(exp_ms) = oauth.get("expiresAt").and_then(|x| x.as_i64()) {
        let now_ms = unix_now_secs()? * 1000;
        if exp_ms < now_ms + 60_000 {
            return None; // expired or about to expire
        }
    }
    Some(token)
}

fn fetch_from_file() -> Option<ClaudeUsage> {
    let home = std::env::var("HOME").ok()?;
    let cache = std::env::var("XDG_CACHE_HOME").unwrap_or_else(|_| format!("{home}/.cache"));
    let path = format!("{cache}/claude/usage.json");
    let data = std::fs::read(&path).ok()?;
    let mut v: Value = serde_json::from_slice(&data).ok()?;
    let now = unix_now_secs();

    if let Some(now) = now {
        if normalize_usage_cache(&mut v, now) {
            if let Ok(data) = serde_json::to_vec_pretty(&v) {
                let _ = std::fs::write(&path, data);
            }
        }
    }

    Some(ClaudeUsage {
        five_hour_pct: usage_pct(v.get("five_hour"), now),
        five_hour_resets_at: v.pointer("/five_hour/resets_at").and_then(|x| x.as_i64()),
        seven_day_pct: usage_pct(v.get("seven_day"), now),
        seven_day_resets_at: v.pointer("/seven_day/resets_at").and_then(|x| x.as_i64()),
        updated_at: v.get("updated_at").and_then(|x| x.as_f64()),
    })
}

fn unix_now_secs() -> Option<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
}

fn usage_pct(window: Option<&Value>, now: Option<i64>) -> Option<u32> {
    let window = window?;
    if let (Some(reset_at), Some(now)) = (
        window.get("resets_at").and_then(|v| v.as_i64()),
        now,
    ) {
        if reset_at <= now {
            return Some(0);
        }
    }
    window
        .get("used_percentage")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
}

fn normalize_usage_cache(v: &mut Value, now: i64) -> bool {
    let mut changed = false;
    for key in ["five_hour", "seven_day"] {
        let Some(window) = v.get_mut(key).and_then(|v| v.as_object_mut()) else {
            continue;
        };
        let Some(reset_at) = window.get("resets_at").and_then(|v| v.as_i64()) else {
            continue;
        };
        if reset_at > now {
            continue;
        }
        if window.get("used_percentage").and_then(|v| v.as_u64()) != Some(0) {
            window.insert("used_percentage".to_string(), Value::from(0));
            changed = true;
        }
    }
    changed
}

/// Detect the installed claude-code version for the User-Agent header.
fn claude_version() -> String {
    // Try reading from the symlink target path: …/versions/X.Y.Z
    if let Ok(p) = std::fs::read_link("/Users/ktaga/.local/bin/claude") {
        if let Some(v) = p.file_name().and_then(|n| n.to_str()) {
            return v.to_string();
        }
    }
    "2.1.196".to_string()
}

/// Parse an ISO 8601 timestamp like "2026-04-11T07:00:00.528743+00:00" to Unix seconds.
/// Only the date+time portion is needed; sub-seconds and timezone are ignored (UTC assumed).
fn parse_iso8601(s: &str) -> Option<i64> {
    // Take only "YYYY-MM-DDTHH:MM:SS" prefix
    let base = s.get(..19)?;
    let mut it = base.splitn(3, 'T');
    let date = it.next()?;
    let time = it.next().unwrap_or("00:00:00");

    let mut dp = date.splitn(3, '-');
    let y: i64 = dp.next()?.parse().ok()?;
    let mo: i64 = dp.next()?.parse().ok()?;
    let d: i64 = dp.next()?.parse().ok()?;

    let mut tp = time.splitn(3, ':');
    let h: i64 = tp.next()?.parse().ok()?;
    let mi: i64 = tp.next()?.parse().ok()?;
    let sec: i64 = tp.next().unwrap_or("0").parse().ok()?;

    // Days since Unix epoch (1970-01-01) via Gregorian formula
    let days = days_since_epoch(y, mo, d)?;
    Some(days * 86400 + h * 3600 + mi * 60 + sec)
}

fn days_since_epoch(y: i64, m: i64, d: i64) -> Option<i64> {
    if m < 1 || m > 12 || d < 1 || d > 31 {
        return None;
    }
    // Shift months so March = 1, making Feb the last month (handles leap day cleanly)
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let julian_day = era * 146097 + doe + 1721119;
    // JD of 1970-01-01 is 2440588
    Some(julian_day - 2440588)
}

/// A single usage snapshot for history tracking.
#[derive(Clone)]
pub(crate) struct UsageSample {
    pub timestamp: i64,
    pub claude_5h: Option<u32>,
    pub claude_7d: Option<u32>,
    pub codex_5h: Option<u32>,
    pub codex_7d: Option<u32>,
    pub agy_pct: Option<u32>,
}

/// Circular buffer of usage samples for sparkline display.
pub(crate) struct UsageHistory {
    pub samples: Vec<UsageSample>,
    pub max_samples: usize,
}

impl UsageHistory {
    pub(crate) fn new() -> Self {
        Self {
            samples: Vec::new(),
            max_samples: 480, // 24h at 3min intervals
        }
    }

    pub(crate) fn push(&mut self, sample: UsageSample) {
        self.samples.push(sample);
        if self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }
    }

    pub(crate) fn sparkline_data(&self, field: &str) -> Vec<u64> {
        self.samples
            .iter()
            .map(|s| {
                let val = match field {
                    "claude_5h" => s.claude_5h,
                    "claude_7d" => s.claude_7d,
                    "codex_5h" => s.codex_5h,
                    "codex_7d" => s.codex_7d,
                    "agy_pct" => s.agy_pct,
                    _ => None,
                };
                val.unwrap_or(0) as u64
            })
            .collect()
    }

    pub(crate) fn load() -> Self {
        let path = match usage_history_path() {
            Some(p) => p,
            None => return Self::new(),
        };
        let data = match std::fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return Self::new(),
        };
        let mut history = Self::new();
        for line in data.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                history.samples.push(UsageSample {
                    timestamp: v.get("timestamp").and_then(|x| x.as_i64()).unwrap_or(0),
                    claude_5h: v.get("claude_5h").and_then(|x| x.as_u64()).map(|x| x as u32),
                    claude_7d: v.get("claude_7d").and_then(|x| x.as_u64()).map(|x| x as u32),
                    codex_5h: v.get("codex_5h").and_then(|x| x.as_u64()).map(|x| x as u32),
                    codex_7d: v.get("codex_7d").and_then(|x| x.as_u64()).map(|x| x as u32),
                    agy_pct: v.get("agy_pct").and_then(|x| x.as_u64()).map(|x| x as u32),
                });
            }
        }
        // Trim to max_samples (keep most recent)
        if history.samples.len() > history.max_samples {
            let start = history.samples.len() - history.max_samples;
            history.samples = history.samples[start..].to_vec();
        }
        history
    }

    pub(crate) fn save(&self) {
        let path = match usage_history_path() {
            Some(p) => p,
            None => return,
        };
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        use std::io::Write;
        if let Ok(mut f) = std::fs::File::create(&path) {
            for s in &self.samples {
                let mut m = serde_json::Map::new();
                m.insert("timestamp".into(), Value::Number(s.timestamp.into()));
                match s.claude_5h {
                    Some(v) => m.insert("claude_5h".into(), Value::Number(v.into())),
                    None => m.insert("claude_5h".into(), Value::Null),
                };
                match s.claude_7d {
                    Some(v) => m.insert("claude_7d".into(), Value::Number(v.into())),
                    None => m.insert("claude_7d".into(), Value::Null),
                };
                match s.codex_5h {
                    Some(v) => m.insert("codex_5h".into(), Value::Number(v.into())),
                    None => m.insert("codex_5h".into(), Value::Null),
                };
                match s.codex_7d {
                    Some(v) => m.insert("codex_7d".into(), Value::Number(v.into())),
                    None => m.insert("codex_7d".into(), Value::Null),
                };
                match s.agy_pct {
                    Some(v) => m.insert("agy_pct".into(), Value::Number(v.into())),
                    None => m.insert("agy_pct".into(), Value::Null),
                };
                let line = serde_json::to_string(&Value::Object(m)).unwrap_or_default();
                let _ = writeln!(f, "{line}");
            }
        }
    }
}

fn usage_history_path() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let cache = std::env::var("XDG_CACHE_HOME").unwrap_or_else(|_| format!("{home}/.cache"));
    Some(format!("{cache}/dev-tui/usage-history.jsonl"))
}
