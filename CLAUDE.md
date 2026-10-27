# spacify

A Rust formatter, shipped as a Python wheel through maturin, that adds and removes blank lines
between Python and Rust statements without ever fighting `ruff format` or rustfmt. README.md has the
user-facing rules, and CONTRIBUTING.md has the layout, the gates and the release flow.

## Rules for changes

- Run `make check` before declaring a change done. A change to `src/python/`, `src/rust/`,
  `src/gaps.rs` or `src/rules.rs` also needs `make compat` to pass, which is part of `make check`.
- Never let a rule produce a blank-line count that the language's formatter would change. Probe
  `ruff format --isolated` or `rustfmt` on a sample first, and add the case to `tests/fixtures/`.
- Never touch a gap next to a Python `def`, `class` or docstring, or a Rust item, or a gap inside a
  switched-off region.
- Edits go through `src/edit.rs` only. Nothing rewrites bytes inside a statement.
- Fixture files are byte-exact: the formatting hooks exclude `tests/fixtures/`, and so must any new
  tooling.
- Never edit a version or `CHANGELOG.md`. Release Please owns both.
- Pin every GitHub Action to a commit SHA with the tag in a trailing comment. `.github/zizmor.yml`
  enforces it.
