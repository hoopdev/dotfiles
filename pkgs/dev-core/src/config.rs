//! dev configuration — typed model of `~/.config/dev/config.toml`.
//!
//! Single source of truth for the fleet topology. Replaces the `DEV_ENVS` /
//! `DEV_LOCAL` / `DEV_REMOTE` / `DEV_SSH_AGENT` bash arrays that used to live in
//! `~/.config/zsh/local.zsh`. Secrets (CODER_URL, Telegram tokens) stay in the
//! shell — this file only describes *where* targets are, never credentials.
//!
//! `dev config export-sh` still emits the old `DEV_*` arrays for compatibility
//! bridges, but `config.toml` is the source of truth.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

// ── model ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, rename = "env")]
    pub envs: Vec<Env>,
    #[serde(default)]
    pub local: Vec<LocalProject>,
    #[serde(default)]
    pub remote: Vec<RemoteProject>,
    #[serde(default)]
    pub settings: Settings,
}

/// A remote environment (SSH target). Mirrors a `DEV_ENVS` entry
/// `name|user@host|proxy|shell|os`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Env {
    pub name: String,
    /// `user@host` SSH connection string.
    pub host: String,
    /// ProxyCommand (e.g. `coder-proxy %h`); empty = direct SSH.
    #[serde(default)]
    pub proxy: String,
    /// Login shell on the remote: `bash` | `zsh` | `pwsh` | `nu`.
    #[serde(default = "default_shell")]
    pub shell: String,
    /// Display OS label; empty = inferred (`pwsh`/drive-path ⇒ windows).
    #[serde(default)]
    pub os: String,
    /// Former `DEV_SSH_AGENT` membership — forward the local agent onward.
    #[serde(default)]
    pub agent_forward: bool,
}

/// A local project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalProject {
    pub name: String,
    pub path: String,
    /// Default agent backend for this project (empty ⇒ claude). Lets `dispatch` /
    /// `attach` skip `--backend` for projects with a fixed agent.
    #[serde(default)]
    pub backend: String,
}

/// A remote project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteProject {
    pub name: String,
    /// References an [`Env::name`].
    pub env: String,
    pub path: String,
    /// Default agent backend for this project (empty ⇒ claude).
    #[serde(default)]
    pub backend: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Former `DEV_SSH_AGENT_SOCK`; empty = fall back to the 1Password default.
    #[serde(default)]
    pub ssh_agent_sock: String,
}

fn default_shell() -> String {
    "bash".to_string()
}

impl Env {
    /// Whether this env is a Windows host (pwsh shell or explicit `os=windows`).
    /// Used to route SSH exec / git status through the windows module.
    pub fn is_windows(&self) -> bool {
        self.shell == "pwsh" || self.os == "windows"
    }
}

// ── resolved target ─────────────────────────────────────────────────────────

/// Result of resolving a name against the config — the three kinds a `dev`
/// command can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Local {
        name: String,
        path: String,
    },
    Remote {
        name: String,
        env: String,
        path: String,
    },
    Env {
        name: String,
    },
}

// ── paths ───────────────────────────────────────────────────────────────────

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

/// Location of `config.toml` — `$DEV_CONFIG`, else `$XDG_CONFIG_HOME/dev/`, else
/// `~/.config/dev/`.
pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("DEV_CONFIG") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}/.config", home()));
    PathBuf::from(base).join("dev").join("config.toml")
}

fn expand_tilde(p: &str) -> String {
    if p == "~" {
        home()
    } else if let Some(rest) = p.strip_prefix("~/") {
        format!("{}/{}", home(), rest)
    } else {
        p.to_string()
    }
}

fn is_drive_path(p: &str) -> bool {
    let b = p.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'/' || b[2] == b'\\')
}

// ── load / save ─────────────────────────────────────────────────────────────

impl Config {
    /// Parse `config.toml`. Errors if the file is missing or malformed — use
    /// [`Config::load_or_default`] for the lenient path the shell/TUI want.
    pub fn load() -> Result<Config> {
        let p = config_path();
        let text =
            std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
        let mut cfg: Config =
            toml::from_str(&text).with_context(|| format!("parsing {}", p.display()))?;
        cfg.normalize();
        Ok(cfg)
    }

    /// Lenient load: missing/invalid config ⇒ empty config (matches the bash
    /// behaviour where absent `DEV_*` arrays simply yield no targets).
    pub fn load_or_default() -> Config {
        Config::load().unwrap_or_default()
    }

