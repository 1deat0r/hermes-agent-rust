#!/usr/bin/env python3
"""Refresh generated conversion documents and the README status snapshot.

The pre-commit hook calls this script only when a source, parity, inventory, or
hook change is staged. It intentionally updates only generated/status fields;
PLAN.md and HANDOFF.md remain human-maintained so the hook cannot invent parity
evidence or a next unit of work.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def run(command: list[str], *, env: dict[str, str]) -> None:
    subprocess.run(command, cwd=ROOT, env=env, check=True)


def update_readme_status() -> None:
    inventory_path = ROOT / "tools" / "inventory.json"
    data = json.loads(inventory_path.read_text(encoding="utf-8"))
    status = data["summary"]["status_counts"]
    production = data["summary"]["prod_status_counts"]
    total = sum(status.values())
    production_total = sum(production.values())

    def percentage(done: int, count: int) -> str:
        return f"{100.0 * done / count if count else 100.0:.2f}%"

    all_line = (
        f"- **All tracked modules:** **{percentage(status['done'], total)}** — "
        f"`{status['done']:,} / {total:,}` done, `{status['partial']:,}` partial,\n"
        f"  `{status['missing']:,}` missing."
    )
    production_line = (
        f"- **Production modules:** **{percentage(production['done'], production_total)}** — "
        f"`{production['done']:,} / {production_total:,}` done, "
        f"`{production['partial']:,}` partial,\n"
        f"  `{production['missing']:,}` missing."
    )

    readme_path = ROOT / "README.md"
    readme = readme_path.read_text(encoding="utf-8")
    all_pattern = re.compile(
        r"(?m)^- \*\*All tracked modules:\*\* \*\*[^*]+\*\* — `[^`]+` done, `[^`]+` partial,\n"
        r"  `[^`]+` missing\."
    )
    production_pattern = re.compile(
        r"(?m)^- \*\*Production modules:\*\* \*\*[^*]+\*\* — `[^`]+` done, `[^`]+` partial,\n"
        r"  `[^`]+` missing\."
    )
    readme, all_count = all_pattern.subn(all_line, readme, count=1)
    readme, production_count = production_pattern.subn(
        production_line, readme, count=1
    )
    if all_count != 1 or production_count != 1:
        raise RuntimeError(
            "README.md status snapshot was not found in the expected format; "
            "update it manually before committing"
        )
    readme_path.write_text(readme, encoding="utf-8")
    print(
        "refreshed README.md status: "
        f"all={percentage(status['done'], total)}, "
        f"production={percentage(production['done'], production_total)}"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--upstream", type=Path, required=True)
    args = parser.parse_args()

    upstream = args.upstream.expanduser().resolve()
    if not upstream.is_dir():
        print(f"upstream checkout does not exist: {upstream}", file=sys.stderr)
        return 2

    env = os.environ.copy()
    env["HERMES_UPSTREAM"] = str(upstream)
    run([str(ROOT / "tools" / "inventory.sh")], env=env)
    run([sys.executable, "tools/conversion_ledger.py"], env=env)
    update_readme_status()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
