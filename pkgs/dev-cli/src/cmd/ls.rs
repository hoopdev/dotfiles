//! `dev ls` / `dev targets` — list the fleet topology from config.toml.

use dev_core::config::Config;
use serde_json::json;

pub fn ls(json_out: bool) {
    let cfg = Config::load_or_default();
    if json_out {
        let v = json!({
            "envs": cfg.envs.iter().map(|e| json!({
                "name": e.name, "host": e.host, "proxy": e.proxy,
                "shell": e.shell, "os": cfg.env_os(&e.name, ""),
            })).collect::<Vec<_>>(),
            "local": cfg.local.iter().map(|l| json!({
                "name": l.name, "path": l.path,
            })).collect::<Vec<_>>(),
            "remote": cfg.remote.iter().map(|r| json!({
                "name": r.name, "env": r.env,
                "host": cfg.env(&r.env).map(|e| e.host.clone()).unwrap_or_default(),
                "shell": cfg.env(&r.env).map(|e| e.shell.clone()).unwrap_or_default(),
                "os": cfg.env_os(&r.env, &r.path), "path": r.path,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string(&v).unwrap());
        return;
    }
    println!("ENVS");
    println!(
        "  {:<20} {:<32} {:<20} {:<6} OS",
        "NAME", "HOST", "PROXY", "SHELL"
    );
    for e in &cfg.envs {
        let proxy = if e.proxy.is_empty() { "-" } else { &e.proxy };
        println!(
            "  {:<20} {:<32} {:<20} {:<6} {}",
            e.name,
            e.host,
            proxy,
            e.shell,
            cfg.env_os(&e.name, "")
        );
    }
    println!("\nLOCAL PROJECTS");
    for l in &cfg.local {
        println!("  {:<20} {}", l.name, l.path);
    }
    println!("\nREMOTE PROJECTS");
    for r in &cfg.remote {
        println!("  {:<20} {:<12} {}", r.name, r.env, r.path);
    }
}

pub fn targets(json_out: bool) {
    let cfg = Config::load_or_default();
    if json_out {
        println!("{}", serde_json::to_string(&cfg.targets_json()).unwrap());
    } else {
        for n in cfg.list_projects().iter().chain(cfg.list_envs().iter()) {
            println!("{n}");
        }
    }
}
