# Architecture

## Flake Structure

- **flake.nix**: Entry point — defines all system/home configurations and inputs
- **nixpkgs (unstable)**: Package repository
- **Inputs**: home-manager, nix-darwin, nixos-hardware, nixos-wsl, stylix, hyprland, hyprpanel, xremap, wezterm, nixvim, nix-colors

## Directory Layout

```
├── flake.nix                 # Main Flake configuration
├── lib/                      # Shared Nix modules
│   ├── devshell.nix         # Development shell (Python 3.13, uv, Nix tools, build tools)
│   ├── nixos-common.nix     # Common NixOS settings (nix-ld, 1Password, etc.)
│   ├── japanese-locale.nix  # Japanese locale settings
│   ├── wsl-common.nix       # WSL-specific settings
│   ├── users.nix            # User account definitions
│   ├── stylix.nix           # Stylix theming (home-manager standalone)
│   ├── stylix-nixos.nix     # Stylix theming (NixOS)
│   ├── stylix-darwin.nix    # Stylix theming (macOS)
│   └── shonan.yaml          # Shonan base16 color scheme definition
├── home/                     # Home-manager configurations
│   ├── common/              # Cross-platform shared
│   │   ├── cli/            # CLI: git, neovim, shells
│   │   └── gui/            # GUI: terminals, apps
│   ├── mac/                # macOS-specific home configurations
│   └── nixos/              # NixOS-specific home configurations
└── hosts/                  # Host-specific system configurations
    ├── kt-proxmox/        # Proxmox VM (NixOS)
    ├── kt-thinkpad/       # ThinkPad (NixOS)
    ├── kt-wsl/            # WSL (NixOS)
    ├── kt-ubuntu/         # Ubuntu (standalone home-manager)
    ├── kt-mba/            # MacBook Air
    └── mac/               # Mac Studio / Mac Mini shared
```

## Design Principles

- **Shared Common Base**: `home/common/` is used across all platforms
- **Platform-Specific Overlays**: `home/mac/` and `home/nixos/` extend the common base
- **Host-Specific Settings**: `hosts/` directories for individual machine configs
- **Unified Theming**: Stylix applies the Shonan color scheme (custom base16) everywhere
- **Reproducible Builds**: `flake.lock` pins all input versions

## Key Components

| Component | Purpose |
|-----------|---------|
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
