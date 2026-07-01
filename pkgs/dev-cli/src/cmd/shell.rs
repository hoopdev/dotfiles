//! `dev shell` — interactive shell on a target (local or remote).

use super::interactive;
use dev_core::config::Config;

pub fn shell(name: Option<String>) {
    let cfg = Config::load_or_default();
    let n = interactive::require_any(&cfg, name.as_deref());
    let Some(t) = cfg.resolve(&n) else {
        eprintln!("dev shell: unknown name '{n}'");
        std::process::exit(1);
    };
    interactive::exec_on_target(&cfg, &t, None);
}
