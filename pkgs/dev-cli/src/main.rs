//! dev CLI — Rust implementation of dev task subcommands.

mod cmd;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("task") => cmd::task::run(&args[1..]),
        Some("run") => cmd::run::run(&args[1..]),
        Some(c) => {
            eprintln!("dev: unknown command '{c}'. This binary handles: task, run");
            std::process::exit(1);
        }
        None => {
            eprintln!("Usage: dev <command> [args...]\nCommands: task, run");
            std::process::exit(1);
        }
    }
}
