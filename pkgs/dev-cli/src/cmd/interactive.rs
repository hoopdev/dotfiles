//! Shared helpers for interactive commands — exec a shell/command/tool on a
//! target (replacing this process) and fzf-based selection for omitted names.

use dev_core::agent::{self, Purpose};
use dev_core::config::{Config, Target};
use dev_core::ssh;
use std::io::IsTerminal;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

/// Resolve which backend to use: an explicit `--backend` wins; else fzf-pick
/// from the registry (filtered by `purpose`) on a TTY; else fall back to
/// `default` silently (keeps scripts / agents / the TUI non-interactive).
pub fn choose_backend(given: Option<&str>, purpose: Purpose, default: &str) -> String {
    if let Some(g) = given {
        return g.to_string();
    }
    if std::io::stdout().is_terminal() {
        let names: Vec<String> = agent::backends_for(purpose)
            .iter()
            .map(|b| b.name.to_string())
            .collect();
        if let Some(sel) = pick(&names, &format!("backend [{default}]> ")) {
            return sel;
        }
    }
    default.to_string()
}

/// Resolve a model override: an explicit `--model` wins; else fzf-pick from the
/// backend's registry on a TTY (Esc/no-TTY ⇒ None = the backend's own default).
pub fn choose_model(given: Option<&str>, backend_name: &str) -> Option<String> {
    if let Some(g) = given {
        return Some(g.to_string());
    }
    if !std::io::stdout().is_terminal() {
        return None;
    }
    let spec = agent::backend(backend_name)?;
    if spec.models.is_empty() {
        return None;
    }
    let labels: Vec<String> = spec
        .models
        .iter()
        .map(|m| {
            if m.id.is_empty() || m.label == m.id {
                m.label.to_string()
            } else {
                format!("{}  ({})", m.label, m.id)
            }
        })
        .collect();
    let sel = pick(&labels, "model> ")?;
    let idx = labels.iter().position(|l| *l == sel).unwrap_or(0);
    let id = spec.models[idx].id;
    (!id.is_empty()).then(|| id.to_string())
}

/// Open an interactive shell (`cmd=None`) or run `cmd` on `target`, replacing
/// this process. Never returns on success.
pub fn exec_on_target(cfg: &Config, target: &Target, cmd: Option<&str>) -> ! {
    match target {
        Target::Local { path, .. } => {
            let err = match cmd {
                Some(c) => Command::new("bash")
                    .arg("-c")
                    .arg(format!("cd {} && exec {}", ssh::sh_quote(path), c))
                    .exec(),
                None => {
                    let _ = std::env::set_current_dir(path);
                    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
                    Command::new(shell).exec()
                }
            };
            eprintln!("dev: {err}");
            std::process::exit(1);
        }
        Target::Remote { env, path, .. } => match cfg.env(env) {
            Some(e) => {
                let err = ssh::exec_interactive(e, path, cmd);
                eprintln!("dev: {err}");
                std::process::exit(1);
            }
            None => {
                eprintln!("dev: unknown env for target");
                std::process::exit(1);
            }
        },
        Target::Env { name } => match cfg.env(name) {
            Some(e) => {
                let err = ssh::exec_interactive(e, "", cmd);
                eprintln!("dev: {err}");
                std::process::exit(1);
            }
            None => {
                eprintln!("dev: unknown env '{name}'");
                std::process::exit(1);
            }
        },
    }
}

/// fzf-pick one of `items`. Returns None if fzf is missing or the user aborted.
pub fn pick(items: &[String], prompt: &str) -> Option<String> {
    fzf_pick(items, prompt)
}

/// fzf-pick a project name from stdin lines. Returns None if fzf is missing or
/// the user aborted.
fn fzf_pick(items: &[String], prompt: &str) -> Option<String> {
    let mut child = Command::new("fzf")
        .args(["--height=60%", "--reverse", "--prompt", prompt])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    {
        use std::io::Write;
        let mut si = child.stdin.take()?;
        let _ = si.write_all(items.join("\n").as_bytes());
    }
    let out = child.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Resolve a name, fzf-picking a project when `name` is empty or `-`.
pub fn require_project(cfg: &Config, name: Option<&str>) -> String {
    match name {
        Some(n) if n != "-" && !n.is_empty() => n.to_string(),
        _ => fzf_pick(&cfg.list_projects(), "dev project> ").unwrap_or_else(|| {
            eprintln!("dev: project name required (fzf unavailable or aborted)");
            std::process::exit(1);
        }),
    }
}

/// Resolve a name across projects + envs, fzf-picking when empty or `-`.
pub fn require_any(cfg: &Config, name: Option<&str>) -> String {
    match name {
        Some(n) if n != "-" && !n.is_empty() => n.to_string(),
        _ => {
            let mut items = cfg.list_projects();
            items.extend(cfg.list_envs());
            fzf_pick(&items, "dev> ").unwrap_or_else(|| {
                eprintln!("dev: name required (fzf unavailable or aborted)");
                std::process::exit(1);
            })
        }
    }
}
