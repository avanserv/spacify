# spacify

[![CI](https://github.com/avanserv/spacify/actions/workflows/ci.yml/badge.svg)](https://github.com/avanserv/spacify/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/spacify)](https://pypi.org/project/spacify/)

A fast formatter for the vertical rhythm of Python code. `ruff format` decides how a statement is
laid out, but leaves most of the blank lines between statements up to you. spacify fills that gap:
it puts air around logical blocks and tightens runs of related statements, and it only touches the
blank lines between statements.

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

spacify never fights ruff. Every rule stays inside the blank-line bounds `ruff format` keeps, and
CI runs both tools over CPython's standard library in both orders to check they reach a fixed point.
Checking all of CPython's `Lib` takes under half a second on 8 cores.

## Install

```bash
uv tool install spacify    # or: pipx install spacify, pip install spacify
```

## Use with prek or pre-commit

Put the hook after `ruff-format`, so spacify sees ruff's line wrapping:

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
spacify                   # format every Python file under the current directory
spacify src/ tests/a.py   # format these paths
spacify --check           # write nothing; exit 1 if any file would change
spacify --diff            # write nothing; print a unified diff
spacify - < a.py          # read stdin, write stdout (--stdin-filename names it)
```

The exit code is 0 on success, 1 when `--check` or `--diff` found changes, and 2 on an error such as
a syntax error, in which case the file is left untouched. Directory walks respect `.gitignore` and
skip the directories ruff skips by default. A file you name explicitly is always formatted, unless
you also pass `--force-exclude`, which is what the hook does.

## Rules

Each rule can be turned on or off. They all apply to sibling statements in the same block.

| Rule | Effect |
| --- | --- |
| `blank-before-block` | At least one blank line before `if`, `for`, `while`, `try`, `with` and `match` (including the `async` forms) when another statement comes before them. |
| `blank-after-block` | At least one blank line after one of those blocks, before the next statement. |
| `blank-before-exit` | At least one blank line before `return`, `raise`, `break`, `continue` and a bare `yield`, in blocks of `exit-min-block-statements` (default 3) or more statements. |
| `collapse-groups` | No blank line between two single-line statements of the same kind: assignments, augmented assignments, calls, `del`, `assert`, `global`/`nonlocal`. |

The added blank lines take priority over `collapse-groups`. A few places are never touched:

- Gaps next to a `def` or `class`, after an `import`, or after a docstring. Ruff decides these.
- Blank lines that have a comment in the gap, because a comment marks a paragraph you meant. When a
  rule does add a blank line, it goes above the comments that belong to the next statement.
- Regions between `# fmt: off` and `# fmt: on` (or `# spacify: off` and `# spacify: on`), and any
  line marked `# fmt: skip`.

Imports are never collapsed, because ruff's isort rules separate import sections with blank lines.
A nested block never gets more than one blank line in a row. At module level the limit is two, or one
in a `.pyi` stub.

## Configuration

spacify reads `[tool.spacify]` from `pyproject.toml`, or the top level of a `spacify.toml` or
`.spacify.toml`. It uses the first one it finds walking up from the current directory, or the file
you pass with `--config`.

```toml
[tool.spacify]
extend-exclude = ["generated", "migrations/*.py"]  # `exclude = [...]` replaces the defaults

[tool.spacify.rules]
blank-before-block = true
blank-after-block = true
blank-before-exit = true
exit-min-block-statements = 3
collapse-groups = true
```

An exclude pattern containing a `/` matches the path relative to the config file. Any other pattern
matches a single file or directory name.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
