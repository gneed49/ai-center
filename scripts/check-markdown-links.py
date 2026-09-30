#!/usr/bin/env python3
"""Check local Markdown link targets without fetching remote URLs."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from urllib.parse import unquote


LINK_RE = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")
SCHEME_RE = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


def markdown_files(paths: list[Path]) -> list[Path]:
    files: list[Path] = []
    for path in paths:
        if path.is_dir():
            files.extend(sorted(path.rglob("*.md")))
        elif path.suffix.lower() == ".md":
            files.append(path)
        else:
            raise ValueError(f"Chemin Markdown introuvable: {path}")
    return sorted(set(files))


def visible_markdown(text: str) -> str:
    visible: list[str] = []
    inside_fence = False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            inside_fence = not inside_fence
            continue
        if not inside_fence:
            visible.append(line)
    return "\n".join(visible)


def extract_target(raw: str) -> str:
    raw = raw.strip()
    if raw.startswith("<") and ">" in raw:
        return raw[1 : raw.index(">")]
    return raw.split(maxsplit=1)[0]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path)
    args = parser.parse_args()

    errors: list[str] = []
    checked = 0
    try:
        files = markdown_files(args.paths)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2

    for markdown_path in files:
        text = visible_markdown(markdown_path.read_text(encoding="utf-8"))
        for match in LINK_RE.finditer(text):
            target = extract_target(match.group(1))
            if not target or target.startswith("#") or SCHEME_RE.match(target):
                continue
            path_part = unquote(target.split("#", 1)[0].split("?", 1)[0])
            if not path_part or path_part.startswith("/"):
                continue
            checked += 1
            resolved = (markdown_path.parent / path_part).resolve()
            if not resolved.exists():
                line = text.count("\n", 0, match.start()) + 1
                errors.append(f"{markdown_path}:{line}: cible absente: {target}")

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Liens Markdown locaux valides: {checked} cibles dans {len(files)} fichiers.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
