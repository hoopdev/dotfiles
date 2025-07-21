# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

This is a dotfiles repository that manages cross-platform system configurations using Nix Flakes and Chezmoi. The repository supports NixOS, macOS (via nix-darwin), and Windows environments with a unified approach to dotfile management.

## Key Commands

### NixOS Systems
```bash
# Build and apply NixOS configuration for ThinkPad
sudo nixos-rebuild switch --flake .#kt-thinkpad

# Build and apply NixOS configuration for Proxmox
sudo nixos-rebuild switch --flake .#kt-prox-nix

# Build and apply WSL NixOS configuration
sudo nixos-rebuild switch --flake .#kt-wsl-nix

# Build and apply WSL configuration
sudo nixos-rebuild switch --flake .#kt-wsl
```

### macOS Systems
```bash
# Build and apply macOS configuration for Mac Studio
darwin-rebuild switch --flake .#kt-mac-studio

# Build and apply macOS configuration for Mac Mini
darwin-rebuild switch --flake .#kt-mac-mini

# Build and apply macOS configuration for MacBook Air
darwin-rebuild switch --flake .#kt-mba
```

### Development Commands
```bash
# Update all flake inputs
nix flake update

# Check flake configuration
nix flake check

# Show flake outputs
nix flake show

# Garbage collection (automatic, but can be run manually)
nix-collect-garbage -d

# Development environments
nix develop                     # Default development shell (Nix tools)
nix develop .#python            # Python development shell with uv
```

## Architecture

### Flake-Based Configuration
- **flake.nix**: Main entry point defining system configurations, inputs, and outputs
- Uses unstable nixpkgs channel for latest packages
- Integrates home-manager for user environment management
- Platform-specific builds with shared configurations

### Directory Structure
```
├── flake.nix                 # Main Nix Flake configuration
├── home/                     # Home-manager configurations
│   ├── common/              # Shared configurations across platforms
│   │   ├── cli/            # Command-line tools (git, neovim, shells)
│   │   └── gui/            # GUI applications and terminals
│   ├── mac/                # macOS-specific home configurations  
│   └── nixos/              # NixOS-specific home configurations
└── hosts/                  # Host-specific system configurations
    ├── kt-prox-nix/       # Proxmox NixOS configuration
    ├── kt-thinkpad/       # ThinkPad NixOS configuration
    ├── kt-wsl-nix/        # WSL NixOS configuration
    ├── kt-wsl/            # WSL NixOS configuration
    └── mac/               # macOS system configuration
```

### Configuration Philosophy
- **Shared Common Base**: Core configurations in `home/common/` used across all platforms
- **Platform-Specific Overlays**: Platform-specific configurations extend the common base
- **Host-Specific Settings**: Individual machine configurations in `hosts/` directories
- **Reproducible Builds**: Flake.lock ensures consistent package versions across rebuilds

### Key Components
- **Home-manager**: User environment and dotfile management
- **Nix-darwin**: macOS system-level configuration management
- **NixOS-WSL**: Windows Subsystem for Linux integration
- **Hardware-specific modules**: Lenovo ThinkPad optimizations via nixos-hardware
- **Development tools**: Neovim (via nixvim), Git, shell configurations (Nushell, Zsh)
- **Window managers**: Hyprland for NixOS, AeroSpace for macOS
- **Terminal**: WezTerm with consistent configuration across platforms
- **System optimizations**: DS_Store prevention, Touch ID for sudo, keyboard remapping

### Flake Inputs
- **nixpkgs**: Main package repository (unstable channel)
- **home-manager**: User environment management
- **nix-darwin**: macOS system configuration
- **nixos-hardware**: Hardware-specific optimizations
- **hyprland**: Wayland compositor for NixOS
- **wezterm**: Terminal emulator with Nix packaging
- **nixvim**: Neovim configuration in Nix
- **nix-colors**: Color scheme management

### User Configuration
- Primary user: `ktaga`
- Editor: `nvim` (Neovim)
- Shell: Nushell and Zsh support
- Color scheme: Nord (via nix-colors)
- Automatic garbage collection enabled (7-day retention)