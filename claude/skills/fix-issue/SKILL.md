---
name: fix-issue
description: Investigate and fix a GitHub issue end to end, including reproduction, scoped implementation, regression tests, and verification.
argument-hint: "[issue-number]"
disable-model-invocation: true
---

# Fix GitHub Issue

Address issue $ARGUMENTS.

## Workflow

1. Fetch the issue: `gh issue view $ARGUMENTS --json title,body,labels,comments`.
   Read the title, body, labels, and comments. Confirm the repository, whether
   this is a bug, feature, or enhancement, and whether later discussion changes
   the acceptance criteria.
2. Inspect the dirty worktree before editing. Do not overwrite or include
   unrelated user changes.
3. Reproduce the problem when feasible. Trace the code path from entry point to
   the bug or missing feature, and identify the root cause and affected public
   contracts.
4. Read the implementation, tests, docs, examples, and user workflows relevant
   to the affected path.
5. State a concise implementation plan before writing code: the root cause (for
   bugs) or design approach (for features), and which files will change. Then
   make the smallest complete change, following project conventions.
6. Add a regression test that fails before the fix, placed alongside the
   project's existing tests for the affected area.
7. Run focused checks, then the project's linter, formatter check, and full
   test suite — all must pass.
8. Re-run the original reproducer (including any reproducer given in the issue)
   and update affected docs and examples.
9. Report the root cause, files changed, verification, and any residual risk.

Do not create a branch, commit, push, or close the issue unless the user asks.
When requested, use `fix/<issue>-<short-description>` and reference the issue
accurately in the commit or pull request.
