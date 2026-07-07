---
name: verify
description: Run non-mutating, scope-aware verification checks before a commit or handoff.
argument-hint: "[--skip-tests]"
disable-model-invocation: true
allowed-tools:
  - Bash(git status *)
  - Bash(git diff *)
  - Bash(uv run ruff *)
  - Bash(uv run pytest *)
  - Bash(uv run mkdocs *)
---

# Verify

Inspect `git status --short` and the complete relevant diff before choosing
checks. Include untracked files in the review; `git diff` alone omits them.

## Workflow

1. Always run non-mutating source checks when Python changed:

   ```bash
   uv run ruff check src/ tests/
   uv run ruff format --check src/ tests/
   ```

   If checks fail, report the errors. Do not silently auto-fix formatting
   during verification.

2. Run focused tests while iterating, then the full suite before handoff:

   ```bash
   uv run pytest
   ```

   The full suite must pass; if tests fail, report the failures and stop.

   `--skip-tests` is valid only for documentation-only changes. Config, example,
   notebook, and build changes are not automatically documentation-only.

3. Report commands run, pass/fail counts, skipped checks, and the reason for
   each skip:
   - All checks passed → ready to commit
   - Checks failed → list failures and recommend fixes

## Usage Examples

Standard verification:
```
/verify
```

Skip tests for documentation-only changes:
```
/verify --skip-tests
```

## Notes

- This skill is automatically invoked by `/commit` before creating commits
- Can be run standalone for quick validation
- Exit with error if any check fails to prevent committing broken code
