//! `dev info` — detailed view of one target (type, host, shell, os, git).

use dev_core::config::{Config, Target};
use dev_core::git;
use serde_json::{json, Value};

fn dash(s: &str) -> String {
    if s.is_empty() {
        "-".to_string()
    } else {
        s.to_string()
    }
}

/// Extract the `{branch,head,changes}` object (or null) from a status row.
fn git_obj(row: &Value) -> Value {
    if row.get("ok").and_then(|v| v.as_bool()) == Some(true)
        && !row.get("branch").map(|v| v.is_null()).unwrap_or(true)
    {
        json!({
            "branch": row.get("branch").cloned().unwrap_or(Value::Null),
            "head": row.get("head").cloned().unwrap_or(Value::Null),
            "changes": row.get("changes").cloned().unwrap_or(Value::Null),
        })
    } else {
        Value::Null
    }
}

fn print_git_human(row: &Value) {
    let g = git_obj(row);
    if g.is_null() {
        println!("GIT     n/a");
        return;
    }
    println!("GIT");
    if let Some(h) = g.get("head").and_then(|v| v.as_str()) {
        println!("  head    {h}");
    }
    if let Some(b) = g.get("branch").and_then(|v| v.as_str()) {
        println!("  branch  {b}");
    }
    if let Some(c) = g.get("changes").and_then(|v| v.as_i64()) {
        println!("  changes {c}");
    }
}

pub fn info(name: Option<String>, json_out: bool) {
    let cfg = Config::load_or_default();
    let Some(name) = name else {
        eprintln!("Usage: dev info <name> [--json]");
        std::process::exit(1);
    };
    let Some(target) = cfg.resolve(&name) else {
        eprintln!("dev: unknown name '{name}'");
        std::process::exit(1);
    };
    match target {
        Target::Local { path, .. } => {
            let row = git::status_for_target(&cfg, &name);
            if json_out {
                println!(
                    "{}",
                    serde_json::to_string(&json!({
                        "name": name, "kind": "local-project", "env": Value::Null,
                        "host": Value::Null, "proxy": Value::Null, "shell": Value::Null,
                        "os": Value::Null, "path": path, "git": git_obj(&row),
                    }))
                    .unwrap()
                );
            } else {
                println!("NAME    {name}");
                println!("TYPE    local project");
                println!("PATH    {path}");
                print_git_human(&row);
            }
        }
        Target::Remote { env, path, .. } => {
            let e = cfg.env(&env);
            let host = e.map(|e| e.host.clone()).unwrap_or_default();
            let proxy = e.map(|e| e.proxy.clone()).unwrap_or_default();
            let shell = e.map(|e| e.shell.clone()).unwrap_or_default();
            let os = cfg.env_os(&env, &path);
            let row = git::status_for_target(&cfg, &name);
            if json_out {
                println!(
                    "{}",
                    serde_json::to_string(&json!({
                        "name": name, "kind": "remote-project", "env": env, "host": host,
                        "proxy": proxy, "shell": shell, "os": os, "path": path,
                        "git": git_obj(&row),
                    }))
                    .unwrap()
                );
            } else {
                println!("NAME    {name}");
                println!("TYPE    remote project");
                println!("ENV     {env}");
                println!("HOST    {host}");
                println!("PROXY   {}", dash(&proxy));
                println!("SHELL   {shell}");
                println!("OS      {os}");
                println!("PATH    {path}");
                print_git_human(&row);
            }
        }
        Target::Env { .. } => {
            let e = cfg.env(&name).unwrap();
            let os = cfg.env_os(&name, "");
            if json_out {
                println!(
                    "{}",
                    serde_json::to_string(&json!({
                        "name": name, "kind": "env", "env": name, "host": e.host,
                        "proxy": e.proxy, "shell": e.shell, "os": os, "path": Value::Null,
                        "git": Value::Null,
                    }))
                    .unwrap()
                );
            } else {
                println!("NAME    {name}");
                println!("TYPE    env");
                println!("HOST    {}", e.host);
                println!("PROXY   {}", dash(&e.proxy));
                println!("SHELL   {}", e.shell);
                println!("OS      {os}");
            }
        }
    }
}
