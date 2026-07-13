# Architecture

## Flake Structure

- **flake.nix**: Thin entry point — calls `flake-parts.lib.mkFlake` and imports modules from `flake-modules/`
- **flake-modules/**: Per-subsystem flake-parts modules (the real outputs live here)
- **Inputs**: nixpkgs, flake-parts, treefmt-nix, home-manager, nix-darwin, nixos-hardware, nixos-wsl, stylix, hyprland, hyprpanel, xremap, wezterm, dev (standalone fleet tool)

## Directory Layout

```
├── flake.nix                  # Thin entry — mkFlake { imports = [...]; }
├── flake-modules/             # flake-parts modules
│   ├── shared.nix            # Helpers + auto-discovered hosts attrset
│   ├── modules.nix           # flake.nixosModules.* exports
│   ├── nixos.nix             # nixosConfigurations (auto-built from meta.nix)
│   ├── darwin.nix            # darwinConfigurations
│   ├── home.nix              # homeConfigurations (standalone home-manager)
│   ├── per-system.nix        # devShells, formatter, treefmt
│   ├── export.nix            # packages.export-dotfiles — renders portable config into chezmoi/
│   └── dev.nix               # re-exports packages from the standalone dev flake
├── modules/nixos/             # Self-exported NixOS modules
│   ├── default.nix           # Kitchen-sink (imports the three below)
│   ├── nix-ld.nix            # nix-ld for unpatched binaries
│   ├── onepassword.nix       # 1Password CLI + GUI
│   └── nix-settings.nix      # Nix daemon settings + Hyprland cache
├── lib/                       # Shared Nix utilities (non-module)
│   ├── devshell.nix          # Development shell (Python 3.13, uv, Nix tools, build tools)
│   ├── japanese-locale.nix   # Japanese locale settings
│   ├── wsl-common.nix        # WSL-specific settings
│   ├── users.nix             # User account definitions
│   ├── stylix.nix            # Unified Stylix theming (NixOS / darwin / home-manager)
│   └── shonan.yaml           # Shonan base16 color scheme definition
├── home/                      # Home-manager configurations
│   ├── common/               # Cross-platform shared
│   │   ├── cli/             # CLI: git, neovim, shells, ssh, AI tools (claude-code, opencode)
│   │   └── gui/             # GUI: terminals, apps
│   ├── mac/                 # macOS-specific home configurations
│   └── nixos/               # NixOS-specific home configurations
├── claude/                    # Claude Code skill library (canonical source)
│   └── skills/               # skills.toml manifest + one dir per skill
└── hosts/                    # Host-specific system configurations
    ├── kt-proxmox/          # Proxmox VM (NixOS)
    ├── kt-thinkpad/         # ThinkPad (NixOS)
    ├── kt-wsl/              # WSL (NixOS)
    ├── kt-ubuntu/           # Ubuntu (standalone home-manager)
    ├── kt-mba/              # MacBook Air
    ├── kt-mac-studio/       # Mac Studio (meta.nix only — shares mac/)
    ├── kt-mac-mini/         # Mac Mini (meta.nix only — shares mac/)
    └── mac/                 # Shared Mac Studio / Mac Mini config
```

Each `hosts/<name>/meta.nix` declares `{ type, system?, users?, configFrom? }`; `flake-modules/shared.nix` reads the directory and dispatches to the matching subsystem module.

### Claude Code skill library (`claude/skills/`)

Canonical source for Claude Code skills that grow across projects. `dev skill`
(from the `dev` fleet tool) distributes them into each subscribed project's
`.claude/skills/` as plain committed copies and classifies sync state via an
`x-canonical-hash` frontmatter key; per-repo rules live in a
`<!-- project-specific -->` block that survives pushes. Subscriptions are
declared in `claude/skills/skills.toml`. The `/skill-sync` skill (symlinked
into `~/.claude/skills/` by `home/common/cli/claude-code.nix`) drives the
semantic merge: harvesting project improvements back into this library and
redistributing. See `~/git/dev/docs/commands.md` (`dev skill`).

### Chezmoi source tree (non-Nix targets)

Dotfiles for environments without Nix (Windows, bare Jupyter) live under `chezmoi/`, isolated from the Nix flake. A root `.chezmoiroot` points [Chezmoi](https://chezmoi.io) at that subdirectory, so Chezmoi only ever sees dotfile sources — the Nix tree, docs, and build outputs sit outside its root and need no `.chezmoiignore` entries.

```
├── .chezmoiroot               # Contains "chezmoi" — sets the Chezmoi source root
├── wallpaper/                 # Wallpaper asset — Nix-owned (Stylix + Hyprland), NOT a Chezmoi target
└── chezmoi/                   # Chezmoi source root
    ├── dot_config/            # → ~/.config   (nvim, wezterm, starship, zoxide, scoop)
    ├── dot_glzr/              # → ~/.glzr     (GlazeWM + Zebar, Windows)
    ├── AppData/               # → ~/AppData   (Nushell, Windows)
    ├── private_dot_jupyter/   # → ~/.jupyter  (JupyterLab settings)
    └── .chezmoiignore         # Only per-OS target exclusions remain
```

Files in this tree come from three places, and it matters which:

| File | Origin |
|------|--------|
| `dot_config/nvim/init.lua` | Copied from `home/common/cli/init.lua` on every rebuild, by an activation hook in `home/common/cli/neovim.nix` |
| `dot_config/readonly_starship.toml`, `dot_config/wezterm/` | **Generated** — rendered from the Nix config by `nix run .#export-dotfiles` (see below) |
| `dot_glzr/`, `AppData/`, `dot_config/{scoop,winget}`, `private_dot_jupyter/` | Hand-maintained; these targets have no Nix equivalent |

### Why starship / wezterm are generated, not hand-written

These configs are *portable structure + Nix-derived values*: starship's `format` interpolates the Stylix base16 palette (`fg:#${colors.base0A}`), and WezTerm's colors, font, and opacity are injected by Stylix into `programs.wezterm.settings` / `colorSchemes`. A static file cannot express them, and hand-copying them into `chezmoi/` is what let those copies silently drift.

So Nix stays the single source and `flake-modules/export.nix` exports the *rendered* artifacts. They are pure data — no `/nix/store` paths — so they work on machines without Nix. The export uses a dedicated home-manager evaluation (not one of the real hosts) so its output does not depend on which machine runs it, and carries the Windows system glyph rather than the exporting host's.

Platform differences that must survive the trip are resolved at runtime rather than at Nix eval time — e.g. `home/common/gui/wezterm.nix` branches on `wezterm.target_triple` in Lua, so the exact same Lua is valid on macOS, Linux, and Windows.

## Design Principles

- **Modular Flake**: `flake-parts` splits flake outputs across `flake-modules/`
- **Auto-Discovered Hosts**: New hosts appear by adding `hosts/<name>/meta.nix` (no edits to `flake-modules/*.nix` needed)
- **Composable NixOS Modules**: `modules/nixos/{nix-ld,onepassword,nix-settings}.nix` are exported via `flake.nixosModules` so hosts can opt in à la carte
- **Shared Common Base**: `home/common/` is used across all platforms
- **Platform-Specific Overlays**: `home/mac/` and `home/nixos/` extend the common base
- **Unified Theming**: Stylix applies the Shonan color scheme (custom base16) everywhere
- **Reproducible Builds**: `flake.lock` pins all input versions

## Key Components

| Component | Purpose |
|-----------|---------|
| flake-parts | Modular flake outputs |
| treefmt-nix | `nix fmt` integration (nixfmt + statix + deadnix) |
| Home-manager | User environment and dotfile management |
| Nix-darwin | macOS system-level configuration |
| NixOS-WSL | WSL integration |
| Stylix | Unified theming (Shonan base16 color scheme) |
| nixos-hardware | ThinkPad hardware optimizations |
| Neovim (home-manager + lazy.nvim) | Editor — see [docs/neovim.md](neovim.md) |
| Hyprland + HyprPanel | Wayland compositor + panel (NixOS) |
| AeroSpace | Tiling window manager (macOS) |
| WezTerm | Terminal emulator (cross-platform) |
| xremap / Karabiner | Key remapping (NixOS / macOS) |
| nix-ld | Run unpatched binaries on NixOS (uv, Python wheels) |
| 1Password CLI | Secret management |
| Chezmoi | Supplementary dotfile sync for non-Nix environments |
| dev | Standalone fleet tool flake consumed from `~/git/dev` (installed on macOS via `home/mac`) |

## Development Shell (`nix develop`)

Defined in `lib/devshell.nix`:

| Category | Tools |
|----------|-------|
| Python | Python 3.13, uv, ruff, mypy, pytest |
| Nix | nixfmt, statix, deadnix |
| Build | gcc, pkg-config, ninja, meson |
| Utilities | git, curl, wget, htop, tree, pre-commit, just |
| Shell | zsh, starship, direnv, nix-direnv |

Platform-specific libraries are included automatically (Linux: glibc, X11; macOS: system frameworks).

## User Configuration

- Primary user: `ktaga`
- Editor: Neovim
- Shells: Nushell, Zsh
- Color scheme: Shonan (custom base16, via Stylix)
- GC: weekly automatic (system via `nix.gc`, user via `programs.nh.clean`)

## Related Docs

- [docs/commands.md](commands.md) — apply / bootstrap / maintenance / dev-fleet commands
- [docs/neovim.md](neovim.md) — Neovim (home-manager + lazy.nvim) setup
- [docs/ssh.md](ssh.md) — SSH client + 1Password agent configuration
