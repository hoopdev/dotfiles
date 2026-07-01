# dev Rust Workspace

`pkgs/` is the Rust workspace for the `dev` fleet tool.

## Crates

| Crate | Kind | Host build? | Verify with |
|---|---|---:|---|
| `dev-core` | Shared library: config, ssh, git, agent, windows, notify, statusline, store, task | yes | `check` / `test` |
| `dev-cli` | `dev` binary and full command surface | yes | `check` / `test`, `run-cli` |
| `dev-tui` | ratatui terminal UI | yes | `check` / `test` |
| `dev-zellij` | Zellij WASM plugin | no host link | `check` only |

`dev-core` modules are feature-gated:

- `config`, `ssh`, `agent`, `notify` are behind the `config` feature.
- `git` is behind `git`.
- Windows pwsh/nu paths are behind `windows`, which enables `config`.
- `dev-zellij` enables only `wasm` plus the always-on `store`, `task`, and `statusline` modules, so it does not pull host process or SSH code into WASM.

## Toolchain

Use the flake `rust` dev shell. It provides cargo, rustc, clippy, rustfmt, cmake,
`just`, and macOS libiconv wiring.

```bash
nix develop .#rust -c just -f pkgs/justfile ci
nix develop .#rust -c cargo check --manifest-path pkgs/Cargo.toml --workspace
```

Or enter it once:

```bash
nix develop .#rust
cd pkgs
just ci
just build
just clippy
just run-cli task list
```

Do not use ad hoc `nix shell nixpkgs#cargo ...`; it misses the workspace's
cmake/libiconv setup and can fail to link on macOS.

## Verification

- Logic and non-interactive behavior: `just ci`.
- CLI behavior: `just run-cli <args>`, for example `just run-cli task list`.
- TUI behavior: compile with `check`; interactive behavior must be exercised with `dev tui`.
- Zellij plugin: `cargo check`/`clippy` can type-check it, but host `build`/`test` intentionally exclude it because it targets `wasm32-wasi`.

`dev-zellij`'s real artifact is produced by Nix and installed through the macOS configuration. `dev board` is the user-facing entry point.

## Useful Commands

```bash
nix develop .#rust -c just -f pkgs/justfile ci
nix develop .#rust -c just -f pkgs/justfile run-cli -- task list
nix develop .#rust -c cargo check --manifest-path pkgs/Cargo.toml -p dev-core -p dev-cli -p dev-tui
```

## Known Notes

- `clippy` and `fmt-check` may report pre-existing `dev-zellij` style/lint debt. Do not use that as a reason to skip formatting your own edits.
- The Cargo warning about profiles being ignored for non-root packages is benign; the workspace root profile wins.
