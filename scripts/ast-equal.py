"""Proves spacify preserved meaning: every file under <original> has the same AST as its copy under
<formatted>. A file CPython cannot parse must come back byte for byte, since spacify refuses it.

Usage: python -I scripts/ast-equal.py <original dir> <formatted dir>. The interpreter must parse
everything in the corpus, so it must be at least as new as the newest syntax there.
"""

import ast
import sys
import warnings
from pathlib import Path


def main() -> int:
    original, formatted = map(Path, sys.argv[1:3])
    # Invalid escape sequences in the corpus warn on every parse, which is noise here.
    warnings.simplefilter("ignore", SyntaxWarning)
    failures = 0
    checked = 0

    for path in sorted(original.rglob("*.py*")):
        if path.suffix not in {".py", ".pyi"}:
            continue

        relative = path.relative_to(original)
        before = path.read_bytes()
        after = (formatted / relative).read_bytes()
        try:
            tree = ast.dump(ast.parse(before))
        except (SyntaxError, ValueError):
            if before != after:
                print(f"spacify rewrote a file CPython cannot parse: {relative}", file=sys.stderr)
                failures += 1
            continue

        if tree != ast.dump(ast.parse(after)):
            print(f"spacify changed the AST of {relative}", file=sys.stderr)
            failures += 1
        checked += 1

    if failures:
        return 1

    print(f"spacify kept the AST of {checked} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
