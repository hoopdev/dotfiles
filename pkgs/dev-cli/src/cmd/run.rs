//! dev run — execute commands on one or many targets in parallel.

use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;

// ── entry point ───────────────────────────────────────────────────────────────

pub fn run(args: &[String]) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(run_async(args));
}

async fn run_async(args: &[String]) {
    let mut target_arg = String::new();
    let mut cmd_parts: Vec<String> = Vec::new();
    let mut json_out = false;
    let mut timeout_secs: u64 = 120;
    let mut all = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--all" => {
                all = true;
            }
            "--json" => {
                json_out = true;
            }
            "--timeout" => {
                i += 1;
                timeout_secs = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(120);
            }
            a => {
                if target_arg.is_empty() {
                    target_arg = a.to_string();
                } else {
                    cmd_parts.push(a.to_string());
                }
            }
        }
        i += 1;
    }

    if cmd_parts.is_empty() {
        eprintln!("Usage: dev run [--all | <target>] <cmd...> [--json]");
        std::process::exit(1);
    }
    let cmd_str = cmd_parts.join(" ");

    // Resolve target list
    let targets: Vec<String> = if all || target_arg.contains(',') {
        // Fetch all targets from dev targets --json
        let raw = fetch_targets_json().await;
        let all_names: Vec<String> = raw
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        if all {
            all_names
        } else {
            // Comma-separated: filter to matching names
            let requested: std::collections::HashSet<String> =
                target_arg.split(',').map(|s| s.trim().to_string()).collect();
            all_names.into_iter().filter(|n| requested.contains(n)).collect()
        }
    } else {
        vec![target_arg.clone()]
    };

    if targets.is_empty() {
        eprintln!("dev run: no targets resolved");
        std::process::exit(1);
    }

    let timeout = Duration::from_secs(timeout_secs);

    if targets.len() == 1 {
        // Single target — run and output directly
        let result = run_on_target(&targets[0], &cmd_str, timeout).await;
        if json_out {
            println!("{}", serde_json::to_string(&[result]).unwrap());
        } else {
            if !result["stdout"].as_str().unwrap_or("").is_empty() {
                print!("{}", result["stdout"].as_str().unwrap_or(""));
            }
            if !result["stderr"].as_str().unwrap_or("").is_empty() {
                eprint!("{}", result["stderr"].as_str().unwrap_or(""));
            }
            if !result["ok"].as_bool().unwrap_or(true) {
                std::process::exit(result["exit"].as_i64().unwrap_or(1) as i32);
            }
        }
    } else {
        // Multiple targets — parallel fan-out
        let handles: Vec<tokio::task::JoinHandle<Value>> = targets
            .iter()
            .map(|t| {
                let target = t.clone();
                let cmd = cmd_str.clone();
                tokio::spawn(async move { run_on_target(&target, &cmd, timeout).await })
            })
            .collect();

        let mut results = Vec::new();
        for h in handles {
            results.push(
                h.await
                    .unwrap_or_else(|_| serde_json::json!({"ok": false, "error": "task panicked"})),
            );
        }

        if json_out {
            println!("{}", serde_json::to_string(&results).unwrap());
        } else {
            for r in &results {
                let target = r["target"].as_str().unwrap_or("?");
                let ok = r["ok"].as_bool().unwrap_or(false);
                let stdout = r["stdout"].as_str().unwrap_or("");
                let stderr = r["stderr"].as_str().unwrap_or("");
                println!("=== {} {} ===", target, if ok { "✓" } else { "✗" });
                if !stdout.is_empty() {
                    print!("{stdout}");
                }
                if !stderr.is_empty() {
                    eprint!("{stderr}");
                }
            }
        }
    }
}

// ── target resolution ─────────────────────────────────────────────────────────