    fn normalize(&mut self) {
        for l in &mut self.local {
            l.path = expand_tilde(&l.path);
        }
        for e in &mut self.envs {
            if e.shell.is_empty() {
                e.shell = default_shell();
            }
        }
    }

    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self).context("serializing config to TOML")
    }
}

// ── resolution ───────────────────────────────────────────────────────────────

impl Config {
    pub fn local_path(&self, name: &str) -> Option<&str> {
        self.local
            .iter()
            .find(|l| l.name == name)
            .map(|l| l.path.as_str())
    }

    pub fn remote(&self, name: &str) -> Option<&RemoteProject> {
        self.remote.iter().find(|r| r.name == name)
    }

    pub fn env(&self, name: &str) -> Option<&Env> {
        self.envs.iter().find(|e| e.name == name)
    }

    /// `local-project` | `remote-project` | `env` | `None`.
    pub fn target_kind(&self, name: &str) -> Option<&'static str> {
        if self.local_path(name).is_some() {
            Some("local-project")
        } else if self.remote(name).is_some() {
            Some("remote-project")
        } else if self.env(name).is_some() {
            Some("env")
        } else {
            None
        }
    }

    pub fn resolve(&self, name: &str) -> Option<Target> {
        if let Some(p) = self.local_path(name) {
            Some(Target::Local {
                name: name.to_string(),
                path: p.to_string(),
            })
        } else if let Some(r) = self.remote(name) {
            Some(Target::Remote {
                name: name.to_string(),
                env: r.env.clone(),
                path: r.path.clone(),
            })
        } else if self.env(name).is_some() {
            Some(Target::Env {
                name: name.to_string(),
            })
        } else {
            None
        }
    }

    pub fn list_projects(&self) -> Vec<String> {
        self.local
            .iter()
            .map(|l| l.name.clone())
            .chain(self.remote.iter().map(|r| r.name.clone()))
            .collect()
    }

    pub fn list_envs(&self) -> Vec<String> {
        self.envs.iter().map(|e| e.name.clone()).collect()
    }

    /// Configured default agent backend for a project (local or remote), if set.
    pub fn project_backend(&self, name: &str) -> Option<String> {
        let t = self
            .local
            .iter()
            .find(|l| l.name == name)
            .map(|l| l.backend.clone())
            .or_else(|| {
                self.remote
                    .iter()
                    .find(|r| r.name == name)
                    .map(|r| r.backend.clone())
            })?;
        (!t.is_empty()).then_some(t)
    }

    /// OS for an env, given an optional path hint (mirrors `_env_get_os`):
    /// explicit `os` wins, then `pwsh` ⇒ windows, then a drive-letter path
    /// (this env's or any remote project using it) ⇒ windows, else `unknown`.
    pub fn env_os(&self, env_name: &str, path: &str) -> String {
        let Some(e) = self.env(env_name) else {
            return "unknown".to_string();
        };
        if !e.os.is_empty() {
            return e.os.clone();
        }
        if e.shell == "pwsh" {
            return "windows".to_string();
        }
        if is_drive_path(path) {
            return "windows".to_string();
        }
        if self
            .remote
            .iter()
            .any(|r| r.env == env_name && is_drive_path(&r.path))
        {
            return "windows".to_string();
        }
        "unknown".to_string()
    }
}

// ── machine-readable surface ─────────────────────────────────────────────────

impl Config {
    /// Flat array of every project + env — the schema `dev targets --json`
    /// produces and that `dev-tui` / `dev run` consume. Key order may differ;
    /// every consumer parses the JSON, so only the schema matters.
    pub fn targets_json(&self) -> Value {
        let mut arr: Vec<Value> = Vec::new();
        for l in &self.local {
            arr.push(json!({
                "name": l.name,
                "kind": "local-project",
                "env": Value::Null,
                "host": Value::Null,
                "shell": Value::Null,
                "os": Value::Null,
                "proxy": Value::Null,
                "path": l.path,
            }));
        }
        for r in &self.remote {
            let host = self.env(&r.env).map(|e| e.host.clone()).unwrap_or_default();
            let shell = self
                .env(&r.env)
                .map(|e| e.shell.clone())
                .unwrap_or_else(default_shell);
            let os = self.env_os(&r.env, &r.path);
            arr.push(json!({
                "name": r.name,
                "kind": "remote-project",
                "env": r.env,
                "host": host,
                "shell": shell,
                "os": os,
                "proxy": Value::Null,
                "path": r.path,
            }));
        }
        for e in &self.envs {
            let os = self.env_os(&e.name, "");
            arr.push(json!({
                "name": e.name,
                "kind": "env",
                "env": e.name,
                "host": e.host,
                "shell": if e.shell.is_empty() { default_shell() } else { e.shell.clone() },
                "os": os,
                "proxy": e.proxy,
                "path": Value::Null,
            }));
        }
        Value::Array(arr)
    }
}

