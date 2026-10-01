set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

format-check:
    nix fmt -- --ci

eval:
    nix flake check --no-build --all-systems

test:
    nix develop --command python3 -B -m unittest discover -s tests -v

export-check:
    nix run .#check-export-dotfiles

verify: format-check eval test export-check