async fn fetch_targets_json() -> Value {
    let res = Command::new("dev").args(["targets", "--json"]).output().await;
    match res {
        Ok(o) => serde_json::from_slice(&o.stdout).unwrap_or(Value::Array(Vec::new())),
        Err(_) => Value::Array(Vec::new()),
    }
}

// ── single-target execution ───────────────────────────────────────────────────

async fn run_on_target(target: &str, cmd: &str, timeout: Duration) -> Value {
    // Get target info
    let targets = fetch_targets_json().await;
    let info = targets.as_array().and_then(|arr| {
        arr.iter()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some(target))
    });

    match info {
        Some(t) => {
            let kind = t.get("kind").and_then(|k| k.as_str()).unwrap_or("remote");
            match kind {
                "local" => {
                    let path = t.get("path").and_then(|p| p.as_str()).unwrap_or(".");
                    run_local(target, path, cmd, timeout).await
                }
                _ => {
                    let host = t.get("host").and_then(|h| h.as_str()).unwrap_or(target);
                    let path = t.get("path").and_then(|p| p.as_str()).unwrap_or("");
                    let proxy = t.get("proxy").and_then(|p| p.as_str()).unwrap_or("");
                    run_remote(target, host, path, proxy, cmd, timeout).await
                }
            }
        }
        None => {
            // Target not found in targets list — try as local path or fallback subprocess
            run_local(target, ".", cmd, timeout).await
        }
    }
}

async fn run_local(target: &str, path: &str, cmd: &str, timeout: Duration) -> Value {
    let full_cmd = if path == "." {
        cmd.to_string()
    } else {
        format!("cd '{}' && {}", path.replace('\'', "'\\''"), cmd)
    };

    let out =
        tokio::time::timeout(timeout, Command::new("bash").args(["-c", &full_cmd]).output()).await;

    match out {
        Ok(Ok(o)) => serde_json::json!({
            "target": target,
            "ok": o.status.success(),
            "exit": o.status.code().unwrap_or(-1),
            "stdout": String::from_utf8_lossy(&o.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&o.stderr).to_string(),
        }),
        Ok(Err(e)) => serde_json::json!({
            "target": target,
            "ok": false,
            "exit": -1,
            "stdout": "",
            "stderr": e.to_string(),
        }),
        Err(_) => serde_json::json!({
            "target": target,
            "ok": false,
            "exit": -1,
            "stdout": "",
            "stderr": "timeout",
        }),
    }
}

async fn run_remote(
    target: &str,
    host: &str,
    path: &str,
    proxy: &str,
    cmd: &str,
    timeout: Duration,
) -> Value {
    let full_cmd = if path.is_empty() {
        cmd.to_string()
    } else {
        format!("cd '{}' && {}", path.replace('\'', "'\\''"), cmd)
    };

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let control_path = format!("{home}/.ssh/cm-%C");

    let mut ssh_cmd = Command::new("ssh");
    ssh_cmd.args([
        "-o",
        "ControlMaster=auto",
        "-o",
        &format!("ControlPath={control_path}"),
        "-o",
        "ControlPersist=10m",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        "ConnectTimeout=10",
    ]);
    if !proxy.is_empty() {
        ssh_cmd.args(["-o", &format!("ProxyCommand={proxy}")]);
    }
    ssh_cmd.args([host, &full_cmd]);

    let out = tokio::time::timeout(timeout, ssh_cmd.output()).await;

    match out {
        Ok(Ok(o)) => serde_json::json!({
            "target": target,
            "ok": o.status.success(),
            "exit": o.status.code().unwrap_or(-1),
            "stdout": String::from_utf8_lossy(&o.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&o.stderr).to_string(),
        }),
        Ok(Err(e)) => serde_json::json!({
            "target": target,
            "ok": false,
            "exit": -1,
            "stdout": "",
            "stderr": e.to_string(),
        }),
        Err(_) => serde_json::json!({
            "target": target,
            "ok": false,
            "exit": -1,
            "stdout": "",
            "stderr": "timeout",
        }),
    }
}