// ── shell bridge (export-sh / import) ───────────────────────────────────────

fn sh_squote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

impl Config {
    /// Emit the legacy `DEV_*` arrays for compatibility bridges.
    pub fn to_export_sh(&self) -> String {
        let mut out = String::new();

        out.push_str("DEV_ENVS=(");
        for e in &self.envs {
            let entry = format!("{}|{}|{}|{}|{}", e.name, e.host, e.proxy, e.shell, e.os);
            out.push(' ');
            out.push_str(&sh_squote(&entry));
        }
        out.push_str(" )\n");

        out.push_str("DEV_LOCAL=(");
        for l in &self.local {
            let entry = format!("{}|{}", l.name, l.path);
            out.push(' ');
            out.push_str(&sh_squote(&entry));
        }
        out.push_str(" )\n");

        out.push_str("DEV_REMOTE=(");
        for r in &self.remote {
            let entry = format!("{}|{}|{}", r.name, r.env, r.path);
            out.push(' ');
            out.push_str(&sh_squote(&entry));
        }
        out.push_str(" )\n");

        out.push_str("DEV_SSH_AGENT=(");
        for e in self.envs.iter().filter(|e| e.agent_forward) {
            out.push(' ');
            out.push_str(&sh_squote(&e.name));
        }
        out.push_str(" )\n");

        if !self.settings.ssh_agent_sock.is_empty() {
            out.push_str(&format!(
                "export DEV_SSH_AGENT_SOCK={}\n",
                sh_squote(&self.settings.ssh_agent_sock)
            ));
        }
        out
    }

    /// Parse the `DEV_*=( … )` shell arrays emitted by [`Config::to_export_sh`].
    /// Used for the round-trip test and as a secondary import path.
    pub fn from_export_sh(text: &str) -> Config {
        let mut cfg = Config::default();
        let mut agent_names: Vec<String> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some(inner) = array_body(line, "DEV_ENVS") {
                for tok in parse_sq_tokens(inner) {
                    let f: Vec<&str> = tok.splitn(5, '|').collect();
                    if f.is_empty() || f[0].is_empty() {
                        continue;
                    }
                    cfg.envs.push(Env {
                        name: f[0].to_string(),
                        host: f.get(1).unwrap_or(&"").to_string(),
                        proxy: f.get(2).unwrap_or(&"").to_string(),
                        shell: {
                            let s = f.get(3).unwrap_or(&"").to_string();
                            if s.is_empty() {
                                default_shell()
                            } else {
                                s
                            }
                        },
                        os: f.get(4).unwrap_or(&"").to_string(),
                        agent_forward: false,
                    });
                }
            } else if let Some(inner) = array_body(line, "DEV_LOCAL") {
                for tok in parse_sq_tokens(inner) {
                    let f: Vec<&str> = tok.splitn(2, '|').collect();
                    if f.len() == 2 && !f[0].is_empty() {
                        cfg.local.push(LocalProject {
                            name: f[0].to_string(),
                            path: f[1].to_string(),
                            backend: String::new(),
                        });
                    }
                }
            } else if let Some(inner) = array_body(line, "DEV_REMOTE") {
                for tok in parse_sq_tokens(inner) {
                    let f: Vec<&str> = tok.splitn(3, '|').collect();
                    if f.len() == 3 && !f[0].is_empty() {
                        cfg.remote.push(RemoteProject {
                            name: f[0].to_string(),
                            env: f[1].to_string(),
                            path: f[2].to_string(),
                            backend: String::new(),
                        });
                    }
                }
            } else if let Some(inner) = array_body(line, "DEV_SSH_AGENT") {
                agent_names = parse_sq_tokens(inner);
            } else if let Some(rest) = line.strip_prefix("export DEV_SSH_AGENT_SOCK=") {
                cfg.settings.ssh_agent_sock = unquote_scalar(rest);
            }
        }
        for e in &mut cfg.envs {
            if agent_names.iter().any(|n| n == &e.name) {
                e.agent_forward = true;
            }
        }
        cfg
    }
}

