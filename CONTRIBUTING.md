# Contributing

## Setup

You need a Rust toolchain (`rustup`; `rust-toolchain.toml` picks the channel and components), `make`,
[prek](https://github.com/j178/prek) and [uv](https://docs.astral.sh/uv/). The ruff round trip also
needs `ruff` on `PATH`, and `make build` needs `maturin`:

```bash
uv tool install prek
uv tool install ruff
uv tool install maturin
make install
```

`make install` installs the git hooks. To use the commit message template as well:

```bash
git config commit.template .gitmessage
```

## The gates

```bash
make check
```

runs everything CI runs except the security scan: format (rustfmt), lint (clippy, zizmor), the
tests, and the formatter round trips: ruff over the fixtures and CPython's `Lib`, rustfmt over the
fixtures, ripgrep and tokio. `make help` lists the
individual targets. `make format` is the one that fixes, where `make check` only reports.

The git hooks run the same Rust checks, plus a few that only make sense there: secrets (gitleaks),
spelling (codespell), Markdown (markdownlint) and the workflow audits (actionlint, zizmor). `make
hooks` runs all of them over the whole tree.

## How the code is laid out

Each language has a module, `src/python/mod.rs` and `src/rust/mod.rs`, that parses with tree-sitter,
finds the statements of every block and classifies them. `src/gaps.rs` turns each pair of sibling
statements into a `Gap`: where it runs, which comments belong to the next statement, and which
regions are switched off. `src/rules.rs` decides how many blank lines that gap should have, or that no
rule has an opinion. `src/edit.rs` rewrites only those blank-line runs, so a rule can never change a
byte inside a statement. `src/language.rs` maps file extensions to languages, and `src/main.rs` and
`src/walk.rs` are the command line and the file walk.

A new language needs a module like those two, a variant in `src/language.rs`, a `[<language>.rules]`
table in `src/config.rs`, fixtures, a round-trip script against its formatter, and a corpus of real
code in the `Makefile`.

A rule must hold the promise in the README: it never moves a blank-line count outside what the
language's formatter keeps. Check `ruff format --isolated` or `rustfmt` on a sample before writing a
rule, because a formatter owns more gaps than it looks like it does (in Python, those next to a `def`
or a `class`, after an import, after a docstring).

## Tests

A rule change comes with a fixture: `tests/fixtures/<name>.<ext>` and the `<name>.expected.<ext>` it
must format to. The test also checks that the expected file is a fixed point. To rewrite the expected
files from the current output, then review the diff:

```bash
SPACIFY_BLESS=1 cargo test --test fixtures
```

Unit tests for a module's internals sit in a `tests` module at the bottom of its file.

`make compat` is the check that matters for the formatters. `scripts/ruff-compat.sh` runs both
orders against ruff over the fixtures and CPython's `Lib` (the Makefile's `CPYTHON_SKIP` lists the
files left out, and why), then formats a copy with spacify alone and runs `scripts/ast-equal.py` to
prove every file kept its AST. `scripts/rustfmt-compat.sh` does the same against rustfmt over the
fixtures, ripgrep and tokio, with `examples/tokens_equal.rs` proving every file kept its tokens. `make bench` times a
check over CPython's `Lib` with hyperfine.

## Commits and pull requests

Commits follow [Conventional Commits](https://www.conventionalcommits.org):
`<type>(<scope>): Summary`, with the body saying why the change exists. `.gitmessage` has the
details.

Pull requests are squash-merged, so **the pull request title is the commit**. CI checks that it is
a conventional one.

- `feat` bumps the minor version and `fix` the patch.
- `feat!`, or a `BREAKING CHANGE:` footer, is a breaking change: a minor bump while the version is
  below 1.0, a major one after. A rule that changes formatted output is a `feat`, since every user's
  next run rewrites files.
- `docs`, `perf` and `revert` bump the patch too: they appear in the changelog, and whatever appears
  there is released.
- Everything else (`chore`, `refactor`, `test`, `ci`, `build`, `style`) bumps nothing.

## Releases

Nobody edits a version. [Release Please](https://github.com/googleapis/release-please) reads the
commits on `main` and keeps a release pull request open with the next version (in `Cargo.toml` and
`Cargo.lock`) and the changelog. `pyproject.toml` reads the version from `Cargo.toml`.

Merging the release pull request tags the release, builds wheels for Linux (manylinux and musl,
x86_64 and aarch64), macOS (x86_64 and arm64) and Windows (x64), and publishes them to PyPI through
[trusted publishing](https://docs.pypi.org/trusted-publishers/). The publish job waits for a
maintainer to approve the `pypi` deployment.

Once PyPI has the version, the workflow pins it in
[spacify-pre-commit](https://github.com/avanserv/spacify-pre-commit) and tags that repository with
the same version. That is the repository users put in their hook configuration.

## Security

See [SECURITY.md](SECURITY.md).
