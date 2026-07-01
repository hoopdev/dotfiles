//! dev CLI — the Rust implementation of the `dev` fleet tool.
//!
//! This binary is `dev`. Every subcommand the old bash `devCmd` supported lives
//! here as a typed clap command. `task` is a clap subcommand tree; `run` captures
//! its tail verbatim (trailing var-arg) and hands it to a small purpose-built
//! parser that preserves the `--` / `--all` / comma-target semantics.

mod cmd;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dev", version, about = "dev fleet CLI")]
struct Cli {
    /// Emit machine-readable JSON instead of human text. Works on any
    /// subcommand; equivalently set `DEV_JSON=1` (handy for agents / skills).
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Manage dev configuration (config.toml).
    Config {
        #[command(subcommand)]
        cmd: cmd::config::ConfigCmd,
    },
    /// List envs and projects.
    Ls,
    /// Flat array of every target (envs + projects).
    Targets,
    /// Detailed view of one target.
    Info { name: Option<String> },
    /// Agent backend registry.
    Backends,
    /// Models per backend.
    Models { backend: Option<String> },
    /// Validate tools, config, and reachability.
    Doctor {
        #[arg(long)]
        connect: bool,
        names: Vec<String>,
    },
    /// Interactive shell on a target.
    Shell { name: Option<String> },
    /// Open a project in VS Code.
    Code { name: Option<String> },
    /// Git across targets.
    Git {
        #[command(subcommand)]
        cmd: GitCmd,
    },
    /// Agent lifecycle.
    Agent {
        #[command(subcommand)]
        cmd: cmd::agent::AgentCmd,
    },
    /// Alias for `agent ps`.
    Ps,
    /// Alias for `agent dispatch` (background agent).
    Bg {
        project: String,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        task: Vec<String>,
    },
    /// Claude sessions.
    Session {
        #[command(subcommand)]
        cmd: SessionCmd,
    },
    /// Machine-readable task-board snapshot (data source for `board` / `tui`).
    Snapshot,
    /// Claude rate-limit summary.
    Usage,
    /// Claude Code statusLine hook: cache rate limits + print the 5h/7d bar.
    Statusline,
    /// Send a Telegram notification.
    Notify { msg: Vec<String> },
    /// Live fleet TUI.
    Tui {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Zellij task-board plugin.
    Board,
    /// Fleet dashboard (alias for the TUI).
    Dash,
    /// Shortcut: start claude in a project.
    Claude {
        project: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    /// Shortcut: start codex in a project.
    Codex {
        project: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    /// Shortcut: start opencode in a project.
    Opencode {
        project: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    /// Shortcut: start agy in a project.
    Agy {
        project: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    /// Task store — CRUD, questions, plan, dispatch, review.
    Task {
        #[command(subcommand)]
        cmd: cmd::task::TaskCmd,
    },
    /// Execute a command on one or many targets in parallel.
    Run {
        /// [--all | <target>] <cmd...> [--json] [--timeout N]
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum GitCmd {
    /// branch / head / dirty-count across targets.
    Status { names: Vec<String> },
    /// Show a diff for one target.
    Diff {
        name: Option<String>,
        #[arg(long)]
        stat: bool,
    },
    /// Worktrees.
    Worktree {
        #[command(subcommand)]
        cmd: WorktreeCmd,
    },
    /// Push HEAD and open a pull request.
    Pr {
        name: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        draft: bool,
    },
}

#[derive(Subcommand)]
enum WorktreeCmd {
    Ls { name: String },
    Rm { name: String, branch: String },
}

#[derive(Subcommand)]
enum SessionCmd {
    List {
        name: Option<String>,
    },
    Resume {
        name: Option<String>,
        session_id: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    // One rule for the whole CLI: human text by default; JSON when `--json` is
    // passed (anywhere) or `DEV_JSON` is set. Every command reads this value.
    let json = cmd::want_json(cli.json);
    match cli.command {
        Cmd::Config { cmd } => {
            if let Err(e) = cmd::config::run(&cmd, json) {
                eprintln!("dev config: {e:#}");
                std::process::exit(1);
            }
        }
        Cmd::Ls => cmd::ls::ls(json),
        Cmd::Targets => cmd::ls::targets(json),
        Cmd::Info { name } => cmd::info::info(name, json),
        Cmd::Backends => cmd::backends::backends(json),
        Cmd::Models { backend } => cmd::models::models(backend, json),
        Cmd::Doctor { connect, names } => cmd::doctor::doctor(connect, names),
        Cmd::Shell { name } => cmd::shell::shell(name),
        Cmd::Code { name } => cmd::code::code(name),
        Cmd::Git { cmd } => match cmd {
            GitCmd::Status { names } => cmd::git::status(names, json),
            GitCmd::Diff { name, stat } => cmd::git::diff(name, stat, json),
            GitCmd::Worktree { cmd } => match cmd {
                WorktreeCmd::Ls { name } => cmd::git::worktree_ls(name),
                WorktreeCmd::Rm { name, branch } => cmd::git::worktree_rm(name, branch),
            },
            GitCmd::Pr {
                name,
                title,
                base,
                draft,
            } => cmd::git::pr(name, title, base, draft, json),
        },
        Cmd::Agent { cmd } => cmd::agent::run(&cmd, json),
        Cmd::Ps => cmd::agent::run(&cmd::agent::AgentCmd::Ps, json),
        Cmd::Bg {
            project,
            backend,
            model,
            worktree,
            task,
        } => cmd::agent::run(
            &cmd::agent::AgentCmd::Dispatch {
                project,
                backend,
                model,
                effort: None,
                sandbox: None,
                worktree,
                supervise: false,
                task,
            },
            json,
        ),
        Cmd::Session { cmd } => match cmd {
            SessionCmd::List { name } => cmd::session::list(name, json),
            SessionCmd::Resume { name, session_id } => cmd::session::resume(name, session_id),
        },
        Cmd::Snapshot => cmd::snapshot::snapshot(json),
        Cmd::Usage => cmd::usage::usage(json),
        Cmd::Statusline => cmd::statusline::statusline(),
        Cmd::Notify { msg } => cmd::notify::notify(msg),
        Cmd::Tui { args } => cmd::launch::tui(args),
        Cmd::Board => cmd::launch::board(),
        Cmd::Dash => cmd::launch::dash(),
        Cmd::Claude { project, extra } => cmd::agent::start_fresh("claude", &project, &extra),
        Cmd::Codex { project, extra } => cmd::agent::start_fresh("codex", &project, &extra),
        Cmd::Opencode { project, extra } => cmd::agent::start_fresh("opencode", &project, &extra),
        Cmd::Agy { project, extra } => cmd::agent::start_fresh("agy", &project, &extra),
        Cmd::Task { cmd } => {
            if let Err(e) = cmd::task::run(cmd, json) {
                eprintln!("dev task: {e:#}");
                std::process::exit(1);
            }
        }
        Cmd::Run { args } => {
            if let Err(e) = cmd::run::run(&args, json) {
                eprintln!("dev run: {e:#}");
                std::process::exit(1);
            }
        }
    }
}
