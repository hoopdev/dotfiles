---
name: run-notebook
description: Execute Jupyter notebooks in isolated output directories and report runtime failures without modifying source notebooks.
argument-hint: "[path or 'all']"
disable-model-invocation: true
allowed-tools:
  - Bash(uv run jupyter *)
  - Bash(uv sync)
  - Read
  - Glob
---

# Run Notebook

Execute and validate notebook(s): $ARGUMENTS

## Steps

1. Resolve the target:
   - `all`: glob the project's notebook locations (e.g. `notebooks/**/*.ipynb`,
     `examples/**/*.ipynb`)
   - otherwise: the exact path supplied by the user

2. Execute each notebook into its own temporary directory so equal basenames do
   not collide and the checked-in notebook is never modified:

   ```bash
   uv run jupyter nbconvert --to notebook --execute \
     --ExecutePreprocessor.timeout=600 \
     --output-dir=<unique-temp-dir> \
     <path>
   ```

3. Treat a nonzero `nbconvert` exit as failure. Inspect the executed notebook
   and logs for cell exceptions (`ename`/`evalue` in output cells), convergence
   failures, NaN or other non-finite results, and missing outputs the notebook
   explicitly promises (e.g. plots).

4. Report each path, pass/fail, cells executed, elapsed time, and the first
   actionable error. Distinguish unavailable optional hardware/dependencies
   from code failures.

## Constraints

- For import errors, run `uv sync`; do not install the project with ad hoc pip.
- Increase the timeout only for a notebook whose expected workload justifies
  it.
- Do not rewrite notebook outputs merely to make the diff clean.
