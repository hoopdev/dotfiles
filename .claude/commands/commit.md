---
allowed-tools: Bash(git *:*)
argument-hint: [commit-message]
description: Stage changes, summarize diff, and create a conventional commit
---

Create a git commit with the following steps:

1. Run `git status` and `git diff --stat` to review what changed
2. If a commit message was provided as `$ARGUMENTS`, use it. Otherwise, generate a concise one-line message from the diff.
3. Use conventional commit format: `feat:`, `fix:`, `chore:`, `refactor:`, `docs:` — most changes in this repo are `chore:`
4. Stage only the changed/new files relevant to the commit (avoid `git add -A`; be explicit)
5. Commit with the message. Append the Co-Authored-By trailer:
   ```
   Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
   ```
6. Show `git log --oneline -1` to confirm
