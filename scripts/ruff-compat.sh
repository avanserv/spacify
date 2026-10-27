#!/usr/bin/env bash
# Proves spacify and `ruff format` never fight over a tree, in both orders, and that spacify alone
# keeps every file's meaning:
#
#   ruff → spacify             then both checks pass at once (the order the hook docs give);
#   spacify → ruff → spacify   then both checks pass (ruff can join a wrapped statement onto one
#                              line, which may make it eligible for collapse-groups once more);
#   spacify                    then every file has the AST it started with (scripts/ast-equal.py).
#
# Usage: scripts/ruff-compat.sh <source dir> [spacify binary]. Needs `ruff` and `uv` on PATH, and
# COMPAT_PYTHON set to a Python version that parses the whole tree (the Makefile sets it).
set -euo pipefail

src=${1:?usage: $0 <source dir> [spacify binary]}
spacify=$(realpath "${2:-target/release/spacify}")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

check() {
    ruff format --isolated --check --quiet . || { echo "ruff format would still change $1" >&2; exit 1; }
    "$spacify" --check --quiet . || { echo "spacify would still change $1" >&2; exit 1; }
}

# A spacify.toml enabling Python alone keeps the other languages in the tree out of it.
for copy in ruff-first spacify-first spacify-only; do
    cp -r "$src" "$work/$copy"
    echo 'languages = ["python"]' > "$work/$copy/spacify.toml"
done

(
    cd "$work/ruff-first"
    ruff format --isolated --quiet .
    "$spacify" --quiet .
    check "ruff → spacify"
)

(
    cd "$work/spacify-first"
    "$spacify" --quiet .
    ruff format --isolated --quiet .
    "$spacify" --quiet .
    check "spacify → ruff → spacify"
)

(
    cd "$work/spacify-only"
    "$spacify" --quiet .
)
uv run --quiet --no-project --python "${COMPAT_PYTHON:?set COMPAT_PYTHON}" -- \
    python -I "$(dirname "$0")/ast-equal.py" "$src" "$work/spacify-only"

echo "ruff and spacify agree on $(find "$src" -name '*.py' -o -name '*.pyi' | wc -l) files"
