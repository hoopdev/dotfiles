# Architecture

## Flake Structure

- **flake.nix**: Thin entry point — calls `flake-parts.lib.mkFlake` and imports modules from `flake-modules/`
- **flake-modules/**: Per-subsystem flake-parts modules (the real outputs live here)
- **Inputs**: flake-parts, treefmt-nix, home-manager, nix-darwin, nixos-hardware, nixos-wsl, stylix, hyprland, hyprpanel, xremap, wezterm, nixvim

## Directory Layout

```
├── flake.nix                  # Thin entry — mkFlake { imports = [...]; }
├── flake-modules/             # flake-parts modules
│   ├── shared.nix            # Helpers + auto-discovered hosts attrset
│   ├── modules.nix           # flake.nixosModules.* exports
│   ├── nixos.nix             # nixosConfigurations (auto-built from meta.nix)
│   ├── darwin.nix            # darwinConfigurations
│   ├── home.nix              # homeConfigurations (standalone home-manager)
│   └── per-system.nix        # devShells, formatter, treefmt
├── modules/nixos/             # Self-exported NixOS modules
│   ├── default.nix           # Kitchen-sink (imports the three below)
│   ├── nix-ld.nix            # nix-ld for unpatched binaries
│   ├── onepassword.nix       # 1Password CLI + GUI
│   └── nix-settings.nix      # Nix daemon settings + Hyprland cache
├── lib/                       # Shared Nix utilities (non-module)
│   ├── devshell.nix          # Development shell (Python 3.13, uv, Nix tools, build tools)
│   ├── japanese-locale.nix   # Japanese locale settings
│   ├── wsl-common.nix        # WSL-specific settings
│   ├── users.nix             # User account definitions
│   ├── stylix.nix            # Unified Stylix theming (NixOS / darwin / home-manager)
│   └── shonan.yaml           # Shonan base16 color scheme definition
├── home/                      # Home-manager configurations
│   ├── common/               # Cross-platform shared
│   │   ├── cli/             # CLI: git, neovim, shells
│   │   └── gui/             # GUI: terminals, apps
│   ├── mac/                 # macOS-specific home configurations
│   └── nixos/               # NixOS-specific home configurations
└── hosts/                    # Host-specific system configurations
    ├── kt-proxmox/          # Proxmox VM (NixOS)
    ├── kt-thinkpad/         # ThinkPad (NixOS)
    ├── kt-wsl/              # WSL (NixOS)
    ├── kt-ubuntu/           # Ubuntu (standalone home-manager)
    ├── kt-mba/              # MacBook Air
    ├── kt-mac-studio/       # Mac Studio (meta.nix only — shares mac/)
    ├── kt-mac-mini/         # Mac Mini (meta.nix only — shares mac/)
    └── mac/                 # Shared Mac Studio / Mac Mini config
```

Each `hosts/<name>/meta.nix` declares `{ type, system?, users?, configFrom? }`; `flake-modules/shared.nix` reads the directory and dispatches to the matching subsystem module.

## Design Principles

- **Modular Flake**: `flake-parts` splits flake outputs across `flake-modules/`
- **Auto-Discovered Hosts**: New hosts appear by adding `hosts/<name>/meta.nix` (no edits to `flake-modules/*.nix` needed)
- **Composable NixOS Modules**: `modules/nixos/{nix-ld,onepassword,nix-settings}.nix` are exported via `flake.nixosModules` so hosts can opt in à la carte
- **Shared Common Base**: `home/common/` is used across all platforms
- **Platform-Specific Overlays**: `home/mac/` and `home/nixos/` extend the common base
- **Unified Theming**: Stylix applies the Shonan color scheme (custom base16) everywhere
- **Reproducible Builds**: `flake.lock` pins all input versions

## Key Components

| Component | Purpose |
|-----------|---------|
| flake-parts | Modular flake outputs |
| treefmt-nix | `nix fmt` integration (nixfmt + statix + deadnix) |
| Home-manager | User environment and dotfile management |
| Nix-darwin | macOS system-level configuration |
| NixOS-WSL | WSL integration |
| Stylix | Unified theming (Shonan base16 color scheme) |
| nixos-hardware | ThinkPad hardware optimizations |
| Neovim (nixvim + lazy.nvim) | Editor — see [docs/neovim.md](neovim.md) |
| Hyprland + HyprPanel | Wayland compositor + panel (NixOS) |
| AeroSpace | Tiling window manager (macOS) |
| WezTerm | Terminal emulator (cross-platform) |
| xremap / Karabiner | Key remapping (NixOS / macOS) |
| nix-ld | Run unpatched binaries on NixOS (uv, Python wheels) |
| 1Password CLI | Secret management |
| Chezmoi | Supplementary dotfile sync for non-Nix environments |

## Development Shell (`nix develop`)

Defined in `lib/devshell.nix`:

| Category | Tools |
|----------|-------|
| Python | Python 3.13, uv, ruff, mypy, pytest |
| Nix | nixfmt, statix, deadnix |
| Build | gcc, pkg-config, ninja, meson |
| Utilities | git, curl, wget, htop, tree, pre-commit, just |
| Shell | zsh, starship, direnv, nix-direnv |

Platform-specific libraries are included automatically (Linux: glibc, X11; macOS: system frameworks).

## User Configuration

- Primary user: `ktaga`
- Editor: Neovim
- Shells: Nushell, Zsh
- Color scheme: Shonan (custom base16, via Stylix)
- GC: weekly automatic (system via `nix.gc`, user via `programs.nh.clean`)
