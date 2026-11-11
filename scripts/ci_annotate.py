"""Turn failing CI logs into GitHub error annotations.

Agents that cannot download Actions logs (for example cloud sandboxes with
access to api.github.com only) read these through the check-run annotations
API: GET /repos/{owner}/{repo}/check-runs/{id}/annotations.

usage: python scripts/ci_annotate.py ci-*.log
"""

import re
import sys
from pathlib import Path

INTERESTING = re.compile(
    r"^(error(\[E\d+\])?:|warning: unused|test .* FAILED|---- |Diff in|thread '.*' panicked)"
    r"|panicked at"
)
MAX_LINES = 9  # GitHub keeps 10 error annotations per step; the tail is the 10th.
TAIL = 30


def escape(text: str) -> str:
    return text.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def annotate(path: Path) -> None:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    picked = []
    for index, line in enumerate(lines):
        if INTERESTING.search(line):
            # Keep the source location rustc prints right after an error line.
            context = lines[index : index + 4] if line.startswith("error") else [line]
            picked.append("\n".join(context))
        if len(picked) >= MAX_LINES:
            break
    for entry in picked:
        print(f"::error title={path.name}::{escape(entry)}")
    print(f"::error title={path.name} tail::{escape(chr(10).join(lines[-TAIL:]))}")


def main() -> None:
    for name in sys.argv[1:]:
        path = Path(name)
        if path.is_file():
            annotate(path)


if __name__ == "__main__":
    main()
