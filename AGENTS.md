# Agent instructions

Nix Flakes dotfiles. NixOS / macOS (nix-darwin) / standalone home-manager.

Apply changes with `nh {os,darwin,home} switch` — full command reference (apply, bootstrap, maintenance) in [docs/commands.md](docs/commands.md).

## Design Guidelines

- Add packages via Nix, never `brew install` / `apt-get`
- Theming is unified via Stylix — color changes go in `lib/shonan.yaml`, not per-app configs
- Cross-platform config lives in `home/common/`; `home/mac/` and `home/nixos/` are overlays that extend it
- Hosts are auto-discovered from `hosts/<name>/meta.nix` — adding one needs no edits to `flake-modules/*.nix`
- Skills are managed outside this repository through Orca. Keep project-specific rules here; do not add skill sources, installers, or synchronization to dotfiles.
- Format with `nixfmt`; lint with `statix` and `deadnix`
- Directory layout, key components, and design principles: [docs/architecture.md](docs/architecture.md)

## Cautions

- Determine target platform (NixOS vs macOS) before suggesting system-level changes
- `flake.nix` / `flake.lock` changes affect all hosts — verify carefully
- Chezmoi sync: an activation hook in `home/common/cli/neovim.nix` copies `init.lua` into `dot_config/nvim/` on rebuild — don't remove it
- Generated Chezmoi files: `dot_config/{readonly_starship.toml,wezterm/}` are rendered from the Nix config by `nix run .#export-dotfiles` — edit the Nix source, never the copy. Their values (base16 colors, fonts) come from Stylix and cannot be written as static files. Genuinely non-Nix targets (`dot_glzr/`, `AppData/`, scoop, winget, Jupyter) stay hand-maintained
- GC runs weekly automatically, once per host (NixOS: system `programs.nh.clean`; macOS: `nix.gc`; standalone home-manager: HM `programs.nh.clean`); manual GC rarely needed

## Orca workflow

- Orca owns task worktrees, agent sessions, status, and review. Use the installed
  `orca-cli` skill and the live `orca skills get orca-cli` guide for those actions.
  Use ordinary Git for diffs and commits within the selected checkout.
- Load the live `orchestration` guide when supervising multiple agents. A task
  handoff and a supervised task are different workflows; keep the user's intent.
- Before editing, inspect the current directory, branch, and dirty files. Keep
  the existing workspace for ongoing work. New independent tasks can use Orca
  worktrees; uncommitted edits are not inherited by a new checkout.
- Record meaningful progress on the current Orca worktree card when available.
  Keep orchestration/session state in Orca, not in a second dotfiles task system.
- Run commands in the selected worktree. `.direnv` and virtual environments belong
  to that checkout; do not share them between worktrees.
- Keep credentials and mutable Orca/agent settings outside Git and the Nix store.
  Orca's desktop and an SSH execution host have separate skill installations.
- Before using an Orca command, resolve `ORCA_CLI_COMMAND` if provided, otherwise
  the installed Orca executable, and consult its live help. A relay can control
  the desktop but cannot install skills into the SSH host through `orca skills install`.

## Setup and verification

Orca runs the setup hook in `orca.yaml` for new worktrees. It prepares the Nix
shell and Git hooks. It does not apply Home Manager or system configurations.

```sh
nix develop --command pre-commit install --hook-type pre-commit --hook-type pre-push
nix develop --command just verify
```

`just verify` checks formatting/lint, evaluates every host on every configured
platform, runs regression tests, and compares generated Chezmoi files. Run focused
tests while editing; run the complete verification before handing off code/config
changes. For documentation-only changes, check changed links and instructions.
Report what ran and any checks not run. An evaluation is not a full system build.

## Commits

Commit when requested, stage explicit paths for the requested work, and preserve
unrelated changes. Group changes by purpose, run the relevant verification,
and review the staged diff before committing.
Use conventional commit subjects. Do not add a fixed model name or attribution
trailer unless the user requests it. Do not push, deploy, or publish skill/artifact
links merely because implementation is complete.
