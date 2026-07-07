---
name: skill-sync
description: Harvest and distribute Claude Code skills across the dev fleet — consolidate per-project skill improvements into the canonical library (dotfiles/claude/skills) and push canonical updates back to subscribed projects. Use when the user wants to sync, harvest, or distribute skills.
disable-model-invocation: true
---

# Skill Sync

Skills grow independently in each project's `.claude/skills/`. This library
(`~/dotfiles/claude/skills/`) is the canonical source; `dev skill` (from
`~/git/dev`) does the deterministic mechanics — hashing, state classification,
and distribution. **Your job is the judgment: merging diverged content.**

## Model

- `skills.toml` here maps each skill → subscribing dev-registry projects.
- Project copies carry an `x-canonical-hash` frontmatter key (library hash at
  last sync) and may hold ONE marked block that `dev skill push` preserves:

  ```markdown
  <!-- project-specific:begin -->
  ## Project Notes
  ...rules that only apply to that repo...
  <!-- project-specific:end -->
  ```

- Everything outside the block is canonical territory: replaced on push.
  Extra project-local files and unmanaged skills are never touched.

## Setup

```bash
export DEV_JSON=1        # all dev output as JSON
dev skill list           # library overview + manifest warnings
dev skill status         # every skill × project, classified
```

If `dev skill` errors about a missing library, add to `~/.config/dev/config.toml`:

```toml
[skills]
library = "~/dotfiles/claude/skills"
```

## States and what to do

| state | meaning | action |
|---|---|---|
| `in-sync` | identical | nothing |
| `missing` | not in project | `dev skill push <proj> <skill>` |
| `library-ahead` | only library moved | `dev skill push <proj> <skill>` |
| `project-ahead` | only project moved | **harvest** (below) |
| `diverged` / `untracked` | both moved / never synced | **merge** (below) |

## Harvest (project-ahead)

1. `dev skill pull <proj> <skill>` — gives files, `stripped_skill_md`, and the
   current `project_block`.
2. Compare with the library version. Move **generic** improvements into
   `~/dotfiles/claude/skills/<skill>/` (Edit the library directly).
3. Anything project-specific goes into the project file's
   `<!-- project-specific -->` block (edit the project's SKILL.md: relocate the
   content into the block — this is the one allowed manual edit of a project copy).
4. `dev skill push <proj> <skill> --force` (force is safe now — you just
   harvested), then `dev skill status` to confirm `in-sync`.

## Merge (diverged / untracked)

Same as harvest, but read BOTH the library and project versions first and
synthesize: canonical = best generic content from both; project block = the
project-only residue. When two projects disagree on generic content, prefer
the tighter, more recently-refined wording; never silently drop a rule — ask
the user if unsure whether something is generic or project-specific.

## Bootstrap (library skill directory missing/empty)

For each skill in `skills.toml` with no `<skill>/` directory here: read every
subscriber's copy, synthesize the canonical version into the library, move
project-only residue into each project's block, then
`dev skill push --all <skill> --force`.

## Finish

1. `dev skill status` — everything should be `in-sync`.
2. `dev git status` — list repos with changes.
3. Show the user a summary of what moved where. **Never commit for them** —
   suggest committing dotfiles and each touched project repo.

## Guardrails

- Never edit a project's skill files except (a) via `dev skill push`, or
  (b) relocating content into its project-specific block during harvest.
- Never delete a project-local skill or extra support files.
- Windows/remote projects are synced via git, not pushed directly.
- If a skill is so project-specific that a canonical core makes no sense,
  leave it unmanaged (remove it from `skills.toml`) rather than force-fit.
