//! `dev git status` / `dev git diff` — git across local & remote targets.
//! (worktree/pr land with the agent/task surface in the next phase.)

use dev_core::config::{Config, Target};
use dev_core::{git, ssh};
use serde_json::json;
use std::process::Command;

pub fn status(names: Vec<String>, json_out: bool) {
    let cfg = Config::load_or_default();
    let targets = if names.is_empty() {
        cfg.list_projects()
    } else {
        names
    };
    let rows = git::status_all(&cfg, &targets);
    if json_out {
        println!("{}", serde_json::to_string(&rows).unwrap());
        return;
    }
    for r in &rows {
        let t = r["target"].as_str().unwrap_or("?");
        if r["ok"].as_bool() != Some(true) {
            println!("=== {t} (unreachable) ===");
            continue;
        }
        let branch = r["branch"].as_str().unwrap_or("");
        let head = r["head"].as_str().unwrap_or("");
        println!("=== {t} ===");
        if branch.is_empty() && head.is_empty() {
            println!("  (not a git repo)");
            continue;
        }
        if !head.is_empty() {
            println!("  head    {head}");
        }
        if !branch.is_empty() {
            println!("  branch  {branch}");
        }
        if let Some(c) = r["changes"].as_i64() {
            println!("  changes {c}");
        }
    }
}

fn diff_text(cfg: &Config, target: &Target, args: &[&str]) -> String {
    match target {
        Target::Local { path, .. } => {
            let out = Command::new("git").arg("-C").arg(path).args(args).output();
            match out {
                Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
                Err(_) => String::new(),
            }
        }
        Target::Remote { env, path, .. } => match cfg.env(env) {
            Some(e) => {
                let cmd = format!("git {}", args.join(" "));
                ssh::exec_stdout(e, path, &cmd).unwrap_or_default()
            }
            None => String::new(),
        },
        Target::Env { .. } => String::new(),
    }
}

pub fn diff(name: Option<String>, stat: bool, json_out: bool) {
    let cfg = Config::load_or_default();
    let Some(name) = name else {
        eprintln!("Usage: dev git diff <target> [--stat] [--json]");
        std::process::exit(1);
    };
    let Some(target) = cfg.resolve(&name) else {
        eprintln!("dev git diff: unknown target '{name}'");
        std::process::exit(1);
    };

    if json_out {
        let numstat = diff_text(&cfg, &target, &["diff", "--numstat"]);
        let mut files = Vec::new();
        for line in numstat.lines() {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() == 3 {
                files.push(json!({
                    "added": cols[0].parse::<i64>().ok(),
                    "removed": cols[1].parse::<i64>().ok(),
                    "path": cols[2],
                }));
            }
        }
        println!(
            "{}",
            serde_json::to_string(&json!({ "target": name, "files": files })).unwrap()
        );
        return;
    }

    let mut args = vec!["diff"];
    if stat {
        args.push("--stat");
    }
    print!("{}", diff_text(&cfg, &target, &args));
}

// ── worktree / pr ───────────────────────────────────────────────────────────

/// Run `cmd` at a target's path (local bash / remote ssh), returning (ok, output).
fn run_at(cfg: &Config, target: &Target, cmd: &str) -> (bool, String) {
    match target {
        Target::Local { path, .. } => {
            let out = Command::new("bash")
                .arg("-c")
                .arg(format!("cd {} && {}", ssh::sh_quote(path), cmd))
                .output();
            match out {
                Ok(o) => (
                    o.status.success(),
                    format!(
                        "{}{}",
                        String::from_utf8_lossy(&o.stdout),
                        String::from_utf8_lossy(&o.stderr)
                    ),
                ),
                Err(e) => (false, e.to_string()),
            }
        }
        Target::Remote { env, path, .. } => match cfg.env(env) {
            Some(e) => match ssh::exec_capture(e, path, cmd) {
                Ok(o) => (
                    o.status.success(),
                    format!(
                        "{}{}",
                        String::from_utf8_lossy(&o.stdout),
                        String::from_utf8_lossy(&o.stderr)
                    ),
                ),
                Err(er) => (false, er.to_string()),
            },
            None => (false, "unknown env".to_string()),
        },
        Target::Env { .. } => (false, "not a project".to_string()),
    }
}

fn resolve_project(cfg: &Config, name: &str) -> Target {
    match cfg.resolve(name) {
        Some(t @ (Target::Local { .. } | Target::Remote { .. })) => t,
        _ => {
            eprintln!("dev git: unknown project '{name}'");
            std::process::exit(1);
        }
    }
}

pub fn worktree_ls(name: String) {
    let cfg = Config::load_or_default();
    let t = resolve_project(&cfg, &name);
    let (_, out) = run_at(&cfg, &t, "git worktree list");
    print!("{out}");
}

pub fn worktree_rm(name: String, branch: String) {
    let cfg = Config::load_or_default();
    let t = resolve_project(&cfg, &name);
    let base = match &t {
        Target::Local { path, .. } | Target::Remote { path, .. } => path.clone(),
        Target::Env { .. } => unreachable!(),
    };
    let repo = std::path::Path::new(&base)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let parent = std::path::Path::new(&base)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let san: String = branch
        .chars()
        .map(|c| if c == '/' || c == ' ' { '-' } else { c })
        .collect();
    let wt = format!("{parent}/.dev-worktrees/{repo}-{san}");
    let (ok, out) = run_at(
        &cfg,
        &t,
        &format!("git worktree remove --force {}", ssh::sh_quote(&wt)),
    );
    print!("{out}");
    if !ok {
        std::process::exit(1);
    }
}

/// Push HEAD and open a PR; returns `(ok, extracted_url, combined_output)`. The
/// url is the last `https://` line from the `git push` + `gh pr create` output.
/// Both `dev git pr` and `dev task pr` build on this so PR creation lives in one
/// place instead of `dev task` shelling out to a `dev git pr` self-subprocess.
pub fn pr_capture(
    cfg: &Config,
    name: &str,
    title: Option<&str>,
    base: Option<&str>,
    draft: bool,
) -> (bool, Option<String>, String) {
    let t = resolve_project(cfg, name);
    let mut gh = String::from("gh pr create --fill");
    if let Some(b) = base {
        gh.push_str(&format!(" --base {}", ssh::sh_quote(b)));
    }
    if let Some(ti) = title {
        gh.push_str(&format!(" --title {}", ssh::sh_quote(ti)));
    }
    if draft {
        gh.push_str(" --draft");
    }
    let (ok, out) = run_at(cfg, &t, &format!("git push -u origin HEAD 2>&1; {gh}"));
    let url = out
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with("https://"))
        .map(|l| l.trim().to_string());
    (ok, url, out)
}

pub fn pr(name: String, title: Option<String>, base: Option<String>, draft: bool, json_out: bool) {
    let cfg = Config::load_or_default();
    let (ok, url, out) = pr_capture(&cfg, &name, title.as_deref(), base.as_deref(), draft);
    if json_out {
        println!(
            "{}",
            serde_json::to_string(&json!({ "target": name, "ok": ok, "url": url })).unwrap()
        );
    } else {
        print!("{out}");
    }
    if !ok {
        std::process::exit(1);
    }
}
