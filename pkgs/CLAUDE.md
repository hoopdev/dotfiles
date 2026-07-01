# CLAUDE.md - `pkgs/`

This workspace is the Rust implementation of the `dev` fleet tool. Keep this
file focused on engineering direction and local decision rules. Operational
commands live in [../docs/dev-rust-workspace.md](../docs/dev-rust-workspace.md).

## North Star

`dev-core` is the application layer. `dev-cli`, `dev-tui`, and `dev-zellij` are
adapters over the same domain concepts.

- Put target resolution, config, task lifecycle, agent/run registry, git status,
  and snapshot construction in `dev-core`.
- Keep `dev-cli` responsible for argument parsing, exit codes, and human/JSON
  formatting.
- Keep `dev-tui` responsible for interaction state, terminal effects, and
  rendering.
- Keep `dev-zellij` constrained to the Zellij sandbox: read `dev snapshot --json`
  and issue explicit action commands.

## Current Direction

The Rust migration is done. Do not revive bash-era migration plans or duplicate
logic in shell.

Current refactor direction is documented in
[../docs/dev-cli-tui-refactor.md](../docs/dev-cli-tui-refactor.md):

- move `dev task` lifecycle behavior into a `dev-core` service layer;
- unify CLI/TUI/Zellij read models through snapshot builders;
- split TUI state by concern;
- separate input handling from side effects;
- contain raw `serde_json::Value` at serialization boundaries.

## Boundaries

Interactive terminal work is allowed to stay in adapters:

- attach/resume flows;
- live log follow;
- pager or Zellij pane/tab operations;
- commands whose purpose is to hand the user into another process.

Everything else should prefer direct Rust calls over invoking `dev` as a
subprocess. A `dev` self-call inside `dev-cli` is a smell unless it is one of the
interactive boundaries above.

## Contracts

Human output is the default. Machine output is requested by the global `--json`
flag or `DEV_JSON=1`. Do not add per-command `--json` flags.

Keep JSON schemas stable for automation and Zellij. Rust-internal consumers
should prefer typed `dev-core` APIs and snapshot structs instead of reparsing
CLI JSON.

Agent run logs are the agent's own stdout and should remain human-formatted.
Renderers may strip ANSI for display, but command launch code should not force
agent logs into JSON unless the receiving command explicitly expects JSON.

## Change Discipline

Before changing behavior, identify which layer owns it:

- domain rule or lifecycle transition: `dev-core`;
- command syntax or output shape: `dev-cli`;
- keybinding, selection, overlay, or terminal effect: `dev-tui`;
- Zellij layout/action bridge: `dev-zellij`.

Prefer adding tests around pure domain behavior in `dev-core`. For TUI changes,
push logic toward state transitions that can be tested without a terminal.

Use the workspace toolchain and verification notes in
[../docs/dev-rust-workspace.md](../docs/dev-rust-workspace.md) before asking the
user to run `nh darwin switch`.
