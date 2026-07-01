//! SSH execution — the Rust port of the bash `_dev_exec_on_env`.
//!
//! The option builder ([`ssh_opts`]) and remote-command builder are pure so both
//! the synchronous query path ([`exec_capture`]) and the async fan-out in
//! `dev run` share them. We deliberately shell out to the `ssh` binary (never an
//! SSH library) so `~/.ssh/config`, ProxyCommand and agent forwarding keep
//! working exactly as before.
//!
//! bash/zsh remotes are handled here; pwsh/nu remotes go through
//! [`crate::windows`] (added in the Windows phase) — [`remote_command`] routes
//! to it by shell.

use crate::config::Env;
use std::process::{Command, Output, Stdio};

/// Shell-quote a value for a POSIX `sh -c` string (equivalent to `printf %q`
/// for our inputs). Wrapped in single quotes with `'\''` escaping.
pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The `-o …` options matching the bash `_dev_exec_on_env`. `interactive=false`
/// adds `BatchMode=yes` so non-interactive callers (TUI/fan-out) fail silently
/// instead of writing a password prompt to the tty.
pub fn ssh_opts(env: &Env, interactive: bool) -> Vec<String> {
    let mut o: Vec<String> = vec![
        "-o".into(),
        "StrictHostKeyChecking=accept-new".into(),
        "-o".into(),
        "UserKnownHostsFile=~/.ssh/known_hosts.coder".into(),
        "-o".into(),
        "ControlMaster=auto".into(),
        "-o".into(),
        "ControlPath=~/.ssh/cm-%C".into(),
        "-o".into(),
        "ControlPersist=10m".into(),
    ];
    if !interactive {
        o.push("-o".into());
        o.push("BatchMode=yes".into());
        // Bound non-interactive query calls (ps / git status / info) so one
        // unreachable or hung remote can't block the TUI or fan-out forever.
        // ConnectTimeout caps the connect phase; ServerAlive* drops a session
        // that connects then stops responding (~2×10s). Interactive sessions
        // are deliberately left uncapped — the user is present and can Ctrl-C,
        // and a slow proxy connect shouldn't be severed.
        o.push("-o".into());
        o.push("ConnectTimeout=10".into());
        o.push("-o".into());
        o.push("ServerAliveInterval=10".into());
        o.push("-o".into());
        o.push("ServerAliveCountMax=2".into());
    }
    if env.agent_forward {
        o.push("-o".into());
        o.push("ForwardAgent=yes".into());
    }
    if !env.proxy.is_empty() {
        o.push("-o".into());
        o.push(format!("ProxyCommand={}", env.proxy));
    }
    o
}

/// Build the remote command string for a target's shell.
/// `rp` (remote path) empty ⇒ no `cd`. pwsh/nu route through [`crate::windows`].
pub fn remote_command(shell: &str, rp: &str, cmd: &str) -> String {
    match shell {
        #[cfg(feature = "windows")]
        "pwsh" => crate::windows::pwsh_inline(rp, cmd),
        #[cfg(feature = "windows")]
        "nu" => crate::windows::nu_inline(rp, cmd),
        _ => {
            if rp.is_empty() {
                cmd.to_string()
            } else {
                format!("cd {} && {}", sh_quote(rp), cmd)
            }
        }
    }
}

/// Run `cmd` on `env` non-interactively at `rp`, capturing output.
/// This is the workhorse for the query commands (`info`, `git status`, `ps`).
pub fn exec_capture(env: &Env, rp: &str, cmd: &str) -> std::io::Result<Output> {
    let mut c = Command::new("ssh");
    c.args(ssh_opts(env, false));
    c.arg("-T");
    c.arg(&env.host);
    c.arg(remote_command(&env.shell, rp, cmd));
    c.stdin(Stdio::null());
    c.output()
}

/// Convenience: run `cmd` and return stdout as a `String` (trimmed of a single
/// trailing newline), or `None` on failure / non-zero exit.
pub fn exec_stdout(env: &Env, rp: &str, cmd: &str) -> Option<String> {
    let out = exec_capture(env, rp, cmd).ok()?;
    if !out.status.success() {
        return None;
    }
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    if s.ends_with('\n') {
        s.pop();
    }
    Some(s)
}

/// Interactive remote command string for a login shell (`cmd=None`) or a
/// specific command, per shell type. Mirrors the interactive branches of the
/// bash `_dev_exec_on_env`.
fn interactive_remote(env: &Env, rp: &str, cmd: Option<&str>) -> String {
    if let Some(c) = cmd {
        return if rp.is_empty() {
            c.to_string()
        } else {
            format!("cd {} && {}", sh_quote(rp), c)
        };
    }
    match env.shell.as_str() {
        "pwsh" => {
            if rp.is_empty() {
                "pwsh -NoLogo".to_string()
            } else {
                format!("Set-Location '{rp}'; pwsh -NoLogo")
            }
        }
        "nu" => {
            if rp.is_empty() {
                "nu".to_string()
            } else {
                format!("nu -e \"cd '{rp}'\"")
            }
        }
        other => {
            let shell = if other.is_empty() { "bash" } else { other };
            let cd = if rp.is_empty() {
                String::new()
            } else {
                format!("cd {} && ", sh_quote(rp))
            };
            format!("{cd}exec env ZELLIJ=1 {shell} -l")
        }
    }
}

/// Exec an interactive ssh session on `env` at `rp`, REPLACING the current
/// process (allocates a TTY, no BatchMode). `cmd=None` opens a login shell.
/// Only returns if exec itself fails.
pub fn exec_interactive(env: &Env, rp: &str, cmd: Option<&str>) -> std::io::Error {
    use std::os::unix::process::CommandExt;
    Command::new("ssh")
        .args(ssh_opts(env, true))
        .arg("-t")
        .arg(&env.host)
        .arg(interactive_remote(env, rp, cmd))
        .exec()
}

/// Run `cmd` on `env` at `rp` non-interactively, piping `input` to its stdin.
/// Used to ship the run-registry files to a remote (`cat > …`).
pub fn exec_with_stdin(env: &Env, rp: &str, cmd: &str, input: &[u8]) -> std::io::Result<Output> {
    use std::io::Write;
    let mut child = Command::new("ssh")
        .args(ssh_opts(env, false))
        .arg("-T")
        .arg(&env.host)
        .arg(remote_command(&env.shell, rp, cmd))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut si) = child.stdin.take() {
        si.write_all(input)?;
    }
    child.wait_with_output()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Env;

    fn env() -> Env {
        Env {
            name: "t".into(),
            host: "user@host".into(),
            proxy: String::new(),
            shell: "bash".into(),
            os: String::new(),
            agent_forward: false,
        }
    }

    #[test]
    fn noninteractive_opts_are_bounded() {
        let o = ssh_opts(&env(), false).join(" ");
        assert!(o.contains("BatchMode=yes"));
        assert!(o.contains("ConnectTimeout=10"));
        assert!(o.contains("ServerAliveInterval=10"));
        assert!(o.contains("ServerAliveCountMax=2"));
    }

    #[test]
    fn interactive_opts_are_uncapped() {
        // The user is present for interactive sessions; don't sever a slow proxy.
        let o = ssh_opts(&env(), true).join(" ");
        assert!(!o.contains("BatchMode"));
        assert!(!o.contains("ConnectTimeout"));
        assert!(!o.contains("ServerAlive"));
    }
}
