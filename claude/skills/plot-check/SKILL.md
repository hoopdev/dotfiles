---
name: plot-check
description: Best practices for writing and reviewing Python plotting code — figure lifecycle, style, numerical fidelity, and verification.
user-invocable: false
---

# Plotting Conventions

## Figure Lifecycle

- Plotting helpers that return figures must create their own `Figure`, return it,
  and avoid unconditional `plt.show()` or `plt.close()` so notebooks display
  once and callers retain control:
  - `plt.show()` + `return fig` → Jupyter displays twice
  - `plt.close()` + `return fig` → Jupyter cannot display a closed figure
  - Just `return fig` → displays once (Jupyter handles it)

```python
def plot_something(..., save_path=None):
    fig, ax = plt.subplots(...)
    # ... plotting code ...
    plt.tight_layout()
    if save_path:
        fig.savefig(save_path, dpi=300, bbox_inches="tight")
    return fig  # Jupyter auto-displays this (once)
```

- Save through `fig.savefig(...)`, not global `plt.savefig(...)`, when a figure
  object is available.
- Live displays and interactive animations may use `plt.ion()`,
  `plt.show(block=False)`, `plt.pause()`, display handles, or explicit close
  operations. Do not apply the static-helper rule to those classes.
- Save-only compatibility functions may close a figure if their documented API
  intentionally returns no figure. Preserve behavior unless the task changes
  that contract.
- Tests should use a non-interactive backend where needed and close figures they
  create.

## Style and Readability

- Label axes, colorbars, units, and coordinate planes explicitly.
- Do not enforce a single font size on every annotation. Dense heatmap labels,
  legends, and live diagnostics may be smaller when readability and layout
  require it.
- Keep physical units consistent with the plotted data and convert only at the
  presentation boundary.
- Avoid global `rcParams` mutation inside reusable plotting helpers.

## Numerical Data

- Do not use one-dimensional interpolation for unstructured 2D/3D data. Use a
  Delaunay/ND interpolation path or a project-provided evaluator.
- Handle points outside the interpolation hull deliberately and make fallback
  behavior visible in code.
- Avoid silently changing normalization, sign, or units for presentation.

## Artifacts and Verification

- Follow the owning workflow's output directory. Existing figures may be tracked
  reference results; do not overwrite them unless requested.
- Add or update tests for return type, axes/artists, labels, and save behavior.
- For notebook-facing changes, execute the affected notebook when practical and
  inspect the rendered output, not just object creation.
