---
name: test-setup
description: Apply pytest fixture, marker, and numerical assertion conventions when adding or changing tests.
user-invocable: false
---

# Testing Conventions

## Fixtures and Structure

- Reuse shared fixtures in `tests/conftest.py` when their assumptions match the
  test. Keep narrowly scoped setup helpers in the test module.
- Follow the Arrange / Act / Assert pattern, grouping related tests in classes:

```python
class TestModuleName:
    def test_basic_functionality(self):
        # Arrange
        setup_data = create_test_data()

        # Act
        result = function_under_test(setup_data)

        # Assert
        assert result == expected_value
```

## Markers and Suite Hygiene

- Keep the default suite fast. Mark only materially expensive or
  production-scale cases with `@pytest.mark.slow`.
- Mark tests that need unavailable resources (hardware, network, GUI) with
  dedicated markers and exclude them by default in `pyproject.toml` so the
  default run stays self-contained.

## Numerical Assertions

- Use `pytest.approx()` for scalar comparisons.
- Use `numpy.testing.assert_allclose()` for arrays.
- Choose tolerances from the underlying numerical error, not convenience.
- Check invariants where useful: shape, finiteness, symmetry/Hermiticity,
  normalization, conservation, boundary values, and monotonic convergence.
- Seed stochastic tests and avoid assertions on outputs with arbitrary
  ordering, phase, or sign (e.g. eigensolver results, degenerate-state bases).

## Execution

```bash
uv run pytest tests/test_module.py -q
uv run pytest tests/test_module.py::test_name -q
uv run pytest -k "pattern"
uv run pytest
```

Add a regression test that fails before the fix. Run focused tests first and the
full suite before handoff.
