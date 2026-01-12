# AGENTS.md

This file provides context, architectural details, and operational guidelines for AI agents (Opencode, Codex, Claude, etc.) working with this repository.

## Project Overview

This is a **dotfiles repository** managing cross-platform system configurations using **Nix Flakes** and **Chezmoi**.
It supports:
- **NixOS** (ThinkPad, Proxmox, WSL)
- **macOS** (via nix-darwin)
- **Linux/Ubuntu** (via standalone Home Manager)

The goal is to provide a unified, reproducible configuration across all environments.

## Architecture & Tech Stack

- **Core Technology:** Nix, Nix Flakes
- **User Environment:** Home Manager
- **System Configuration:**
  - NixOS (Linux)
  - Nix-darwin (macOS)
  - NixOS-WSL (Windows Subsystem for Linux)
- **Secret Management:** (Implied context: likely handled via external tools or manual setup, though not explicitly detailed in CLAUDE.md besides 1Password mention)
- **Editor:** Neovim (via nixvim)
- **Shells:** Nushell, Zsh
- **Window Management:** Hyprland (NixOS), AeroSpace (macOS)
- **Terminal:** WezTerm

### Directory Structure

| Path | Description |
|------|-------------|
| `flake.nix` | **Entry Point.** Defines system configurations, inputs, and outputs. |
| `lib/` | Shared Nix modules (devshell, locales, common settings). |
| `home/` | **Home Manager** configurations. |
| `home/common/` | Shared configs across all platforms (CLI, GUI tools). |
| `home/mac/` | macOS-specific home configs. |
| `home/nixos/` | NixOS-specific home configs. |
| `hosts/` | **Host-specific** configurations (machine definitions). |

### Key Hosts

- **kt-mba**: MacBook Air (macOS)
- **kt-mac-studio / kt-mac-mini**: Desktop Macs
- **kt-thinkpad**: Lenovo ThinkPad (NixOS)
- **kt-proxmox**: Proxmox VM (NixOS)
- **kt-wsl**: WSL environment (NixOS)
- **kt-ubuntu**: Ubuntu environment (Standalone Home Manager)

## Common Operations

### Applying Configurations

**NixOS:**
```bash
sudo nixos-rebuild switch --flake .#<host>
# e.g., sudo nixos-rebuild switch --flake .#kt-thinkpad
```

**macOS:**
```bash
darwin-rebuild switch --flake .#<host>
# e.g., darwin-rebuild switch --flake .#kt-mba
```

**Standalone Home Manager (Ubuntu/Non-NixOS):**
```bash
nix run home-manager/master -- switch --flake .#<user>@<host>
# e.g., nix run home-manager/master -- switch --flake .#ktaga@kt-ubuntu
```

### Development Environment

To enter the reproducible development shell (includes Python, Nix tools, git, etc.):
```bash
nix develop
```

### Flake Maintenance

```bash
nix flake update   # Update inputs
nix flake check    # Check configuration validity
nix flake show     # Show outputs
```

## Guidelines for Agents

1.  **Context Awareness:** Always determine the target platform (NixOS vs macOS) before suggesting system-level changes.
2.  **Modularity:** Prefer editing files in `home/common/` for tools used everywhere (like Neovim or git). Use `hosts/` or platform-specific folders (`home/mac/`, `home/nixos/`) only for environment-specific settings.
3.  **Nix Syntax:** This project uses Nix Flakes. Ensure syntax compliance.
4.  **Reproducibility:** Do not suggest ad-hoc `brew install` or `apt-get install` commands unless strictly necessary for debugging. The primary installation method is adding packages to the Nix configuration.
5.  **Safety:** Verify `flake.nix` or `flake.lock` changes carefully as they affect the build integrity.
