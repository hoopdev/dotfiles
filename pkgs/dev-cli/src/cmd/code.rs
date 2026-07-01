//! `dev code` — open a project in VS Code (local or via Remote-SSH).

use super::interactive;
use dev_core::config::{Config, Target};
use std::os::unix::process::CommandExt;
use std::process::Command;

pub fn code(name: Option<String>) {
    let cfg = Config::load_or_default();
    let n = interactive::require_project(&cfg, name.as_deref());
    let Some(t) = cfg.resolve(&n) else {
        eprintln!("dev code: unknown project '{n}'");
        std::process::exit(1);
    };
    let err = match t {
        Target::Local { path, .. } => Command::new("code").arg(&path).exec(),
        Target::Remote { env, path, .. } => {
            let Some(e) = cfg.env(&env) else {
                eprintln!("dev code: unknown env for '{n}'");
                std::process::exit(1);
            };
            if e.shell == "pwsh" {
                eprintln!("dev code: pwsh remotes are not supported");
                std::process::exit(1);
            }
            Command::new("code")
                .arg("--remote")
                .arg(format!("ssh-remote+{}", e.host))
                .arg(&path)
                .exec()
        }
        Target::Env { .. } => {
            eprintln!("dev code: '{n}' is an env; pass a project");
            std::process::exit(1);
        }
    };
    eprintln!("dev code: {err}");
    std::process::exit(1);
}
