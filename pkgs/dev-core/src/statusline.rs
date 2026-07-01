//! Claude Code statusline — the *write side* of the rate-limit cache. Claude Code
//! pipes a JSON payload to its `statusLine.command` on every response; this module
//! turns that payload into the `~/.cache/claude/usage.json` cache that `dev usage`
//! and the TUI read, and into the `5h:X% 7d:Y%` line printed back onto the bar.
//!
//! This replaces the bash+jq `~/.claude/statusline.sh` (was in `home/mac/dev.nix`).
//! It's a pure, always-on module (serde_json + std only) so the `dev statusline`
//! subcommand and the readers can share the one "expired window → 0%" rule.

use serde_json::{json, Value};
use std::path::PathBuf;

/// `~/.cache/claude/usage.json` (honours `XDG_CACHE_HOME`), the cache both the
/// writer and the readers agree on. `None` only if `$HOME` is unset.
pub fn usage_cache_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let cache = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{home}/.cache"));
    Some(PathBuf::from(cache).join("claude").join("usage.json"))
}

/// Seconds since the Unix epoch (the unit Claude Code uses for `resets_at`).
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

/// A rate-limit window, cloned with `used_percentage` forced to 0 once the window
/// has reset (`resets_at <= now`). The single source of truth for expiry — shared
/// by the writer and, via [`window_pct`], the readers. `None`/non-object → `Null`.
pub fn normalize_window(window: Option<&Value>, now: i64) -> Value {
    let Some(w) = window else {
        return Value::Null;
    };
    let mut w = w.clone();
    if let Some(obj) = w.as_object_mut() {
        let expired = obj
            .get("resets_at")
            .and_then(|v| v.as_i64())
            .is_some_and(|r| r <= now);
        if expired {
            obj.insert("used_percentage".to_string(), Value::from(0));
        }
    }
    w
}

/// The cache object a Claude Code statusline payload distils to:
/// `{updated_at, five_hour, seven_day}`, each window already normalized.
pub fn build_cache(input: &Value, now: i64) -> Value {
    json!({
        "updated_at": now,
        "five_hour": normalize_window(input.pointer("/rate_limits/five_hour"), now),
        "seven_day": normalize_window(input.pointer("/rate_limits/seven_day"), now),
    })
}

/// A window's used-percentage, treating an already-reset window as 0% (so callers
/// need not re-implement expiry). `None` when the window has no percentage at all.
pub fn window_pct(window: Option<&Value>, now: i64) -> Option<f64> {
    let w = window?;
    if w.get("resets_at")
        .and_then(|v| v.as_i64())
        .is_some_and(|r| r <= now)
    {
        return Some(0.0);
    }
    w.get("used_percentage").and_then(|v| v.as_f64())
}

/// The `5h:X% 7d:Y%` bar line for a normalized cache object, or `None` when there
/// is no five-hour figure (matches the old `… // empty` bash behaviour: no data →
/// print nothing). A missing seven-day figure renders as 0, as before.
pub fn status_line(cache: &Value) -> Option<String> {
    let five = cache
        .pointer("/five_hour/used_percentage")
        .and_then(|v| v.as_f64())?;
    let seven = cache
        .pointer("/seven_day/used_percentage")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    Some(format!("5h:{five:.0}% 7d:{seven:.0}%"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(five_reset: i64, five_pct: i64, seven_reset: i64, seven_pct: i64) -> Value {
        json!({
            "rate_limits": {
                "five_hour": { "resets_at": five_reset, "used_percentage": five_pct },
                "seven_day": { "resets_at": seven_reset, "used_percentage": seven_pct },
            }
        })
    }

    #[test]
    fn build_cache_normalizes_expired_window() {
        let now = 1_000_000;
        // five_hour still active (resets in the future), seven_day already reset.
        let input = payload(now + 3600, 42, now - 10, 88);
        let cache = build_cache(&input, now);
        assert_eq!(
            cache.pointer("/five_hour/used_percentage"),
            Some(&json!(42))
        );
        assert_eq!(cache.pointer("/seven_day/used_percentage"), Some(&json!(0)));
        assert_eq!(cache.pointer("/updated_at"), Some(&json!(now)));
    }

    #[test]
    fn status_line_formats_and_rounds() {
        let cache = json!({
            "five_hour": { "used_percentage": 42.6 },
            "seven_day": { "used_percentage": 7.1 },
        });
        assert_eq!(status_line(&cache).as_deref(), Some("5h:43% 7d:7%"));
    }

    #[test]
    fn status_line_missing_five_hour_is_none() {
        // No five-hour data → print nothing (the `// empty` case).
        let cache = json!({ "five_hour": Value::Null, "seven_day": { "used_percentage": 3 } });
        assert_eq!(status_line(&cache), None);
    }

    #[test]
    fn status_line_missing_seven_day_defaults_zero() {
        let cache = json!({ "five_hour": { "used_percentage": 12 } });
        assert_eq!(status_line(&cache).as_deref(), Some("5h:12% 7d:0%"));
    }

    #[test]
    fn window_pct_expired_reads_zero() {
        let now = 500;
        let w = json!({ "resets_at": 400, "used_percentage": 90 });
        assert_eq!(window_pct(Some(&w), now), Some(0.0));
        let w2 = json!({ "resets_at": 600, "used_percentage": 90 });
        assert_eq!(window_pct(Some(&w2), now), Some(90.0));
    }
}
