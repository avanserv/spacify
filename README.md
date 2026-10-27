# spacify

[![CI](https://github.com/avanserv/spacify/actions/workflows/ci.yml/badge.svg)](https://github.com/avanserv/spacify/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/spacify)](https://pypi.org/project/spacify/)

A fast formatter for the vertical rhythm of Python and Rust code. `ruff format` and rustfmt decide
how a statement is laid out, but leave most of the blank lines between statements up to you. spacify
fills that gap: it puts air around logical blocks and tightens runs of related statements, and it
only touches the blank lines between statements.

Before:

```python
def load(paths):
    seen = set()
    for path in paths:
        if path in seen:
            continue
        seen.add(path)
    total = len(seen)

    count = 0
    log(total)
    return total
```

After `spacify`:

```python
def load(paths):
    seen = set()

    for path in paths:
        if path in seen:
            continue

        seen.add(path)

    total = len(seen)
    count = 0
    log(total)

    return total
```

spacify never fights your formatter. Every rule stays inside the blank-line bounds `ruff format` and
rustfmt keep, and CI runs spacify and the formatter over CPython's standard library, ripgrep and
tokio in both orders to check they reach a fixed point. It also checks that spacify leaves every
Python file's AST, and every Rust file's tokens, unchanged. Checking CPython's `Lib`, without its
tests, takes under half a second on 8 cores.

## Install

```bash
uv tool install spacify    # or: pipx install spacify, pip install spacify
```

## Use with prek or pre-commit

Put the hook after your formatter's (`ruff-format`, or a `cargo fmt` hook), so spacify sees its
line wrapping:

```yaml
repos:
  - repo: https://github.com/astral-sh/ruff-pre-commit
    rev: v0.16.9
    hooks:
      - id: ruff-format
  - repo: https://github.com/avanserv/spacify-pre-commit
    rev: v0.1.0
    hooks:
      - id: spacify
```

The hook installs the prebuilt wheel from PyPI, so you don't need a Rust toolchain.

## Command line

```bash
spacify                   # format every Python and Rust file under the current directory
spacify src/ tests/a.py   # format these paths
spacify --check           # write nothing; exit 1 if any file would change
spacify --diff            # write nothing; print a unified diff
spacify - < a.py          # read stdin, write stdout (--stdin-filename names it)
```

The exit code is 0 on success, 1 when `--check` or `--diff` found changes, and 2 on an error such as
a syntax error, in which case the file is left untouched. Directory walks respect `.gitignore` and
skip the directories ruff skips by default. A file you name explicitly is always formatted, unless
you also pass `--force-exclude`, which is what the hook does.

## Languages

spacify formats every language it supports by default: Python (`.py`, `.pyi`) and Rust (`.rs`). To
format only some, list them in the configuration, for example `languages = ["python"]`. Files in
other languages are skipped, even when you name them, and stdin takes its language from
`--stdin-filename` (Python without one).

## Known limits

In Python, spacify parses with tree-sitter-python, which rejects a few valid constructs: type parameter
defaults (`def f[T = int]()`, PEP 696), a parenthesised starred subscript (`a[*(b)]`), and an
attribute access broken after its dot onto a line indented less than the statement. A file using one
exits 2 and is left untouched. Files must be UTF-8, as for ruff, and line endings that are a bare
`\r` are refused the same way. Python 2 `print` and `exec` statements are not rejected, since
tree-sitter cannot tell some of them from valid Python 3 such as `print >>f, x`.

In Rust, tree-sitter-rust refuses some nightly-only syntax, and such a file exits 2 untouched.
spacify assumes rustfmt's default ceiling of one blank line, so it does not support a
`blank_lines_upper_bound` above 1.

## Rules

Each rule can be turned on or off. They all apply to sibling statements in the same block.

| Rule | Effect |
| --- | --- |
| `blank-before-block` | At least one blank line before a block statement when another statement comes before it. |
| `blank-after-block` | At least one blank line after a block statement, before the next statement. |
| `blank-before-exit` | At least one blank line before an exit, in blocks of `exit-min-block-statements` (default 3) or more statements. |
| `collapse-groups` | No blank line between two single-line statements of the same kind. |

The added blank lines take priority over `collapse-groups`. Blank lines that have a comment in the
gap are kept, because a comment marks a paragraph you meant. When a rule does add a blank line, it
goes above the comments and attributes that belong to the next statement. What each term means
depends on the language:

| | Python | Rust |
| --- | --- | --- |
| Block statements | `if`, `for`, `while`, `try`, `with`, `match`, and their `async` forms | `if`, `match`, `for`, `while`, `loop`, and bare, `unsafe`, `async` and `const` blocks |
| Exits | `return`, `raise`, `break`, `continue`, a bare `yield` | `return`, `break`, `continue`, and a function body's tail expression |
| Kinds that collapse | assignments, augmented assignments, calls, `del`, `assert`, `global`/`nonlocal`, module constants | `let` and assignments, compound assignments, calls (macros, `.await` and `?` included) |
| Never touched | gaps next to a `def` or `class`, after an `import`, after a docstring | gaps next to an item (`fn`, `struct`, `use`, `const`, ...) |
| Switched off by | `# fmt: off`/`# fmt: on`, `# spacify: off`/`# spacify: on`, `# fmt: skip` | `// spacify: off`/`// spacify: on`, `#[rustfmt::skip]` |
| Most blank lines in a row | 2 at module level (1 in a `.pyi` stub), 1 in a nested block | 1 |

In Python, an assignment outside every function and class whose target names all have an uppercase
letter and no lowercase one (`MAX = 3`, `T = TypeVar("T")`, `A, *REST = items`) is a constant, a kind
apart from other assignments. Imports are never collapsed, because ruff's isort rules separate
import sections with blank lines. In Rust, the tail expression of a closure, a match arm or a
`let x = { ... }` block is a value rather than a return, so it stays tight.

## Configuration

spacify reads `[tool.spacify]` from `pyproject.toml`, or the top level of a `spacify.toml` or
`.spacify.toml`, which is the place for a Rust project. It uses the first one it finds walking up from the current directory, or the file
you pass with `--config`.

```toml
[tool.spacify]
extend-exclude = ["generated", "migrations/*.py"]  # `exclude = [...]` replaces the defaults

languages = ["python", "rust"]                     # the default: every supported language

[tool.spacify.rules]
blank-before-block = true
blank-after-block = true
blank-before-exit = true
exit-min-block-statements = 3
collapse-groups = true

[tool.spacify.rust.rules]                          # overrides the shared rules for Rust only
exit-min-block-statements = 4
```

An exclude pattern containing a `/` matches the path relative to the config file. Any other pattern
matches a single file or directory name.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
