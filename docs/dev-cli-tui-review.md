# dev CLI/TUI Review

Date: 2026-07-01

## Scope

- `pkgs/dev-cli`
- `pkgs/dev-core`
- `pkgs/dev-tui`
- Review/test/task lifecycle paths used by the TUI

## Findings

### High: Review/Test artifact IDs could overwrite prior runs

`next_review_id` and `next_test_run_id` scanned the project root, while artifacts were written under task-local directories:

- `tasks/<task-id>/reviews/`
- `tasks/<task-id>/test-results/`

This could generate the same `R-YYYYMMDD-001` or `V-YYYYMMDD-001` repeatedly and overwrite previous artifacts.

Status: fixed.

### High: `dev task review` ignored `dev agent review` failures

`dev task review` captured output but did not honor the child exit status. A failed review command could still write a normal review artifact, append `review_completed`, and exit successfully.

Status: fixed.

### High: Review recommendation parsing was unsafe

The parser checked `mergeable` before negative phrases. Output such as `not mergeable` or `not yet mergeable` could incorrectly move the task to `mergeable`.

Status: fixed with safer precedence and explicit success markers.

### Medium: `dev run --all <cmd>` did not match its usage

The parser consumed the first non-flag argument after `--all` as a target instead of as the command, so `dev run --all hostname` failed with a usage error.

Status: fixed.

### Medium: Multi-target `dev run` did not propagate failures

For multiple targets, failed child results were printed but the parent command still exited successfully.

Status: fixed.

### Low: TUI review spinner could remain stuck

When streaming review startup failed, `review_running` remained true. The TUI could keep showing a running spinner even though no child process existed.

Status: fixed.

## Additional Robustness Fixes

- Avoided byte slicing of UTF-8 text in task context/test/review output previews.
- Streamed review stderr into the TUI log view.
- Replaced `sh -c` review launch with direct `Command` invocation.
- Cleared review state on completion.
- Addressed nearby Clippy warnings in modified packages.

## Verification

Passed:

```sh
git diff --check
```

Passed for modified Rust packages:

```sh
nix shell nixpkgs#cargo nixpkgs#rustfmt nixpkgs#clippy nixpkgs#libiconv -c \
  env LIBRARY_PATH=/nix/store/wjb6smwmv8cynnr776j01kjjzdks1863-libiconv-113/lib \
  cargo test -p dev-core -p dev-cli -p dev-tui
```

```sh
nix shell nixpkgs#cargo nixpkgs#rustfmt nixpkgs#clippy nixpkgs#libiconv -c \
  env LIBRARY_PATH=/nix/store/wjb6smwmv8cynnr776j01kjjzdks1863-libiconv-113/lib \
  cargo clippy -p dev-core -p dev-cli -p dev-tui --all-targets -- -D warnings
```

Not run to completion:

```sh
cargo test --workspace
```

Reason: the unrelated `dev-zellij` test link step failed on macOS with `ld: library not found for -lcurl`.
