# CLAUDE.md

Nix Flakes dotfiles. NixOS / macOS (nix-darwin) / standalone home-manager.

## Commands

```bash
# Apply (nh preferred; raw fallback: sudo nixos-rebuild switch --flake .#<host>)
nh os switch . -H <host>          # NixOS: kt-thinkpad, kt-proxmox, kt-wsl
nh darwin switch . -H <host>      # macOS: kt-mac-studio, kt-mac-mini, kt-mba
nh home switch . -c <user>@<host> # home-manager: ktaga@kt-ubuntu, jovyan@kt-ubuntu

# Bootstrap (fresh host, no nh yet)
nix run nixpkgs#nh -- darwin switch . -H kt-mac-studio

# Maintenance
nix flake update                              # Update inputs
nix flake check                               # Validate
nh clean all --keep 5 --keep-since 7d         # GC (user + system)
nix develop                                   # Dev shell
```

## Code Style

- Nix Flakes syntax; format with `nixfmt`, lint with `statix` and `deadnix`
- Packages are added via Nix, not `brew install` / `apt-get`
- Theming via Stylix — color changes go in `lib/shonan.yaml`, not per-app configs

## Architecture

- `home/common/` — cross-platform shared configs (edit here for universal tools)
- `home/mac/`, `home/nixos/` — platform-specific overlays
- `hosts/` — per-machine definitions
- `lib/` — shared modules (devshell, theming, locale, users)
- Details: @docs/architecture.md

## Cautions

- Determine target platform (NixOS vs macOS) before suggesting system-level changes
- `flake.nix` / `flake.lock` changes affect all hosts — verify carefully
- Chezmoi syncs some configs (nvim, wezterm) for non-Nix environments; don't remove those hooks
- GC runs weekly automatically (`nix.gc` + `programs.nh.clean`); manual GC rarely needed
