# Justfile

# List available recipes
default:
    @just --list

# Update flake inputs
update:
    nix flake update

# Check flake for errors
check:
    nix flake check

# Format all nix files
fmt:
    nixfmt .

# Lint nix files
lint:
    nix develop . --command statix check .
    nix develop . --command deadnix .

# Switch configuration (auto-detects OS and hostname)
switch host="":
    #!/usr/bin/env bash
    TARGET_HOST="{{host}}"
    # Clean up host if passed as host=value (e.g. accidental positional arg)
    TARGET_HOST=${TARGET_HOST#host=}
    if [ -z "$TARGET_HOST" ]; then
        TARGET_HOST=$(hostname | cut -d. -f1)
    fi
    
    OS=$(uname -s)
    if [ "$OS" = "Darwin" ]; then
        echo "🍎 Switching macOS configuration for $TARGET_HOST..."
        darwin-rebuild switch --flake .#$TARGET_HOST
    elif [ "$OS" = "Linux" ]; then
        if [ -f /etc/NIXOS ]; then
             echo "❄️ Switching NixOS configuration for $TARGET_HOST..."
             sudo nixos-rebuild switch --flake .#$TARGET_HOST
        else
             echo "🐧 Switching Home Manager configuration for $USER@$TARGET_HOST..."
             nix run home-manager/master -- switch --flake .#$USER@$TARGET_HOST
        fi
    else
        echo "Unknown OS: $OS"
        exit 1
    fi

# Clean garbage
clean:
    nix-collect-garbage -d

# Run pre-commit checks on all files
pre-commit:
    pre-commit run --all-files
