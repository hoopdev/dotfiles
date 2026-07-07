---
name: commit
description: Review pending changes, group them into logical commits, verify the scope, and commit without including unrelated work.
argument-hint: "[optional commit message]"
disable-model-invocation: true
---

# Commit Changes

Never assume every dirty file belongs to the requested commit.

## Context

- Current git status: !`git status`
- Current git diff summary: !`git diff HEAD --stat`
- Current branch: !`git branch --show-current`
- Recent commits: !`git log --oneline -10`

## Workflow

1. Inspect `git status --short`, staged and unstaged diffs (`git diff HEAD`,
   per-file as needed), untracked files, current branch, and recent commit
   style.
2. Separate requested work from pre-existing or unrelated user changes.
3. If `$ARGUMENTS` provides a message, use it only for files relevant to the
   current request. It does not authorize `git add -A`.
4. Otherwise propose the smallest coherent commit set as a numbered chunk plan:

   ```
   Proposed commits:
   1. refactor: remove X dependency from core — file_a.py, file_b.py, tests/test_a.py
   2. feat: add Y feature — app.py
   3. docs: add docstrings to public API — config.py, core.py
   ```

5. Obtain explicit approval (via AskUserQuestion) when scope or chunk
   boundaries are not already clear from the user's request. The user may
   reorder, merge, split, or rename chunks. If there is only one logical
   chunk, just make one commit — no need to force splitting.
6. Run `/verify` for the scope before committing. Do not bypass failed checks
   unless the user explicitly accepts the named failure.
7. For each approved chunk: stage explicit paths (never `git add -A`, to avoid
   accidentally committing sensitive files), review `git diff --cached`, then
   commit using HEREDOC format:

   ```bash
   git commit -m "$(cat <<'EOF'
   <type>: <title under 70 chars>

   <optional body — what and why, not how>
   EOF
   )"
   ```

8. Show the resulting commit hashes and subjects plus any dirty files left out.

## Commit Messages

- Keep the title under 70 characters.
- Use conventional prefixes: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`,
  `chore:`, or `style:`.
- Match recent history and use imperative mood ("add feature" not "added
  feature").
- Explain why in the body only when useful — focus on "why" rather than "what".
- Do not add attribution trailers unless the user requests them.
- Reference an issue only when the relationship is real; use `Closes #N` only
  when the commit is intended to close it.

## Chunking Heuristics

Good chunk boundaries:

- **Core logic change + its tests** — same commit (feature and tests belong together)
- **Refactor** (removing a dependency, renaming) — one commit even if it touches many files
- **Pure formatting / lint fixes** — separate commit
- **Docstring / comment additions** — separate commit if no logic changes
- **New files** (examples, configs) — group by purpose
- **Config / build changes** — group with related changes or separate

Bad chunk boundaries:

- Splitting a single refactor across multiple commits (half the files updated)
- Mixing unrelated features in one commit
- Separating a feature from its tests

## Hard Rules

Never use `--no-verify`, amend, rebase, reset, or force-push unless explicitly
requested. After a hook failure, create a NEW commit rather than amending. Do
not revert or clean unrelated changes.
