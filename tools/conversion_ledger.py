#!/usr/bin/env python3
"""Generate CONVERSION-LEDGER.md from the authoritative module inventory.

The ledger deliberately gives credit only to modules marked ``done`` in
``tools/port_status.json``.  A partial port is useful progress, but it is not
1:1 completion.  Test modules are retained as oracle tasks so the whole
upstream checkout has an explicit, auditable row.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INVENTORY = ROOT / "tools" / "inventory.json"
DEFAULT_OUTPUT = ROOT / "CONVERSION-LEDGER.md"

PHASE_ORDER = {"P1": 1, "P2": 2, "P3": 3, "P4": 4, "P5": 5, "oracle": 6}
FOUNDATION_ROOTS = {
    "hermes_constants",
    "hermes_logging",
    "hermes_state",
    "hermes_state_common",
    "hermes_state_portability",
    "hermes_state_schema",
    "hermes_state_search",
    "hermes_time",
    "utils",
}
P2_ROOTS = {
    "agent",
    "batch",
    "providers",
    "run_agent",
    "tools",
    "trajectory_compressor",
    "mini_swe_runner",
}
P3_ROOTS = {"cli", "hermes_cli"}
P4_ROOTS = {"cron", "gateway", "plugins"}
P5_ROOTS = {"acp_adapter", "tui_gateway"}


def phase_for(module: str, is_test: bool) -> str:
    """Assign the repository phase owning a module or its oracle."""

    root = module
    if root.startswith("tests."):
        root = root[len("tests.") :]
    root = root.split(".", 1)[0]
    if root in FOUNDATION_ROOTS or module in {"agent.redact", "agent.session_activity"}:
        return "P1"
    if root in P2_ROOTS:
        return "P2"
    if root in P3_ROOTS:
        return "P3"
    if root in P4_ROOTS:
        return "P4"
    if root in P5_ROOTS:
        return "P5"
    # Test-only/top-level support files are still explicit oracle work.  They
    # do not silently disappear into an unowned bucket.
    return "oracle" if is_test else "P2"


def status_label(status: str) -> str:
    return {
        "done": "✅ done",
        "partial": "🟡 partial",
        "missing": "⬜ missing",
    }.get(status, f"❔ {status}")


def action_for(status: str, is_test: bool) -> str:
    if status == "done":
        return "Maintain parity evidence; no remaining task in this row."
    if status == "partial":
        if is_test:
            return "Close the documented oracle gap and promote to done."
        return "Close every documented seam, add parity evidence, then promote to done."
    if is_test:
        return "Read the upstream test; add the matching Rust parity coverage and evidence."
    return "Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit."


def pct(done: int, total: int) -> str:
    return f"{(100.0 * done / total) if total else 100.0:.2f}%"


def sorted_entries(modules: dict[str, dict]) -> list[tuple[str, dict]]:
    return sorted(modules.items(), key=lambda item: item[0])


def render(data: dict, inventory_path: Path) -> str:
    modules: dict[str, dict] = data["modules"]
    entries = sorted_entries(modules)
    all_done = sum(v["port_status"] == "done" for _, v in entries)
    all_partial = sum(v["port_status"] == "partial" for _, v in entries)
    all_missing = sum(v["port_status"] == "missing" for _, v in entries)
    prod = [(m, v) for m, v in entries if not v["is_test"]]
    prod_done = sum(v["port_status"] == "done" for _, v in prod)
    prod_partial = sum(v["port_status"] == "partial" for _, v in prod)
    prod_missing = sum(v["port_status"] == "missing" for _, v in prod)
    all_lines = sum(v["lines"] for _, v in entries)
    prod_lines = sum(v["lines"] for _, v in prod)
    done_lines = sum(v["lines"] for _, v in entries if v["port_status"] == "done")
    done_prod_lines = sum(v["lines"] for _, v in prod if v["port_status"] == "done")

    partials = [
        (m, v)
        for m, v in prod
        if v["port_status"] == "partial"
    ]
    missing_prod = [
        (m, v)
        for m, v in prod
        if v["port_status"] == "missing"
    ]
    missing_prod.sort(
        key=lambda item: (
            PHASE_ORDER.get(phase_for(item[0], False), 99),
            -item[1]["lines"],
            item[0],
        )
    )

    generated_at = data.get("generated_at", "unknown")
    generated_from = data.get("generated_from", "unknown")
    lines: list[str] = []
    lines.append("# Hermes Agent → Hermes Agent Rust Conversion Ledger")
    lines.append("")
    lines.append(
        f"**Current strict completion: {pct(all_done, len(entries))} of all tracked upstream modules "
        f"({all_done}/{len(entries)}).**"
    )
    lines.append(
        f"Production-only strict completion: **{pct(prod_done, len(prod))}** "
        f"({prod_done}/{len(prod)} production modules)."
    )
    lines.append("")
    lines.append(
        "> This file is generated from `tools/inventory.json`; update the source "
        "ledger in `tools/port_status.json`, regenerate the inventory, then run "
        "`python3 tools/conversion_ledger.py`. Only `done` counts toward the "
        "percentage. `partial` is intentionally zero credit until its stated "
        "parity seams are closed."
    )
    lines.append("")
    lines.append("## Current state")
    lines.append("")
    lines.append("| Scope | Done | Partial | Missing | Strict completion | Lines |")
    lines.append("|---|---:|---:|---:|---:|---:|")
    lines.append(
        f"| All tracked modules | {all_done} | {all_partial} | {all_missing} | "
        f"{pct(all_done, len(entries))} | {done_lines:,}/{all_lines:,} done LOC |"
    )
    lines.append(
        f"| Production modules | {prod_done} | {prod_partial} | {prod_missing} | "
        f"{pct(prod_done, len(prod))} | {done_prod_lines:,}/{prod_lines:,} done LOC |"
    )
    lines.append("")
    lines.append(f"Inventory source: `{generated_from}` at `{generated_at}`.")
    lines.append("")
    lines.append("## Definition of 100.00%")
    lines.append("")
    lines.append("The conversion is complete only when all of these are true:")
    lines.append("")
    lines.extend(
        [
            "1. Every row below is `✅ done`; no production or oracle module remains `partial` or `missing`.",
            "2. Each production module has Rust behavior, upstream-derived parity tests, and a line-by-line review of errors, fail-open paths, precedence, caching, and lifecycle semantics.",
            "3. Each upstream test/oracle row has equivalent Rust coverage or an explicit, resolved reason why the behavior is covered elsewhere; no ignored test hides a parity gap.",
            "4. `cargo build --workspace` and `cargo test --workspace` are green, with the exact commands and evidence tier recorded in `PLAN.md`.",
            "5. All intentional divergences have been removed or explicitly signed off; config/provider/gateway/platform seams are wired, not merely injectable placeholders.",
            "6. `PLAN.md`, `tools/port_status.json`, `tools/inventory.json`, and this ledger agree, and every logical unit is committed and pushed.",
        ]
    )
    lines.append("")
    lines.append("## Active partial modules")
    lines.append("")
    if partials:
        lines.append("| Module | Phase | Upstream LOC | Required closure |")
        lines.append("|---|---|---:|---|")
        for module, value in partials:
            lines.append(
                f"| `{module}` | {phase_for(module, value['is_test'])} | {value['lines']:,} | "
                "Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |"
            )
    else:
        lines.append("None.")
    lines.append("")
    lines.append("## Recommended next production units")
    lines.append("")
    lines.append(
        "Work bottom-up by phase. The list is regenerated from missing production rows, "
        "with the current phase and largest modules first; completing a different unit "
        "is valid when its dependency boundary is better prepared."
    )
    lines.append("")
    lines.append("| Order | Module | Phase | Upstream LOC | Task |")
    lines.append("|---:|---|---|---:|---|")
    for order, (module, value) in enumerate(missing_prod[:20], 1):
        lines.append(
            f"| {order} | `{module}` | {phase_for(module, value['is_test'])} | {value['lines']:,} | "
            "TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |"
        )
    lines.append("")
    lines.append("## Operating protocol")
    lines.append("")
    lines.extend(
        [
            "1. Load this file, `PLAN.md` §5, `tools/inventory.json`, and the session log; take the next dependency-safe row.",
            "2. Read the pinned upstream module and its tests. Write Rust parity tests first and label each `unit`, `mock`, or `live`.",
            "3. Implement exact behavior. Keep `// PARITY:` references and document every intentional divergence in code and `PLAN.md`.",
            "4. Run `cargo build --workspace` and `cargo test --workspace`; never record a red commit.",
            "5. Set the module status in `tools/port_status.json`, run `HERMES_UPSTREAM=... tools/inventory.sh`, then run this generator.",
            "6. Commit one logical module at most per commit and push it; append the exact evidence and next unit to `PLAN.md` §7.",
        ]
    )
    lines.append("")
    lines.append("## Complete upstream module task ledger")
    lines.append("")
    lines.append(
        "Every upstream Python module in the inventory has one row. Production rows are "
        "conversion tasks; test rows are parity-oracle tasks. A test row is not silently "
        "treated as complete merely because a production port exists."
    )
    lines.append("")
    lines.append("| Module task | Kind | Phase | Upstream LOC | Status | Remaining action |")
    lines.append("|---|---|---|---:|---|---|")
    for module, value in entries:
        is_test = bool(value["is_test"])
        kind = "oracle/test" if is_test else "production"
        phase = phase_for(module, is_test)
        lines.append(
            f"| `{module}` | {kind} | {phase} | {value['lines']:,} | "
            f"{status_label(value['port_status'])} | {action_for(value['port_status'], is_test)} |"
        )
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append(
        "Generated by `tools/conversion_ledger.py`; do not hand-edit the generated rows."
    )
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    data = json.loads(args.inventory.read_text(encoding="utf-8"))
    args.output.write_text(render(data, args.inventory), encoding="utf-8")
    entries = data["modules"]
    done = sum(v["port_status"] == "done" for v in entries.values())
    print(f"wrote {args.output} ({len(entries)} module rows; {pct(done, len(entries))} strict completion)")


if __name__ == "__main__":
    main()
