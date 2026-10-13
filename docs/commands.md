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
brew update && brew upgrade                # macOS: Homebrew is no longer upgraded by `nh darwin switch`
nix flake update                        # Update inputs
nix flake check                         # Validate
nix flake check --no-build --all-systems # Evaluate every host/profile output
nix run .#check-export-dotfiles         # Verify generated Chezmoi files have no drift
nh clean all --keep 5 --keep-since 7d   # GC (user + system)
nix develop                             # Dev shell (Python + Nix tools)
```

## Git hooks (no CI)

Nothing runs this repo's checks automatically, so the pre-commit hooks stand in:

```bash
pre-commit install --hook-type pre-commit --hook-type pre-push
```

Commit → Gitleaks scans staged changes with secret values redacted, then
`nix fmt -- --ci` checks nixfmt + statix + deadnix drift.
Push   → Gitleaks scans every local Git ref, including merge diffs, then
`nix flake check --no-build --all-systems` (every host / profile evaluates),
`nix develop --command python3 -B -m unittest discover -s tests -v` (SSH agent and
secrets regression tests), and `nix run .#check-export-dotfiles` (build and compare
generated files). Push checks run regardless of which file types changed.

## Public repository security

```bash
nix develop --command just secrets-check   # Current files, including untracked files
nix develop --command just secrets-history # Every local Git ref and merge diff
nix develop --command just verify          # Secrets + all regular checks
```

Scans redact secret values. Do not commit scan reports, local credentials, or
private agent settings. `.gitignore` blocks common credential filenames, but
does not protect previously tracked files or arbitrary filenames. Hooks can be
bypassed; run `just verify` before publishing. Scanning cannot establish that
every possible secret is absent. Revoke any real credential found in history
before considering history cleanup.

Existing commit author identities are retained in the working repository. Before
publishing a separate copy, replace author/committer emails ending in `.ts.net`
with the existing GitHub noreply identity: these emails embed private network
and device identifiers. The commit contents and other author identities can
be preserved, but rewritten commits and descendants receive new IDs. Never
publish the original refs as well. Host names,
user names, hardware details and filesystem UUIDs are public configuration
metadata; UUIDs are not credentials. New credentials, private endpoints and
personal connection details belong in machine-local files outside this repo.
Chezmoi's `private_` prefix controls destination permissions, not repository
visibility. Nix store files are readable by all local users: store only
`op://` references in `dotfiles.secrets.env`, never resolved values.

NixOS SSH servers on kt-proxmox and kt-thinkpad accept public-key authentication
only and refuse direct root login. Before applying on a remote machine,
ensure the user's `authorized_keys` is provisioned and test key authentication
in a second session. SSH remains reachable through the existing network
configuration; listener exposure is separate from authentication.

Ollama is no longer installed or served by Nix. Existing Homebrew installations
are left under Homebrew's ownership; activation keeps `cleanup = "none"`.
The normal user on kt-proxmox is not a Nix trusted user; Docker group membership
remains an intentional, privileged capability on this host.

JupyterLab settings and server configuration are no longer synced by Chezmoi.
Existing `~/.jupyter` files on deployed machines need a separate local review;
removing their source from this repository does not harden running servers.

OpenCode authentication plugins are pinned to explicit versions. Review
upstream changes before updating them; version pinning does not itself audit
their code. Native AI app installers, their auto-updates, Homebrew packages,
and Neovim plugins remain outside `flake.lock`. Bootstrap checks official HTTPS
installer scripts against the SHA-256 hashes in `ai-tools.nix` before executing
them. A changed script fails closed: inspect the upstream script before updating
its hash. The scripts still fetch upstream-selected app releases; pinning the
installer does not pin those releases or their self-updates.

## Orca

Orca manages worktrees, sessions, and skills. This repository only supplies
project setup and verification commands:

```bash
nix develop --command pre-commit install --hook-type pre-commit --hook-type pre-push
nix develop --command just verify
```

Manage skills in Orca independently of dotfiles. See [Orca workflow](orca.md).

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
Bootstrap also refuses scripts whose SHA-256 differs from the inspected version
in `home/common/cli/ai-tools.nix`; maintainers must review changes before updating
these hashes.
To migrate, remove the previous installation with its owning package manager,
then run bootstrap. App versions are outside Nix generation rollback; Claude's
own update channel and auto-update settings remain under its control.

Running sessions and services are not restarted. Codex version mismatches are
reported after maintenance. Once active work finishes, use
`codex app-server daemon restart` for a CLI-managed daemon; externally launched
servers must be restarted through their original supervisor or launcher.
Do not create a second daemon to replace one owned by Coder or another service.

## Secrets from 1Password

Secrets are never written to disk by this config. `op://` references are
declared per host in `dotfiles.secrets.env` (rendered to
`~/.config/op/secrets.env`); machine-local extras go in the untracked
`~/.config/op/secrets.local.env` in the same `KEY=op://vault/item/field` form.

```bash
with-secrets claude             # run one command with the variables injected
with-secrets codex              # interactive terminals preserve TUI support
with-secrets command >log.txt   # redirected output uses 1Password's secret masking
with-secrets --no-masking command # explicit opt-out; output may expose secret values
secrets-load                    # zsh: export them into the current shell (one auth prompt)
secrets-unload                  # zsh: drop them again
op signin                       # headless Linux without the desktop app
```

Prefer `with-secrets` for one command. `secrets-load` exports values to the
shell and subsequent child processes; `secrets-unload` cannot remove them from
already running children. Interactive terminals disable output masking only
when both stdout and stderr are terminals. Session recorders still see that
output; use redirected, masked output when collecting logs.

The 1Password desktop app must have "Integrate with 1Password CLI" enabled
(Settings → Developer) so `op` can authenticate with Touch ID / system auth.

### Headless hosts (kt-proxmox, kt-wsl, kt-ubuntu)

`op` is installed everywhere, but without the desktop app it needs one of:

```bash
# Interactive sessions (expire after 30 min idle):
op account add            # once: sign-in address, email, Secret Key, password
eval "$(op signin)"       # per session; then with-secrets / secrets-load work

# Unattended (cron, agents): a 1Password Service Account scoped to a
# dedicated vault. Keep the token in ~/.config/zsh/local.zsh (chmod 600):
export OP_SERVICE_ACCOUNT_TOKEN=ops_…
```

The 1Password SSH agent forwarded from a Mac only carries SSH keys — it does
not authenticate `op` on the remote side.

## Export config to non-Nix machines (Chezmoi)

```bash
nix run .#export-dotfiles   # from the repo root
```

Renders the configs whose values come from Nix/Stylix (starship, WezTerm) plus Neovim's `init.lua` into `chezmoi/dot_config/`, then commit the result. Windows picks them up with `chezmoi apply`. Output is host-independent — running it on a Mac or a Linux box produces byte-identical files. Never edit the copies under `chezmoi/dot_config/{readonly_starship.toml,wezterm/}`; edit the Nix source and re-run.
