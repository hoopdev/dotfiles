# KT's Dotfiles

Dotfiles managed by **Nix/Home-Manager** with **Chezmoi** for cross-platform support.

## System Overview

**Nix (Primary)**: System configs for NixOS/macOS/WSL via home-manager
**Chezmoi (Secondary)**: Cross-platform dotfile distribution, auto-synced from Nix

### Auto-Synced Configs
On every Nix rebuild, these files are automatically copied to Chezmoi:
- `~/.config/nvim/init.lua` ← `home/common/cli/init.lua`
- `~/.config/wezterm/wezterm.lua` ← `home/common/gui/terminals/wezterm.lua`

## Chezmoi Quick Start

```bash
# Initialize chezmoi with this repo
chezmoi init https://github.com/yourusername/dotfiles

# Preview changes
chezmoi diff

# Apply dotfiles
chezmoi apply

# Update from repo
chezmoi update
```

## Platform Support

- **NixOS**: Full system configuration
- **macOS**: via nix-darwin
- **WSL**: NixOS-WSL
- **Windows/Other**: via Chezmoi only

## Prerequisites

- Nix package manager
- Git
- For macOS: nix-darwin
- For NixOS: NixOS installed
- For WSL: Windows Subsystem for Linux with NixOS

## Installation

### NixOS Setup

1. Clone the repository
2. Move to an appropriate hostname configuration:
   ```bash
   # For ThinkPad configuration
   sudo nixos-rebuild switch --flake .#kt-thinkpad
   ```

### macOS Setup

1. Install Nix and nix-darwin
2. Clone the repository
3. Build the configuration:
   ```bash
   # For Mac Studio configuration
   darwin-rebuild switch --flake .#kt-mac-studio
   ```

### WSL Setup

1. Install NixOS in WSL using the [NixOS-WSL project](https://github.com/nix-community/NixOS-WSL)
   - Simply click and install from the project's installation options
2. Clone the repository
3. Apply the configuration:
   ```bash
   sudo nixos-rebuild switch --flake .#kt-wsl-nix
   ```

### Windows Setup

1. Install Chezmoi
2. Clone the repository
3. Apply Windows-specific configurations:
   ```powershell
   chezmoi init
   chezmoi apply
   ```

This will set up Windows-specific configurations in the appropriate locations (AppData, etc.).

## Repository Structure

```
.
├── flake.nix              # Main Nix Flake configuration
├── home/                  # Home-manager configurations
│   ├── common/           # Shared configurations
│   │   ├── cli/         # Command-line tools and configs
│   │   └── gui/         # GUI applications and configs
│   ├── mac/             # macOS-specific configurations
│   └── nixos/           # NixOS-specific configurations
└── hosts/               # Host-specific configurations
    ├── kt-prox-nix/    # Proxmox NixOS configuration
    ├── kt-thinkpad/    # ThinkPad NixOS configuration
    ├── kt-wsl-nix/     # WSL NixOS configuration
    └── mac/            # macOS configuration
```

## Features

### Common Features

- Shell environments (Nushell, Zsh)
- Development tools
- Terminal emulators (WezTerm)
- Git configuration
- Neovim setup

### Platform-Specific Features

#### NixOS
- Hyprland window manager
- System-level configuration
- Hardware-specific optimizations

#### macOS
- nix-darwin integration
- macOS-specific tools and configurations

#### WSL
- WSL-optimized configuration
- Windows integration features


## License

Feel free to use and modify this configuration as you see fit.
