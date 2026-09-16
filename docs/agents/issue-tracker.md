# Issue tracker: Local Markdown (Hermes-Agent-Rust convention)

Issues and specs for this repo live in the conversion ledger flow, not in
`.scratch/`:

- **Parity matrix + next unit**: `PLAN.md` §5 (matrix) and §7 (session log).
- **Resume/next actions**: `HANDOFF.md`.
- **Machine-readable rows**: `tools/port_status.json` (status source) →
  `tools/inventory.json` → `CONVERSION-LEDGER.md`.
- One logical unit per commit; triage state is the `port_status` value
  (`missing` = backlog, `partial` = in-progress/blocked-on-seam with the seam
  in `retarget_note`, `done` = closed with evidence).

## When a skill says "publish to the issue tracker"

Append to `PLAN.md` §7 and update `HANDOFF.md` next-actions (both are
human-maintained by repo protocol — the pre-commit hook requires them
staged alongside source changes). For multi-ticket efforts, mirror the
ticket list in `docs/board-review-*.md` or the relevant phase doc.

## When a skill says "fetch the relevant ticket"

Read `PLAN.md` §7 (latest entries first) and `tools/port_status.json` for
the module row. The user normally passes the module name directly.

## Wayfinding operations

Used by `/wayfinder`: the **map** is `CONVERSION-LEDGER.md` ("Recommended
next production units"); child tickets are module rows in
`tools/port_status.json`; blocking = dependency direction (bottom-up crates;
a row is unblocked when its lower-layer seams are `done`); frontier = the
next dependency-safe unit named in `HANDOFF.md`.
