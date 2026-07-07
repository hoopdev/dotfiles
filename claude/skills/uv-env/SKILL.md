---
name: uv-env
description: Manage a project's uv Python environment — dependencies, syncing, running commands, and troubleshooting imports.
user-invocable: false
---

# uv Environment

This project uses [uv](https://github.com/astral-sh/uv) for Python environment management.

## Rules

- Run project Python tools through `uv run` — never run `python` or `pip` directly.
- Add and remove dependency entries with `uv add` / `uv remove` (auto-updates `pyproject.toml` and the lock file); do not hand-edit dependency entries. Direct edits to non-dependency `pyproject.toml` tables (e.g. `[tool.*]`) are allowed when needed.
- Do not use `pip install` against the project environment.
- Use `uv sync` to install from the lock file, and after pulling changes.
- Commit both `pyproject.toml` and `uv.lock` to git.
- Do not manually edit or delete `uv.lock` as a routine fix.

## Common Commands

```bash
# Install / sync dependencies
uv sync                    # Install from lock file
uv sync -U                 # Update and install
uv sync --extra name       # Include an optional extra
uv sync --all-extras       # Include all extras

# Dependency changes
uv add numpy               # Add package
uv add "numpy>=1.26"       # Add with version constraint
uv add --dev pytest        # Add to dev dependencies
uv remove package-name     # Remove package
uv remove --dev package-name

# Execution
uv run python script.py
uv run python -m module_name
uv run pytest
uv run pytest tests/test_file.py
uv run pytest -k "pattern"
uv run ruff check src/ tests/
uv run ruff format --check src/ tests/
```

## Diagnosis

Inspect first:

```bash
uv run python -c "import sys; print(sys.executable)"
uv tree
uv sync --refresh
uv sync
```

Upgrade dependencies only when requested; use `uv lock --upgrade-package NAME`
for a targeted upgrade or `uv lock --upgrade` for an intentional broad upgrade.
Regenerating the lock file (`rm uv.lock && uv sync`) and cache deletion
(`uv cache clean`) are last resorts — the former discards all pins, the latter
discards useful build artifacts.
