# Headless development

Select `base` and `headless` in NixOS `systemProfiles`, and `nixos-headless` and
`developer` in `homeProfiles`. The latter is also usable on standalone Home
Manager Linux and macOS hosts.

## System defaults

The headless system module disables X11, the display manager, printing and
PipeWire by default. Hosts can explicitly enable a service when needed.
Persistent journald logs are limited to 512 MiB and 30 days; runtime logs to
128 MiB. SSH, Tailscale, GPU drivers, containers and user lingering are separate
host decisions. WSL does not acquire an SSH listener through this profile.

Proxmox retains NVIDIA/graphics and 1Password CLI, without the desktop app.
Existing authentication and SSH agent forwarding are unchanged.

## Orca on SSH hosts

Orca can own worktrees and sessions on this host while the desktop supplies the
UI. `orca.yaml` installs Git hooks directly inside the Nix development shell;
`nix develop --command just verify` provides the common verification command.
Skills are managed in Orca outside this repository. Desktop and SSH execution
hosts have separate installations. See [Orca workflow](orca.md) for ownership
boundaries; the relay CLI may describe the desktop rather than this host.

## Project environments and agents

Apply Home Manager, open a new shell, and approve a project's `.envrc` with
`direnv allow`. This checkout provides `use flake`; keep project dependencies
and lock files in each project's own flake rather than growing the shared shell.
Zsh and Nushell load the environment when entering the directory.

IDE tasks and agent subprocesses may use non-interactive shells. In project
instructions for Codex or Claude, document the working directory and explicit
test/build commands, for example:

```bash
# Run from a project with its own devShell and pytest dependency:
nix develop --command pytest -q
# Use an already approved .envrc:
direnv exec . pytest -q
```

Alternatively, launch the agent from the activated project shell so it inherits
the environment. Neither workflow requires Zellij. The shared devShell does not
replace the calling shell; request `nix develop --command zsh` for interactive Zsh.

Machine-local interactive Zsh configuration is not loaded by these commands.
Provide required credentials through the parent process or the existing secret
provider, not as literal values in Nix expressions. Nix activation does not run
the Codex or Claude installers; the existing `ai-tools-bootstrap` and
`ai-tools-update` commands continue to manage their user-local installations.

## Applying

Review/build first, then apply on the intended NixOS host:

```bash
nh os build . -H kt-proxmox
nh os switch . -H kt-proxmox
```

For standalone Home Manager use the matching configuration, for example
`nh home switch . -c jovyan@kt-ubuntu`. System profile changes affect only NixOS.
