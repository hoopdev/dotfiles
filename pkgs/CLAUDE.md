# CLAUDE.md — Rust workspace (`pkgs/`)

Cargo workspace that **is** the `dev` fleet tool. `dev-cli` builds the `dev`
binary. The build definitions live in `pkgs/default.nix` (exposed as flake
packages by `flake-modules/rust.nix` — `nix build .#dev`); `home/mac/dev.nix` is
just a thin wrapper that sets up the env and execs it (all target-resolution /
SSH / git / agent logic lives here, not in bash). `home/mac/coder.nix` is now
coder-connection only. Fleet topology is read from `~/.config/dev/config.toml`
(managed by `dev config`, backed by `dev_core::config`); `local.zsh` holds only
secrets.

Verify changes here **before** a `nh darwin switch` (the switch rebuilds these
via nix, which is slow and non-incremental — use `cargo` for iteration).

## Toolchain: the `rust` devShell

`cargo` is **not** on your PATH. The toolchain (cargo, rustc, clippy, rustfmt,
cmake, `just`, and — on macOS — libiconv with `LIBRARY_PATH` set) lives in the
flake's `rust` devShell. Prefix every command with it:

```bash
# from the repo root
nix develop .#rust -c just -f pkgs/justfile ci      # check + test (the gate)
nix develop .#rust -c cargo check --manifest-path pkgs/Cargo.toml --workspace
```

Or enter it once and use bare `cargo`/`just` (recipes live in `pkgs/justfile`):

```bash
nix develop .#rust          # then, from pkgs/:
just ci                     # check + test
just build                  # host binaries
just clippy                 # advisory lint
just run-cli task list      # actually run dev-cli to check behavior
```

Do **not** hand-roll `nix shell nixpkgs#cargo …`; it misses cmake/libiconv and
the `-liconv` link fails on macOS.

## Crates

| Crate        | Kind                        | Host build? | Verify with |
|--------------|-----------------------------|-------------|-------------|
| `dev-core`   | lib: `config` `ssh` `git` `agent` `windows` `notify` `statusline` `store` `task` | ✅ | `check` / `test` |
| `dev-cli`    | bin `dev` (full command surface) | ✅     | `check` / `test`, `run-cli` |
| `dev-tui`    | bin (ratatui TUI)           | ✅          | `check` / `test` (see below) |
| `dev-zellij` | **WASM** plugin             | ❌ link     | `check` only |

`dev-core` modules are feature-gated: `config`/`ssh`/`agent`/`notify` behind the
`config` feature, `git` behind `git`, the pwsh/nu paths behind `windows`
(`= ["config"]`). `dev-cli` and `dev-tui` enable `config`, `git`, `windows`;
`dev-zellij` (WASM) enables only `wasm` + the always-on `store`/`task`/`statusline`,
so it never pulls `std::process`/ssh. The TUI and Zellij plugin call `dev-core`
in-process (no `dev … --json` subprocess) for data; they shell out to `dev` only
for interactive actions (attach, dispatch).

`dev-zellij` is built by nix for `wasm32-wasi` (see `pkgs/default.nix`
`dev-zellij`, wired in via `home/mac/dev.nix`). It pulls curl/openssl
transitively and **cannot link for the
host**, so `just build`/`just test` exclude it. `cargo check`/`clippy` still
cover it because they don't link. Its real artifact is produced by
`nh darwin switch` (or `dev board` to run it).

## Verifying behavior

- **Logic / correctness** — `just ci` (check + test). Add tests next to the code
  under `#[cfg(test)]`; the harness already runs (currently 0 tests).
- **`dev-cli`** — run it: `just run-cli <args>` (e.g. `run-cli task list`,
  `run-cli run <target> -- echo hi`). This is the easiest end-to-end check.
- **`dev-tui`** — a full-screen TUI; it can't be driven headlessly here. Confirm
  it compiles (`check`), then have the user run `dev tui` for real interaction.
  The `dev agent review` flow it drives lives in `coder.nix` `_dev_review`.

## Output convention (humans ↔ agents)

One rule across every `dev` command: **human-formatted text by default; JSON on
demand.** JSON is requested by the single global `--json` flag (works before or
after any subcommand — it's `global = true` on the root) OR the `DEV_JSON=1` env
var, so an agent/skill opts in once instead of per-command. `main()` computes it
via `cmd::want_json(cli.json)` and threads one `bool` into every handler; do
**not** add per-command `--json` flags (they'd clash with the global one). Keep
human output stable/greppable (aligned columns, one record per line).

Agent run logs (`~/.dev/runs/*.log`) are the *agent's own* stdout, shown raw by
`dev logs` / the TUI — so agents are launched in their **human-formatted** mode
(e.g. opencode default, not `--format json`); the TUI strips ANSI on display.

## Notes

- `clippy` and `fmt-check` are **advisory** — `dev-zellij` currently carries some
  pre-existing style/lint debt, so they report but don't gate. Run `just fmt`
  before committing your own changes.
- The `Cargo.toml` "profiles ignored for non-root package" warning is benign
  (per-crate `[profile.release]` blocks; the workspace root wins).
