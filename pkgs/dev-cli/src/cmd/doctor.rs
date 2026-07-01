//! `dev doctor` — validate tools, config, and (with --connect) reachability.

use dev_core::config::{Config, Target};
use dev_core::ssh;
use std::process::Command;

fn have(tool: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {tool} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn doctor(connect: bool, names: Vec<String>) {
    let cfg = Config::load_or_default();
    let mut failures = 0usize;
    let mut warnings = 0usize;

    println!("TOOLS");
    for t in ["ssh", "git"] {
        if have(t) {
            println!("ok    {t}");
        } else {
            println!("fail  {t} not found");
            failures += 1;
        }
    }
    for (t, why) in [
        ("fzf", "omitted-name selection is unavailable"),
        ("code", "VS Code 'code' command not found"),
    ] {
        if have(t) {
            println!("ok    {t}");
        } else {
            println!("warn  {t} not found; {why}");
            warnings += 1;
        }
    }

    println!("\nCONFIG");
    for (label, n) in [
        ("envs", cfg.envs.len()),
        ("local", cfg.local.len()),
        ("remote", cfg.remote.len()),
    ] {
        if n > 0 {
            println!("ok    {label}: {n}");
        } else {
            println!("warn  {label} is empty");
            warnings += 1;
        }
    }
    for l in &cfg.local {
        if std::path::Path::new(&l.path).is_dir() {
            println!("ok    local {} path exists", l.name);
        } else {
            println!("fail  local {} path missing: {}", l.name, l.path);
            failures += 1;
        }
    }
    for r in &cfg.remote {
        if cfg.env(&r.env).is_some() {
            println!("ok    remote {} uses env {}", r.name, r.env);
        } else {
            println!("fail  remote {} references unknown env {}", r.name, r.env);
            failures += 1;
        }
    }

    println!("\nCONNECTIVITY");
    if connect {
        let checklist: Vec<String> = if names.is_empty() {
            cfg.list_envs()
                .into_iter()
                .chain(cfg.list_projects())
                .collect()
        } else {
            names
        };
        for n in &checklist {
            match cfg.resolve(n) {
                Some(Target::Local { path, .. }) => {
                    if std::path::Path::new(&path).is_dir() {
                        println!("ok    {n} local path reachable");
                    } else {
                        println!("fail  {n} local path missing");
                        failures += 1;
                    }
                }
                Some(Target::Remote { env, path, .. }) => {
                    let ok = cfg
                        .env(&env)
                        .and_then(|e| {
                            ssh::exec_capture(e, "", &format!("test -d {}", ssh::sh_quote(&path)))
                                .ok()
                        })
                        .map(|o| o.status.success())
                        .unwrap_or(false);
                    if ok {
                        println!("ok    {n} remote path reachable");
                    } else {
                        println!("fail  {n} remote path unreachable");
                        failures += 1;
                    }
                }
                Some(Target::Env { .. }) => {
                    let ok = cfg
                        .env(n)
                        .and_then(|e| ssh::exec_capture(e, "", "true").ok())
                        .map(|o| o.status.success())
                        .unwrap_or(false);
                    if ok {
                        println!("ok    {n} ssh reachable");
                    } else {
                        println!("fail  {n} ssh unreachable");
                        failures += 1;
                    }
                }
                None => {
                    println!("fail  unknown name for connectivity check: {n}");
                    failures += 1;
                }
            }
        }
    } else {
        println!("warn  skipped; run 'dev doctor --connect' to check SSH and remote paths");
    }

    println!("\nSUMMARY failures={failures} warnings={warnings}");
    if failures > 0 {
        std::process::exit(1);
    }
}
