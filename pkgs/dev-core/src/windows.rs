//! Windows / PowerShell remote support — the Rust port of the bash
//! `_dev_*windows*` / `_ps_*` helpers in `coder.nix`.
//!
//! Windows remotes are reached over the same `ssh` binary as unix remotes, but
//! their command shape differs: a pwsh target wants a `-Command` string, a nu
//! target wants `nu -c`, and every *script* (git summary, agent discovery) is
//! shipped as a base64 `-EncodedCommand` so quoting/newlines survive the trip
//! through cmd/pwsh/nu.
//!
//! Compiled behind the `windows` feature (which enables `config`, so
//! [`crate::config::Env`] and [`crate::ssh`] are available). We add no crates:
//! base64 and UTF-8→UTF-16LE are implemented by hand below.

use crate::config::Env;
use std::process::{Command, Stdio};

// ── low-level encoders (hand-rolled; no `base64` crate) ──────────────────────

/// UTF-8 → UTF-16LE bytes. `str::encode_utf16` already yields UTF-16 code units
/// (splitting supplementary characters into surrogate pairs); we emit each unit
/// little-endian, matching `iconv -f UTF-8 -t UTF-16LE`.
fn utf16le_bytes(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 2);
    for u in s.encode_utf16() {
        out.push((u & 0x00FF) as u8);
        out.push((u >> 8) as u8);
    }
    out
}

