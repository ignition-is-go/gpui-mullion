#!/usr/bin/env python3
"""Reject screenshots which contain no rendered RGB variation."""
import sys
from pathlib import Path
from PIL import Image


def is_uniform(path: Path) -> bool:
    with Image.open(path) as image:
        return all(low == high for low, high in image.convert("RGB").getextrema())


def main() -> int:
    captures = [Path(argument) for argument in sys.argv[1:]]
    if not captures:
        print("usage: check_capture.py CAPTURE.png [...]", file=sys.stderr)
        return 2
    rejected = [path for path in captures if is_uniform(path)]
    if rejected:
        for path in rejected:
            print(f"uniform visual capture rejected before comparison: {path}", file=sys.stderr)
        return 1
    print(f"pixel sanity passed for {len(captures)} captures")
    return 0


if __name__ == "__main__":
    sys.exit(main())
