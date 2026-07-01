pub mod agent;
pub mod backends;
pub mod code;
pub mod config;
pub mod doctor;
pub mod git;
pub mod info;
pub mod interactive;
pub mod launch;
pub mod ls;
pub mod models;
pub mod notify;
pub mod run;
pub mod session;
pub mod shell;
pub mod snapshot;
pub mod statusline;
pub mod task;
pub mod usage;

/// Whether commands should emit machine-readable JSON instead of human text.
///
/// True when the global `--json` flag is passed OR the `DEV_JSON` env var is
/// set truthy. The env var lets an AI agent / skill opt into JSON once for a
/// whole session (`export DEV_JSON=1`) instead of appending `--json` to every
/// invocation; humans get formatted text by default.
pub fn want_json(flag: bool) -> bool {
    flag || matches!(
        std::env::var("DEV_JSON").ok().as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}
