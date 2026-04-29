# KT's Dotfiles

Cross-platform system configuration management using Nix Flakes. Unified management of NixOS, macOS (nix-darwin), and standalone home-manager environments.

## Features

- **Declarative Configuration**: Reproducible builds with Nix Flakes
- **Cross-Platform**: Manage NixOS / macOS / Ubuntu with a single flake
- **Unified Theming**: Shonan color scheme (custom base16) applied everywhere via Stylix
- **Unified User Environment**: Dotfile management via home-manager
- **Development Shell**: Consistent development environment with `nix develop`

## Quick Start

### Prerequisites

- [Nix](https://nixos.org/download.html) (with Flakes enabled)
- Git

### Apply System Configuration

```bash
# Clone the repository
git clone https://github.com/hoopdev/dotfiles.git
cd dotfiles

# NixOS (preferred: nh)
nh os switch . -H <hostname>

# macOS (preferred: nh)
nh darwin switch . -H <hostname>

# Standalone home-manager (preferred: nh)
nh home switch . -c <user>@<hostname>

# Bootstrap on a fresh host (no nh yet)
nix run nixpkgs#nh -- darwin switch . -H kt-mac-studio   # or `os` / `home`
```

### Start Development Shell

```bash
# Enter development environment
nix develop

# From a specific project directory
cd ~/projects/myapp && nix develop ~/git/dotfiles
```

## System Configurations

### NixOS

| Hostname | Description | Command |
|----------|-------------|---------|
| `kt-thinkpad` | ThinkPad (Hyprland) | `nh os switch . -H kt-thinkpad` |
| `kt-proxmox` | Proxmox VM | `nh os switch . -H kt-proxmox` |
| `kt-wsl` | WSL2 | `nh os switch . -H kt-wsl` |

### macOS (nix-darwin)

| Hostname | Description | Command |
|----------|-------------|---------|
| `kt-mac-studio` | Mac Studio | `nh darwin switch . -H kt-mac-studio` |
| `kt-mac-mini` | Mac Mini | `nh darwin switch . -H kt-mac-mini` |
| `kt-mba` | MacBook Air | `nh darwin switch . -H kt-mba` |

### Ubuntu/Linux (standalone home-manager)

For non-NixOS Linux systems (Ubuntu, Debian, etc.), you can use home-manager standalone to manage your user environment without requiring root access or full system configuration.

| Configuration | Command |
|---------------|---------|
| `ktaga@kt-ubuntu` | `nh home switch . -c ktaga@kt-ubuntu` |
| `jovyan@kt-ubuntu` | `nh home switch . -c jovyan@kt-ubuntu` |

**Quick Start:**

```bash
# 1. Install Nix (if not already installed)
curl -L https://nixos.org/nix/install | sh

# 2. Enable flakes
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# 3. Clone and apply
git clone https://github.com/hoopdev/dotfiles.git
cd dotfiles
nix run nixpkgs#nh -- home switch . -c $USER@kt-ubuntu

# 4. Update font cache after first install
fc-cache -fv
```

**What gets installed:**
- Neovim (with nixvim + lazy.nvim configuration)
- Git, GitHub CLI (gh)
- Modern CLI tools (bat, zellij, eza, ripgrep, fd)
- Shell environment (Zsh, Nushell, Starship prompt)
- Shonan color scheme (via Stylix)
- All dotfiles and configurations from `home/common/cli/`

## Development Environment (`nix develop`)

A unified development environment is available via `nix develop`.

### Included Tools

| Category | Tools |
|----------|-------|
| **Python** | Python 3.13, uv, ruff, mypy, pytest |
| **Nix** | nixfmt, statix, deadnix |
| **Build** | gcc, pkg-config, ninja, meson |
| **Utilities** | git, curl, wget, htop, tree, pre-commit, just |
| **Shell** | zsh, starship, direnv, nix-direnv |

### Features

- **Cross-Platform**: Supports both Linux and macOS
- **nix-ld Support** (Linux): Run unpatched binaries like uv and Python wheels
- **Starship Prompt**: Unified shell prompt across environments

## Maintenance

### Common Commands

| Command | Description |
|---------|-------------|
| `nix flake update` | Update all flake inputs |
| `nix flake check` | Check flake for errors (incl. format drift) |
| `nix flake show` | Show flake outputs |
| `nix fmt` | Format all `.nix` files (nixfmt + statix + deadnix) |
| `nh clean all --keep 5 --keep-since 7d` | Garbage collection (user + system) |
| `nh clean user --keep 5 --keep-since 7d` | Garbage collection (user-only) |

### Formatting & Linting

`nix fmt` runs nixfmt + statix + deadnix via [treefmt-nix](https://github.com/numtide/treefmt-nix). `nix flake check` also runs the treefmt drift check, so unformatted code fails CI-style verification.

## Directory Structure

```
.
├── flake.nix                  # Thin entry point — calls flake-parts.lib.mkFlake
├── flake.lock                 # Lock file (reproducibility)
├── flake-modules/             # flake-parts modules (the real flake outputs live here)
│   ├── shared.nix            # Shared helpers + auto-discovered hosts attrset
│   ├── modules.nix           # flake.nixosModules.* exports
│   ├── nixos.nix             # nixosConfigurations (auto-built from hosts/*/meta.nix)
│   ├── darwin.nix            # darwinConfigurations
│   ├── home.nix              # homeConfigurations (standalone home-manager)
│   └── per-system.nix        # devShells, formatter, treefmt
├── modules/nixos/             # Self-exported NixOS modules
│   ├── default.nix           # Aggregate (imports the three below)
│   ├── nix-ld.nix            # nix-ld for unpatched binaries
│   ├── onepassword.nix       # 1Password CLI + GUI
│   └── nix-settings.nix      # Nix daemon settings + Hyprland cache
├── lib/                       # Shared Nix utilities (non-module)
│   ├── devshell.nix          # Development shell definition
│   ├── japanese-locale.nix   # Japanese locale settings
│   ├── wsl-common.nix        # WSL-specific settings
│   ├── users.nix             # User account definitions
│   ├── stylix.nix            # Unified Stylix theming (NixOS / darwin / home-manager)
│   └── shonan.yaml           # Shonan base16 color scheme
├── home/                      # home-manager configurations
│   ├── common/               # Cross-platform shared
│   │   ├── cli/             # CLI: git, neovim, shells
│   │   └── gui/             # GUI: terminals, apps
│   ├── mac/                 # macOS-specific
│   └── nixos/               # NixOS-specific
└── hosts/                    # Host-specific configurations
    ├── kt-thinkpad/         # ThinkPad (NixOS) — meta.nix + configuration.nix + home.nix
    ├── kt-proxmox/          # Proxmox VM (NixOS)
    ├── kt-wsl/              # WSL (NixOS)
    ├── kt-ubuntu/           # Ubuntu (standalone home-manager)
    ├── kt-mba/              # MacBook Air
    ├── kt-mac-studio/       # Mac Studio (meta.nix only — shares mac/)
    ├── kt-mac-mini/         # Mac Mini (meta.nix only — shares mac/)
    └── mac/                 # Shared Mac Studio/Mini config
```

Each `hosts/<name>/meta.nix` declares `{ type, system?, users?, configFrom? }`; new hosts are picked up automatically by `flake-modules/shared.nix`.

## Key Components

### Flake Inputs

| Input | Purpose |
|-------|---------|
| nixpkgs (unstable) | Main package repository |
| home-manager | User environment management |
| nix-darwin | macOS system configuration |
| nixos-hardware | Hardware optimizations |
| nixos-wsl | WSL integration |
| stylix | Unified theming (base16) |
| nixvim | Neovim configuration |
| hyprland | Wayland compositor (NixOS) |
| hyprpanel | Status panel for Hyprland |
| xremap | Key remapping (NixOS) |
| wezterm | Terminal emulator |
| flake-parts | Modular flake structure |
| treefmt-nix | Multi-formatter integration (nixfmt + statix + deadnix) |

### Platform-Specific Features

**NixOS**
- Hyprland + HyprPanel (Wayland)
- xremap (key remapping)
- nix-ld (unpatched binary support)
- nixos-hardware optimizations

**macOS**
- AeroSpace (tiling window manager)
- Touch ID for sudo
- DS_Store auto-cleanup
- Karabiner-Elements (key remapping)

**Common**
- WezTerm (unified terminal)
- Neovim (nixvim + lazy.nvim)
- Nushell / Zsh
- Starship prompt
- Shonan color scheme (via Stylix)
- 1Password CLI integration

## Chezmoi

Chezmoi is supported as a supplementary tool for environments where Nix is not available.

```bash
# Initialize
chezmoi init https://github.com/hoopdev/dotfiles

# Apply
chezmoi apply
```

Some configuration files (nvim, wezterm) are automatically synced to the Chezmoi directory during Nix rebuilds.

## License

Feel free to use and modify as you see fit.
