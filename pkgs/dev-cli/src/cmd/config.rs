//! `dev config` — manage `~/.config/dev/config.toml`, the single source of
//! truth for the fleet topology (see `dev_core::config`).

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use dev_core::config::{self, Config};
use std::io::Read;

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    /// Print the resolved config (TOML by default, or `--json`).
    Show,
    /// Validate reference integrity and local path existence.
    Validate,
    /// Import DEV_* entries from stdin (tagged lines) into config.toml.
    ///
    /// Feed it from your live zsh arrays:
    ///   { for e in $DEV_ENVS; print "env $e";
    ///     for e in $DEV_LOCAL; print "local $e";
    ///     for e in $DEV_REMOTE; print "remote $e";
    ///     for e in $DEV_SSH_AGENT; print "ssh-agent $e";
    ///     [[ -n $DEV_SSH_AGENT_SOCK ]] && print "ssh-agent-sock $DEV_SSH_AGENT_SOCK"
    ///   } | dev config import
    Import {
        /// Overwrite an existing config.toml.
        #[arg(long)]
        force: bool,
    },
    /// Emit the DEV_* bash arrays for the coder.nix bridge (`eval` this).
    ExportSh,
}

pub fn run(cmd: &ConfigCmd, json: bool) -> Result<()> {
    match cmd {
        ConfigCmd::Show => show(json),
        ConfigCmd::Validate => validate(),
        ConfigCmd::Import { force } => import(*force),
        ConfigCmd::ExportSh => {
            // Strict on purpose: the coder.nix bridge only calls this when
            // config.toml exists, and evals the output. If the file is malformed
            // we must emit *nothing* and fail, so `eval "$(…)"` is a no-op and the
            // DEV_* arrays sourced from local.zsh survive as the fallback.
            print!("{}", Config::load()?.to_export_sh());
            Ok(())
        }
    }
}

fn show(json: bool) -> Result<()> {
    let cfg = Config::load_or_default();
    if json {
        println!("{}", serde_json::to_string_pretty(&cfg)?);
    } else {
        print!("{}", cfg.to_toml_string()?);
    }
    Ok(())
}

fn validate() -> Result<()> {
    let cfg = Config::load().context("config.toml must exist and parse to validate")?;
    let mut failures = 0usize;
    let mut warnings = 0usize;

    println!("CONFIG {}", config::config_path().display());
    println!(
        "  envs={} local={} remote={}",
        cfg.envs.len(),
        cfg.local.len(),
        cfg.remote.len()
    );

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
    for e in &cfg.envs {
        match e.shell.as_str() {
            "bash" | "zsh" | "pwsh" | "nu" => {}
            other => {
                println!("warn  env {} has unknown shell '{}'", e.name, other);
                warnings += 1;
            }
        }
        if e.agent_forward && cfg.settings.ssh_agent_sock.is_empty() {
            // not fatal — SSH_AUTH_SOCK may still be set at runtime
            println!(
                "warn  env {} forwards agent but no ssh_agent_sock set",
                e.name
            );
            warnings += 1;
        }
    }

    println!("SUMMARY failures={failures} warnings={warnings}");
    if failures > 0 {
        bail!("{failures} config error(s)");
    }
    Ok(())
}

fn import(force: bool) -> Result<()> {
    let path = config::config_path();
    if path.exists() && !force {
        bail!(
            "{} already exists — pass --force to overwrite",
            path.display()
        );
    }
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("reading stdin")?;
    let cfg = config::from_import_lines(&buf);
    if cfg.envs.is_empty() && cfg.local.is_empty() && cfg.remote.is_empty() {
        bail!("no entries parsed from stdin — expected tagged lines (env/local/remote/…)");
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, cfg.to_toml_string()?)
        .with_context(|| format!("writing {}", path.display()))?;
    eprintln!(
        "wrote {} ({} env, {} local, {} remote)",
        path.display(),
        cfg.envs.len(),
        cfg.local.len(),
        cfg.remote.len()
    );
    Ok(())
}
