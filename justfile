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

secrets-check:
    gitleaks dir --redact .

secrets-history:
    gitleaks git --redact --log-opts="--all --full-history -m" .

verify: secrets-check secrets-history format-check eval test export-check