/// Standard base64 (RFC 4648, `+/` alphabet, `=` padding), no newlines —
/// equivalent to `base64 | tr -d '\n'`.
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((n >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(n & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Escape a value for embedding inside a PowerShell single-quoted string: a `'`
/// becomes `''`. The caller is responsible for the surrounding quotes. The bash
/// `_ps_single_quote` was a passthrough; this is the intended, safer behaviour.
fn ps_single_quote(s: &str) -> String {
    s.replace('\'', "''")
}

// ── inline remote-command builders (routed to by ssh::remote_command) ────────

/// Non-interactive remote command for a pwsh target.
/// `rp` empty ⇒ no `Set-Location`. Mirrors the pwsh branch of `_dev_exec_on_env`:
///   `pwsh -NoLogo -NonInteractive -Command "Set-Location '<rp>'; <cmd>"`
pub fn pwsh_inline(rp: &str, cmd: &str) -> String {
    let set_loc = if rp.is_empty() {
        String::new()
    } else {
        format!("Set-Location '{rp}'; ")
    };
    format!("pwsh -NoLogo -NonInteractive -Command \"{set_loc}{cmd}\"")
}

/// Non-interactive remote command for a nu target. Mirrors the nu branch of
/// `_dev_exec_on_env`:
///   `nu -c "cd '<rp>'; <cmd>"`  (rp empty ⇒ `nu -c "<cmd>"`)
pub fn nu_inline(rp: &str, cmd: &str) -> String {
    let body = if rp.is_empty() {
        cmd.to_string()
    } else {
        format!("cd '{rp}'; {cmd}")
    };
    format!("nu -c \"{body}\"")
}

/// Encode a PowerShell script as a `-EncodedCommand` argument: UTF-16LE bytes
/// then standard base64. Mirrors `_ps_encoded_command`.
pub fn encoded_command(ps_script: &str) -> String {
    base64_encode(&utf16le_bytes(ps_script))
}

// ── encoded-script execution over ssh (mirrors `_dev_exec_windows_ps`) ───────

/// Run an encoded PowerShell script on `env` over ssh (non-interactive); return
/// trimmed stdout, or `None` on ssh/exec failure.
///
/// We build the ssh invocation directly (`ssh_opts` + `-T` + host + remote
/// command) rather than going through [`crate::ssh::remote_command`], which for
/// a pwsh env would wrap the command in *another* `pwsh -Command` (double-wrap).
/// The remote command tries `pwsh` then falls back to `powershell`; when the
/// env's outer login shell is `nu` we wrap that choice in `nu -c`, otherwise we
/// assume the default OpenSSH shell can evaluate the pwsh/powershell invocation.
pub fn exec_ps(env: &Env, ps_script: &str) -> Option<String> {
    let enc = encoded_command(ps_script);
    let pwsh = format!("pwsh -NoLogo -NoProfile -NonInteractive -EncodedCommand {enc}");
    let powershell = format!("powershell -NoLogo -NoProfile -NonInteractive -EncodedCommand {enc}");
    let remote = if env.shell == "nu" {
        format!("nu -c \"if ((which pwsh | length) > 0) {{ {pwsh} }} else {{ {powershell} }}\"")
    } else {
        format!("if (Get-Command pwsh -ErrorAction SilentlyContinue) {{ {pwsh} }} else {{ {powershell} }}")
    };

    let mut c = Command::new("ssh");
    c.args(crate::ssh::ssh_opts(env, false));
    c.arg("-T");
    c.arg(&env.host);
    c.arg(remote);
    c.stdin(Stdio::null());
    let out = c.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ── higher-level queries (mirror the `_dev_windows_*` helpers) ───────────────

/// Remote git summary over PowerShell → `(branch, head_oneline, changes)`.
/// Mirrors the windows path of `_dev_remote_git_summary`: emit `B:`/`H:`/`C:`
/// prefixed lines from the remote and parse them here.
pub fn git_summary(env: &Env, rp: &str) -> Option<(String, String, i64)> {
    let qrp = ps_single_quote(rp);
    let ps = format!(
        "$ErrorActionPreference = 'SilentlyContinue'
Set-Location -LiteralPath '{qrp}'
Write-Output ('B:' + (& git branch --show-current))
Write-Output ('H:' + (& git log --oneline -1))
$changes = (& git status --short | Measure-Object -Line).Lines
Write-Output ('C:' + $changes)
"
    );
    let out = exec_ps(env, &ps)?;
    let mut branch = String::new();
    let mut head = String::new();
    let mut changes: i64 = 0;
    for line in out.lines() {
        if let Some(b) = line.strip_prefix("B:") {
            branch = b.to_string();
        } else if let Some(h) = line.strip_prefix("H:") {
            head = h.to_string();
        } else if let Some(c) = line.strip_prefix("C:") {
            changes = c.trim().parse().unwrap_or(0);
        }
    }
    Some((branch, head, changes))
}

/// Running codex/opencode/agy processes under `rp` on a Windows remote, as
/// `(tool, pid, cwd_base)` rows. Mirrors `_dev_windows_process_rows`: match
/// tools by executable stem and a normalized (`\`→`/`) command line containing
/// the normalized base path.
pub fn process_rows(env: &Env, rp: &str) -> Vec<(String, String, String)> {
    let qrp = ps_single_quote(rp);
    let ps = format!(
        "$ErrorActionPreference = 'SilentlyContinue'
$base = '{qrp}'
$needle = $base.Replace('\\', '/')
$tools = @('codex','opencode','agy')
Get-CimInstance Win32_Process | ForEach-Object {{
  $tool = [IO.Path]::GetFileNameWithoutExtension([string]$_.Name)
  if ($tools -contains $tool) {{
    $cmd = [string]$_.CommandLine
    $norm = $cmd.Replace('\\', '/')
    if ($norm.Contains($needle)) {{
      Write-Output (('{{0}} {{1}} {{2}}' -f $tool, $_.ProcessId, $base))
    }}
  }}
}}
"
    );
    let Some(out) = exec_ps(env, &ps) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // `<tool> <pid> <base>` — base may contain spaces, so split into 3.
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() == 3 {
            rows.push((
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2].to_string(),
            ));
        }
    }
    rows
}

/// `claude agents --json --cwd <rp>` on a Windows remote → parsed JSON (an array
/// on success, `[]` when claude is absent/empty), or `None` on ssh/parse
/// failure. Mirrors `_dev_windows_claude_agents_json`.
pub fn claude_agents_json(env: &Env, rp: &str) -> Option<serde_json::Value> {
    let qrp = ps_single_quote(rp);
    let ps = format!(
        "$ErrorActionPreference = 'SilentlyContinue'
$json = '[]'
if (Get-Command claude -ErrorAction SilentlyContinue) {{
  $raw = & claude agents --json --cwd '{qrp}' 2>$null
  if ($LASTEXITCODE -eq 0 -and $raw) {{ $json = ($raw -join [Environment]::NewLine) }}
}}
[Console]::Out.WriteLine($json)
"
    );
    let out = exec_ps(env, &ps)?;
    serde_json::from_str(&out).ok()
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_command_known_vector() {
        // printf '%s' 'Write-Output 1' | iconv -f UTF-8 -t UTF-16LE | base64 | tr -d '\n'
        assert_eq!(
            encoded_command("Write-Output 1"),
            "VwByAGkAdABlAC0ATwB1AHQAcAB1AHQAIAAxAA=="
        );
    }

    #[test]
    fn encoded_command_multibyte() {
        // BMP Japanese — 2 bytes/char.
        assert_eq!(encoded_command("こんにちは"), "UzCTMGswYTBvMA==");
        // Non-BMP (surrogate pair) followed by ASCII.
        assert_eq!(encoded_command("𝄞x"), "NNge3XgA");
    }

    #[test]
    fn base64_rfc4648_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn pwsh_inline_shape() {
        assert_eq!(
            pwsh_inline("C:/proj", "git status"),
            "pwsh -NoLogo -NonInteractive -Command \"Set-Location 'C:/proj'; git status\""
        );
        assert_eq!(
            pwsh_inline("", "git status"),
            "pwsh -NoLogo -NonInteractive -Command \"git status\""
        );
    }

    #[test]
    fn nu_inline_shape() {
        assert_eq!(nu_inline("C:/proj", "ls"), "nu -c \"cd 'C:/proj'; ls\"");
        assert_eq!(nu_inline("", "ls"), "nu -c \"ls\"");
    }

    #[test]
    fn ps_single_quote_escapes() {
        assert_eq!(ps_single_quote("plain"), "plain");
        assert_eq!(ps_single_quote("a'b"), "a''b");
        assert_eq!(ps_single_quote("''"), "''''");
    }
}