/// If `line` is `NAME=( … )`, return the `…` between the parens.
fn array_body<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=(");
    let rest = line.strip_prefix(&prefix)?;
    let rest = rest.strip_suffix(')')?;
    Some(rest.trim())
}

/// Tokenize a run of single-quoted shell words, honouring the `'\''` escape.
fn parse_sq_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_whitespace() {
            i += 1;
            continue;
        }
        if bytes[i] != '\'' {
            // Bare (unquoted) token — read until whitespace.
            let start = i;
            while i < bytes.len() && !bytes[i].is_whitespace() {
                i += 1;
            }
            out.push(bytes[start..i].iter().collect());
            continue;
        }
        // Quoted token starting at a `'`.
        i += 1; // skip opening quote
        let mut val = String::new();
        loop {
            if i >= bytes.len() {
                break;
            }
            if bytes[i] == '\'' {
                // `'\''` → literal single quote, still inside the token.
                if i + 3 < bytes.len()
                    && bytes[i + 1] == '\\'
                    && bytes[i + 2] == '\''
                    && bytes[i + 3] == '\''
                {
                    val.push('\'');
                    i += 4;
                    continue;
                }
                i += 1; // closing quote
                break;
            }
            val.push(bytes[i]);
            i += 1;
        }
        out.push(val);
    }
    out
}

fn unquote_scalar(s: &str) -> String {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')) {
        return inner.replace("'\\''", "'");
    }
    if let Some(inner) = s.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        return inner.to_string();
    }
    s.to_string()
}

// ── tagged-line import (for the initial migration from live zsh arrays) ──────

