# spacify

A Rust formatter, shipped as a Python wheel through maturin, that adds and removes blank lines
between Python statements without ever fighting `ruff format`. README.md has the user-facing rules,
and CONTRIBUTING.md has the layout, the gates and the release flow.

## Rules for changes

- Run `make check` before declaring a change done. A change to `src/python/` also needs `make compat`
  to pass, which is part of `make check`.
- Never let a rule produce a blank-line count that `ruff format --isolated` would change. Probe ruff
  on a sample first, and add the case to `tests/fixtures/`.
- Never touch a gap next to a `def`, a `class` or a docstring, or a gap inside a `fmt: off` region.
- Edits go through `src/edit.rs` only. Nothing rewrites bytes inside a statement.
- Fixture files are byte-exact: the formatting hooks exclude `tests/fixtures/`, and so must any new
  tooling.
- Never edit a version or `CHANGELOG.md`. Release Please owns both.
- Pin every GitHub Action to a commit SHA with the tag in a trailing comment. `.github/zizmor.yml`
  enforces it.
