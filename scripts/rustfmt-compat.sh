#!/usr/bin/env bash
# Proves spacify and rustfmt never fight over a tree, in both orders, and that spacify alone keeps
# every file's meaning:
#
#   rustfmt → spacify             then both checks pass at once;
#   spacify → rustfmt → spacify   then both checks pass (rustfmt can join a wrapped statement onto
#                                 one line, which may make it eligible for collapse-groups once more);
#   spacify                       then every file has the tokens it started with (tokens_equal).
#
# Each copy loses its rustfmt.toml, the way `ruff format --isolated` ignores a project's settings,
# and gets a spacify.toml enabling Rust alone, so the other languages in the tree stay out of it.
#
# Usage: scripts/rustfmt-compat.sh <source dir> <edition> [spacify binary]. Needs `rustfmt` on PATH
# and the `tokens_equal` example built in release mode (the Makefile builds it).
set -euo pipefail

src=${1:?usage: $0 <source dir> <edition> [spacify binary]}
edition=${2:?usage: $0 <source dir> <edition> [spacify binary]}
spacify=$(realpath "${3:-target/release/spacify}")
tokens_equal=$(realpath target/release/examples/tokens_equal)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

rustfmt_all() {
    find . -name '*.rs' -print0 | xargs -0 rustfmt --edition "$edition" "$@"
}

check() {
    rustfmt_all --check >/dev/null || { echo "rustfmt would still change $1" >&2; exit 1; }
    "$spacify" --check --quiet . || { echo "spacify would still change $1" >&2; exit 1; }
}

for copy in rustfmt-first spacify-first spacify-only; do
    cp -r "$src" "$work/$copy"
    find "$work/$copy" \( -name rustfmt.toml -o -name .rustfmt.toml \) -delete
    echo 'languages = ["rust"]' > "$work/$copy/spacify.toml"
done

(
    cd "$work/rustfmt-first"
    rustfmt_all
    "$spacify" --quiet .
    check "rustfmt → spacify"
)

(
    cd "$work/spacify-first"
    "$spacify" --quiet .
    rustfmt_all
    "$spacify" --quiet .
    check "spacify → rustfmt → spacify"
)

(
    cd "$work/spacify-only"
    "$spacify" --quiet .
)
"$tokens_equal" "$src" "$work/spacify-only"

echo "rustfmt and spacify agree on $(find "$src" -name '*.rs' | wc -l) files"