/// Parse the tagged-line stream produced by the documented zsh one-liner:
/// ```text
/// env    name|host|proxy|shell|os
/// local  name|path
/// remote name|env|path
/// ssh-agent      name
/// ssh-agent-sock path
/// ```
pub fn from_import_lines(text: &str) -> Config {
    let mut cfg = Config::default();
    let mut agent_names: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (tag, rest) = match line.split_once(char::is_whitespace) {
            Some((t, r)) => (t, r.trim()),
            None => (line, ""),
        };
        match tag {
            "env" => {
                let f: Vec<&str> = rest.splitn(5, '|').collect();
                if f.is_empty() || f[0].is_empty() {
                    continue;
                }
                cfg.envs.push(Env {
                    name: f[0].to_string(),
                    host: f.get(1).unwrap_or(&"").to_string(),
                    proxy: f.get(2).unwrap_or(&"").to_string(),
                    shell: {
                        let s = f.get(3).unwrap_or(&"").to_string();
                        if s.is_empty() {
                            default_shell()
                        } else {
                            s
                        }
                    },
                    os: f.get(4).unwrap_or(&"").to_string(),
                    agent_forward: false,
                });
            }
            "local" => {
                // name|path[|backend]
                let f: Vec<&str> = rest.splitn(3, '|').collect();
                if f.len() >= 2 && !f[0].is_empty() {
                    cfg.local.push(LocalProject {
                        name: f[0].to_string(),
                        path: expand_tilde(f[1]),
                        backend: f.get(2).unwrap_or(&"").to_string(),
                    });
                }
            }
            "remote" => {
                // name|env|path[|backend]
                let f: Vec<&str> = rest.splitn(4, '|').collect();
                if f.len() >= 3 && !f[0].is_empty() {
                    cfg.remote.push(RemoteProject {
                        name: f[0].to_string(),
                        env: f[1].to_string(),
                        path: f[2].to_string(),
                        backend: f.get(3).unwrap_or(&"").to_string(),
                    });
                }
            }
            "ssh-agent" if !rest.is_empty() => {
                agent_names.push(rest.to_string());
            }
            "ssh-agent-sock" => {
                cfg.settings.ssh_agent_sock = rest.to_string();
            }
            _ => {}
        }
    }
    for e in &mut cfg.envs {
        if agent_names.iter().any(|n| n == &e.name) {
            e.agent_forward = true;
        }
    }
    cfg
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Config {
        Config {
            envs: vec![
                Env {
                    name: "myenv".into(),
                    host: "user@myenv.example.com".into(),
                    proxy: "coder-proxy %h".into(),
                    shell: "bash".into(),
                    os: "linux".into(),
                    agent_forward: true,
                },
                Env {
                    name: "win".into(),
                    host: "user@win.ts.net".into(),
                    proxy: "".into(),
                    shell: "nu".into(),
                    os: "".into(),
                    agent_forward: false,
                },
            ],
            local: vec![LocalProject {
                name: "proj-local".into(),
                path: "/home/user/git/proj".into(),
                backend: "opencode".into(),
            }],
            remote: vec![
                RemoteProject {
                    name: "proj-server".into(),
                    env: "myenv".into(),
                    path: "/home/user/proj".into(),
                    backend: String::new(),
                },
                RemoteProject {
                    name: "win-proj".into(),
                    env: "win".into(),
                    path: "C:/Users/user/project".into(),
                    backend: String::new(),
                },
            ],
            settings: Settings {
                ssh_agent_sock: "/tmp/agent.sock".into(),
            },
        }
    }

    #[test]
    fn export_sh_round_trips() {
        let cfg = sample();
        let sh = cfg.to_export_sh();
        let back = Config::from_export_sh(&sh);
        assert_eq!(back.envs.len(), 2);
        assert_eq!(back.local.len(), 1);
        assert_eq!(back.remote.len(), 2);
        assert_eq!(back.settings.ssh_agent_sock, "/tmp/agent.sock");
        // agent_forward is reconstructed from the DEV_SSH_AGENT array
        assert!(back.env("myenv").unwrap().agent_forward);
        assert!(!back.env("win").unwrap().agent_forward);
        assert_eq!(
            back.remote("win-proj").unwrap().path,
            "C:/Users/user/project"
        );
        assert_eq!(back.env("myenv").unwrap().proxy, "coder-proxy %h");
    }

    #[test]
    fn toml_round_trips() {
        let cfg = sample();
        let s = cfg.to_toml_string().unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(back.envs.len(), 2);
        assert_eq!(back.remote.len(), 2);
        assert_eq!(back.env("win").unwrap().shell, "nu");
    }

    #[test]
    fn os_inference() {
        let cfg = sample();
        // explicit
        assert_eq!(cfg.env_os("myenv", "/home/user/proj"), "linux");
        // nu shell, no explicit os, but a remote project on it uses a drive path
        assert_eq!(cfg.env_os("win", ""), "windows");
        assert_eq!(cfg.env_os("win", "C:/x"), "windows");
    }

    #[test]
    fn targets_json_schema() {
        let cfg = sample();
        let v = cfg.targets_json();
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 5); // 1 local + 2 remote + 2 env

        let local = arr
            .iter()
            .find(|t| t["name"] == json!("proj-local"))
            .unwrap();
        assert_eq!(local["kind"], json!("local-project"));
        assert_eq!(local["host"], Value::Null);
        assert_eq!(local["path"], json!("/home/user/git/proj"));

        let remote = arr
            .iter()
            .find(|t| t["name"] == json!("proj-server"))
            .unwrap();
        assert_eq!(remote["kind"], json!("remote-project"));
        assert_eq!(remote["env"], json!("myenv"));
        assert_eq!(remote["host"], json!("user@myenv.example.com"));
        assert_eq!(remote["os"], json!("linux"));

        let env = arr.iter().find(|t| t["name"] == json!("win")).unwrap();
        assert_eq!(env["kind"], json!("env"));
        assert_eq!(env["path"], Value::Null);
        assert_eq!(env["os"], json!("windows"));
    }

    #[test]
    fn import_lines_parse() {
        let text = "\
env myenv|user@myenv.example.com|coder-proxy %h|bash|linux
env win|user@win.ts.net||nu|
local proj-local|/home/user/git/proj
remote proj-server|myenv|/home/user/proj
remote win-proj|win|C:/Users/user/project
ssh-agent myenv
ssh-agent-sock /tmp/agent.sock
";
        let cfg = from_import_lines(text);
        assert_eq!(cfg.envs.len(), 2);
        assert_eq!(cfg.local.len(), 1);
        assert_eq!(cfg.remote.len(), 2);
        assert_eq!(cfg.env("win").unwrap().shell, "nu");
        assert!(cfg.env("myenv").unwrap().agent_forward);
        assert_eq!(cfg.settings.ssh_agent_sock, "/tmp/agent.sock");
        // full pipeline: import → export-sh → import is stable
        let back = Config::from_export_sh(&cfg.to_export_sh());
        assert_eq!(back.envs.len(), cfg.envs.len());
        assert_eq!(back.remote.len(), cfg.remote.len());
        assert!(back.env("myenv").unwrap().agent_forward);
    }
}
