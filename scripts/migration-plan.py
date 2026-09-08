#!/usr/bin/env python3
"""List the complete, unambiguous migration chain for the isolated upgrade test."""
from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys


BASELINE = "20260818003330_initial_domain.sql"
NAME = re.compile(r"([0-9]{14})_[A-Za-z0-9_]+\.sql\Z")


def discover(directory: Path) -> list[str]:
    if not directory.is_dir() or directory.is_symlink():
        raise ValueError("migration directory must be a regular directory")
    files = sorted(directory.iterdir(), key=lambda path: path.name)
    versions: set[str] = set()
    for path in files:
        match = NAME.fullmatch(path.name)
        if not match:
            raise ValueError("unrecognized migration name")
        if path.is_symlink() or not path.is_file():
            raise ValueError("migration must be a regular file")
        if not path.read_text().strip():
            raise ValueError("empty migration")
        version = match.group(1)
        if version in versions:
            raise ValueError("duplicate migration version")
        versions.add(version)
    names = [path.name for path in files]
    if not names or names[0] != BASELINE:
        raise ValueError("expected baseline must be the first migration")
    if len(names) < 2:
        raise ValueError("at least one upgrade migration is required")
    return names


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    try:
        print("\n".join(discover(args.directory)))
    except (OSError, UnicodeError, ValueError) as error:
        print(f"Migration plan refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
