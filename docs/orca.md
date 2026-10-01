# Orca workflow

Orca owns worktrees, agent sessions, progress, and review. Nix owns the tools and
runtime dependencies. This repository owns project instructions and verification. Agent accounts, MCP credentials, and Orca's mutable state stay local.

## This repository

- `AGENTS.md` is the shared project guide; `CLAUDE.md` imports it.
- `orca.yaml` runs `pre-commit install` in the Nix shell for new worktrees.
- `justfile` groups repository verification tasks only. Skill operations use
  native commands, not project-specific wrappers.

Keep the current workspace when continuing existing work. New worktrees contain
committed files from their selected base; they do not inherit uncommitted fixes.
Once these changes are committed and available on the selected base, new Orca
worktrees receive the setup hook and shared instructions automatically.

## Skills are managed in Orca

This repository does not contain skill sources, installers, compatibility links,
or synchronization rules. Manage installed skills and their distribution through
Orca's Skills UI. Project-specific instructions belong in `AGENTS.md`.

Removing the library from dotfiles does not uninstall the independent copies
already installed on the execution host. It also does not publish them or establish
cross-host sharing. Use Orca's sharing and install management when distributing
custom skills; sharing publishes versions, rather than continuously synchronizing
edits in both directions. See [Orca's skills documentation](https://www.onorca.dev/docs/cli/skills).

Desktop and SSH execution hosts have separate installations. Use the intended
host in Orca; a relay CLI in an SSH terminal may control the desktop rather than
perform a host-local operation. Load the installed Orca guide and consult live
help instead of maintaining a second command catalog in dotfiles.

## Repository setup

The worktree hook prepares the Nix environment and Git hooks. For an existing
checkout or skipped setup, run the same native command from the repository:

```sh
nix develop --command pre-commit install --hook-type pre-commit --hook-type pre-push
```

Setup does not activate Home Manager, fetch secrets, install skills, or launch
additional agents.

## Work and review

Use Orca to create an independent task workspace, select the agent, and attach
the relevant issue when applicable. Its setup policy must permit repository
hooks. The checked-in setup hook is idempotent; the repository setup command
above also works in older checkouts or when automatic setup was skipped.

For non-interactive agent commands, make the environment explicit:

```sh
nix develop --command just test
nix develop --command just verify
```

Verification includes formatting/lint, every NixOS/Darwin/Home Manager host's
evaluation, shell regression tests, and generated Chezmoi drift. It does not
build or apply a complete operating system. Git commit/push hooks remain the
same gate whether the action starts in Orca, an agent, or a terminal.

Keep `.direnv` and environments specific to the worktree. This repository has
no shared-directory or secret-copy rules in `orca.yaml`/`.worktreeinclude`.
Global Nix/download caches already provide reuse without sharing mutable shells.

Use Orca worktree comments/status for meaningful checkpoints. Review edits in
its diff viewer and use review notes when returning feedback to an agent. Open
local reports in the editor/browser; publishing artifact links is a separate
external action. Use the `orca-cli` skill for worktrees, terminal control, and
embedded-browser interaction. Use `orchestration` only for supervised parallel
tasks; a simple handoff does not need a task DAG. Load the live guide before
acting rather than copying a permanent command list into project instructions.

## Hooks, MCP, and credentials

Orca's agent-status hooks track sessions; Git hooks validate repository changes.
They serve different purposes. Let Orca manage its agent hooks rather than
installing another status hook through Nix. The desktop's existing hook status
can be checked with `orca agent hooks status --json`; a relay reports the
desktop, not proof of an SSH host's local configuration.

Manage supported MCP integrations in Orca Settings → Integrations → MCP.
For an existing server, record its purpose and execution host, configure the
Orca-managed registration, verify the intended agent can call it on that host,
then remove the duplicate registration. Do not delete working agent-specific
registrations until the replacement works. This repository does not copy
credentials or overwrite agent settings to perform that migration.

CLI application installation/update stays with `ai-tools-bootstrap` and
`ai-tools-update`; Orca owns its active sessions and any services it launches.
Do not restart Orca-owned services through an independent daemon manager.

## References

- [Orca worktrees](https://www.onorca.dev/docs/model/worktrees)
- [Skills and MCP](https://www.onorca.dev/docs/cli/skills)
- [Repository setup example](https://github.com/stablyai/orca/blob/main/orca.yaml)
- [Orca settings](https://www.onorca.dev/docs/settings)
- [Headless environment](headless.md)
