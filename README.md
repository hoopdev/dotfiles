# KT's Dotfiles

This repository manages my dotfiles using two distinct systems:

## 1. Nix Configuration

### Purpose and Scope
- System-level package management and configuration
- Development environment setup
- Cross-platform environment management (NixOS, macOS, WSL)
- User environment configuration via home-manager

### Key Components
- Flake-based configuration for reproducible builds
- Home-manager for user environment management
- Platform-specific configurations
- Shared common configurations

### Supported Platforms
- **NixOS**: Full system configuration for Linux machines
- **macOS**: System configuration via nix-darwin
- **WSL**: NixOS configuration optimized for Windows Subsystem for Linux

## 2. Windows Configuration via Chezmoi

### Purpose and Scope
- Windows-specific application configurations
- AppData directory management
- Windows-only tools and settings

### Managed Configurations
- Terminal emulator settings (Alacritty)
- Shell configurations for Windows
- Window manager settings (GlazeWM)
- Other Windows-specific application configs

### Location
- AppData/Roaming configurations
- Windows-specific dotfiles
- Tool-specific settings for Windows environment

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

1. Set up NixOS WSL
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
