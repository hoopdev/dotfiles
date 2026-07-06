# CLAUDE.md

Nix Flakes dotfiles. NixOS / macOS (nix-darwin) / standalone home-manager.

Apply changes with `nh {os,darwin,home} switch` — full command reference (apply, bootstrap, maintenance, dev fleet tool) in [docs/commands.md](docs/commands.md).

## Design Guidelines

- Add packages via Nix, never `brew install` / `apt-get`
- Theming is unified via Stylix — color changes go in `lib/shonan.yaml`, not per-app configs
- Cross-platform config lives in `home/common/`; `home/mac/` and `home/nixos/` are overlays that extend it
- Hosts are auto-discovered from `hosts/<name>/meta.nix` — adding one needs no edits to `flake-modules/*.nix`
- Format with `nixfmt`; lint with `statix` and `deadnix`
- Directory layout, key components, and design principles: @docs/architecture.md

## Cautions

- Determine target platform (NixOS vs macOS) before suggesting system-level changes
- `flake.nix` / `flake.lock` changes affect all hosts — verify carefully
- Chezmoi sync: an activation hook in `home/common/cli/neovim.nix` copies `init.lua` into `dot_config/nvim/` on rebuild — don't remove it. Other Chezmoi files (e.g. `dot_config/wezterm/`) are hand-maintained, not auto-generated
- GC runs weekly automatically (`nix.gc` + `programs.nh.clean`); manual GC rarely needed
