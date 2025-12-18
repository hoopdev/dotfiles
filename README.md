# KT's Dotfiles

Cross-platform system configuration management using Nix Flakes. Unified management of NixOS, macOS (nix-darwin), and WSL environments.

## Features

- **Declarative Configuration**: Reproducible builds with Nix Flakes
- **Cross-Platform**: Manage NixOS / macOS / WSL with a single flake
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

# NixOS
sudo nixos-rebuild switch --flake .#<hostname>

# macOS
darwin-rebuild switch --flake .#<hostname>
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
| `kt-thinkpad` | ThinkPad (Hyprland) | `sudo nixos-rebuild switch --flake .#kt-thinkpad` |
| `kt-prox-nix` | Proxmox VM | `sudo nixos-rebuild switch --flake .#kt-prox-nix` |
| `kt-wsl` | WSL2 | `sudo nixos-rebuild switch --flake .#kt-wsl` |
| `kt-ubuntu` | Ubuntu/VM (CLI only) | `sudo nixos-rebuild switch --flake .#kt-ubuntu` |

### macOS (nix-darwin)

| Hostname | Description | Command |
|----------|-------------|---------|
| `kt-mac-studio` | Mac Studio | `darwin-rebuild switch --flake .#kt-mac-studio` |
| `kt-mac-mini` | Mac Mini | `darwin-rebuild switch --flake .#kt-mac-mini` |
| `kt-mba` | MacBook Air | `darwin-rebuild switch --flake .#kt-mba` |

### Ubuntu/Linux (home-manager only)

For non-NixOS Linux systems (Ubuntu, Debian, etc.), you can use home-manager standalone to manage your user environment without requiring root access or full system configuration.

| Configuration | Description | Build Command | Apply Command |
|---------------|-------------|---------------|---------------|
| `ktaga@kt-ubuntu` | User environment for ktaga | `nix build .#homeConfigurations."ktaga@kt-ubuntu".activationPackage` | `./result/activate` |
| `jovyan@kt-ubuntu` | User environment for jovyan | `nix build .#homeConfigurations."jovyan@kt-ubuntu".activationPackage` | `./result/activate` |

**Quick Start:**

```bash
# 1. Install Nix (if not already installed)
curl -L https://nixos.org/nix/install | sh

# 2. Enable flakes
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# 3. Clone and build
git clone https://github.com/hoopdev/dotfiles.git
cd dotfiles
nix build .#homeConfigurations."$USER@kt-ubuntu".activationPackage --out-link result

# 4. Apply configuration
./result/activate
```

**What gets installed:**
- Neovim (with nixvim configuration)
- Git, GitHub CLI (gh)
- Modern CLI tools (bat, zellij, eza, ripgrep, fd)
- Shell environment (Zsh, Nushell, Starship prompt)
- Nord color scheme
- All dotfiles and configurations from `home/common/cli/`

## Development Environment (`nix develop`)

A unified development environment is available via `nix develop`.

### Included Tools

| Category | Tools |
|----------|-------|
| **Python** | Python 3.13, uv, ruff, mypy, pytest |
| **Nix** | nixfmt-rfc-style, statix, deadnix |
| **Build** | gcc, pkg-config, ninja, meson |
| **Utilities** | git, curl, wget, htop, tree, pre-commit |
| **Shell** | zsh, starship, direnv, zoxide |

### Features

- **Cross-Platform**: Supports both Linux and macOS
- **nix-ld Support** (Linux): Run unpatched binaries like uv and Python wheels
- **Common Aliases**: Auto-loaded from `home/common/cli/shell/aliases.nix`
- **Starship Prompt**: Unified shell prompt across environments

## Flake Management Commands

```bash
# Update all flake inputs
nix flake update

# Check flake configuration
nix flake check

# Show flake outputs
nix flake show

# Garbage collection (manual)
nix-collect-garbage -d
```

## Directory Structure

```
.
├── flake.nix                 # Main Flake configuration
├── flake.lock               # Lock file (reproducibility)
├── lib/                     # Shared Nix modules
│   ├── devshell.nix        # Development shell definition
│   ├── nixos-common.nix    # Common NixOS settings (nix-ld, 1Password, etc.)
│   ├── japanese-locale.nix # Japanese locale settings
│   └── wsl-common.nix      # WSL-specific settings
├── home/                    # home-manager configurations
│   ├── common/             # Cross-platform shared
│   │   ├── cli/           # CLI: git, neovim, shells
│   │   └── gui/           # GUI: terminals, apps
│   ├── mac/               # macOS-specific
│   └── nixos/             # NixOS-specific
└── hosts/                  # Host-specific configurations
    ├── kt-thinkpad/       # ThinkPad
    ├── kt-prox-nix/       # Proxmox
    ├── kt-wsl/            # WSL
    ├── kt-ubuntu/         # Ubuntu (home-manager only)
    ├── kt-mba/            # MacBook Air
    └── mac/               # Mac Studio/Mini shared
```

## Key Components

### Flake Inputs

| Input | Purpose |
|-------|---------|
| nixpkgs (unstable) | Main package repository |
| home-manager | User environment management |
| nix-darwin | macOS system configuration |
| nixos-hardware | Hardware optimizations |
| nixvim | Neovim configuration |
| hyprland | Wayland compositor (NixOS) |
| wezterm | Terminal emulator |
| nix-colors | Color scheme (Nord) |

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
- Neovim (nixvim)
- Nushell / Zsh
- Starship prompt
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
