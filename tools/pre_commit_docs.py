#!/usr/bin/env python3
"""Enforce the repository documentation checkpoint before a commit."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REQUIRED_SOURCE_DOCS = ("PLAN.md", "HANDOFF.md")
GENERATED_DOCS = ("tools/inventory.json", "CONVERSION-LEDGER.md")
DESCRIPTION_FILE = ".github/repository-description.txt"


def git(*args: str) -> list[str]:
    result = subprocess.run(
        ["git", *args],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    return [line for line in result.stdout.splitlines() if line]


def is_source_change(path: str) -> bool:
    if path in {
        "Cargo.toml",
        "Cargo.lock",
        "tools/inventory.py",
        "tools/inventory.sh",
        "tools/conversion_ledger.py",
    }:
        return True
    return path.startswith(
        (
            "crates/",
            "upstream/",
            "scripts/",
            "examples/",
            "tools/",
            ".githooks/",
            ".github/",
        )
    ) and path not in GENERATED_DOCS and path != DESCRIPTION_FILE


def resolve_upstream() -> Path | None:
    explicit = os.environ.get("HERMES_UPSTREAM")
    configured = subprocess.run(
        ["git", "config", "--get", "hermes.upstream"],
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout.strip()
    candidates: list[str] = []
    if explicit:
        candidates.append(explicit)
    if configured:
        candidates.append(configured)
    # AGENTS.md is the fixed reference point. The removable-media fallback is
    # the actual checkout recorded in HANDOFF.md for this machine.
    candidates.extend(
        [
            "/home/mustbearn/Projects/Research/hermes-agent-repo",
            str(ROOT.parent.parent / "Research" / "hermes-agent-repo"),
        ]
    )
    seen: set[str] = set()
    for candidate in candidates:
        path = str(Path(candidate).expanduser())
        if path in seen:
            continue
        seen.add(path)
        resolved = Path(path).resolve()
        if resolved.is_dir():
            return resolved
    return None


def fail(message: str) -> int:
    print(f"pre-commit: {message}", file=sys.stderr)
    return 1


def main() -> int:
    staged_before = set(git("diff", "--cached", "--name-only", "--diff-filter=ACMRT"))
    if not staged_before:
        return fail("the index is empty")

    unstaged = set(git("diff", "--name-only", "--diff-filter=ACMRT"))
    source_change = any(is_source_change(path) for path in staged_before)

    if source_change:
        dirty_generated = sorted(unstaged.intersection(GENERATED_DOCS))
        if dirty_generated:
            return fail(
                "generated docs have unstaged edits; stage or stash before committing: "
                + ", ".join(dirty_generated)
            )
        dirty_required = sorted(unstaged.intersection(REQUIRED_SOURCE_DOCS))
        if dirty_required:
            return fail(
                "required docs have unstaged edits; stage the intended versions before "
                "committing: "
                + ", ".join(dirty_required)
            )
        if "README.md" in unstaged:
            return fail(
                "README.md has unstaged edits; stage or stash the intended version "
                "before the hook refreshes its status snapshot"
            )

        upstream = resolve_upstream()
        if upstream is None:
            return fail(
                "cannot refresh inventory/ledger because the pinned upstream checkout "
                "is unavailable; set HERMES_UPSTREAM or git config hermes.upstream"
            )
        subprocess.run(
            [sys.executable, "tools/refresh_docs.py", "--upstream", str(upstream)],
            cwd=ROOT,
            check=True,
        )
        subprocess.run(
            ["git", "add", "--", *GENERATED_DOCS, "README.md"],
            cwd=ROOT,
            check=True,
        )

    staged_after = set(git("diff", "--cached", "--name-only", "--diff-filter=ACMRT"))
    missing = [path for path in REQUIRED_SOURCE_DOCS if path not in staged_after]
    if source_change and missing:
        return fail(
            "source/parity changes require staged documentation updates: "
            + ", ".join(missing)
        )

    description_path = ROOT / DESCRIPTION_FILE
    if not description_path.is_file():
        return fail(f"missing tracked GitHub description source: {DESCRIPTION_FILE}")
    description = description_path.read_text(encoding="utf-8").strip()
    if not description or len(description) > 350 or "\n" in description:
        return fail(
            f"{DESCRIPTION_FILE} must contain one non-empty GitHub description line "
            "of at most 350 characters"
        )

    print("pre-commit: documentation checkpoint passed")
    if source_change:
        print(
            "pre-commit: refreshed and staged tools/inventory.json, "
            "CONVERSION-LEDGER.md, and README.md status"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
