# Commands

## Apply

`nh` is preferred (raw fallback: `sudo nixos-rebuild switch --flake .#<host>`).

```bash
nh os switch . -H <host>          # NixOS: kt-thinkpad, kt-proxmox, kt-wsl
nh darwin switch . -H <host>      # macOS: kt-mac-studio, kt-mac-mini, kt-mba
nh home switch . -c <user>@<host> # home-manager: ktaga@kt-ubuntu, jovyan@kt-ubuntu
```

## Bootstrap (fresh host, no nh yet)

```bash
nix run nixpkgs#nh -- darwin switch . -H kt-mac-studio
```

## Maintenance

```bash
nix flake update                        # Update inputs
nix flake check                         # Validate
nix flake check --all-systems           # Evaluate every host/profile output
nix run .#check-export-dotfiles         # Verify generated Chezmoi files have no drift
nh clean all --keep 5 --keep-since 7d   # GC (user + system)
nix develop                             # Dev shell (Python + Nix tools)
```

## AI tools (developer profile)

Apply Home Manager first, then open a new shell:

```bash
ai-tools-bootstrap          # Install missing Claude Code / Codex native apps
ai-tools-update             # Update both apps without rebuilding Nix
ai-tools-bootstrap codex    # Install just Codex
ai-tools-update claude      # Update just Claude Code
```

Nix manages these commands, PATH, and the NixOS `nix-ld` runtime. App binaries
live in writable user directories and are installed by the
[Claude Code](https://code.claude.com/docs/en/setup) and
[Codex](https://learn.chatgpt.com/docs/codex/cli) official installers.
Home Manager activation does not download either app. Bootstrap skips existing
native installations; update requires them to be installed. Both commands refuse
to overwrite installations managed by Nix, npm, Homebrew, or custom launchers.
To migrate, remove the previous installation with its owning package manager,
then run bootstrap. App versions are outside Nix generation rollback; Claude's
own update channel and auto-update settings remain under its control.

Running sessions and services are not restarted. Codex version mismatches are
reported after maintenance. Once active work finishes, use
`codex app-server daemon restart` for a CLI-managed daemon; externally launched
servers must be restarted through their original supervisor or launcher.
Do not create a second daemon to replace one owned by Coder or another service.

## Export config to non-Nix machines (Chezmoi)

```bash
nix run .#export-dotfiles   # from the repo root
```

Renders the configs whose values come from Nix/Stylix (starship, WezTerm) plus Neovim's `init.lua` into `chezmoi/dot_config/`, then commit the result. Windows picks them up with `chezmoi apply`. Output is host-independent — running it on a Mac or a Linux box produces byte-identical files. Never edit the copies under `chezmoi/dot_config/{readonly_starship.toml,wezterm/}`; edit the Nix source and re-run.
