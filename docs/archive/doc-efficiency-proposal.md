# Doc-efficiency proposal (spec r1, 2026-09-17 — SUPERSEDED by r2)

> R1 verdict 1R/4C; r2 (`doc-efficiency-proposal-r2.md`) is authoritative.
> R1 adjudication: S1-number wrong (32→143 file-wide); S3-B1/B2/B3/B4,
> S2, S4, S5-CONFIRMED; fix 3 (single log) DROPPED as harmful.

## Problem

Per-turn doc load is ~44k words minimum (PLAN 32k + HANDOFF 8.5k + AGENTS
1k + misc), plus a 230k-word generated ledger one bad read away from
~300k tokens. PLAN §7 grows unboundedly (32 dated entries); HANDOFF
duplicates it; board/long-horizon docs are read-every-turn but
read-once-useful.

## Proposed fixes

1. **Ledger rule**: AGENTS.md gains "query `tools/port_status.json` /
   `tools/inventory.json` via `python3 -c`; never read
   CONVERSION-LEDGER.md whole (generated, 230k words)". CONVERSION-LEDGER
   header gains "generated-only, agents query the JSON".
2. **HANDOFF cap**: keep resume state + last 2 batches only; move older
   history to `docs/archive/handoff-<pin>.md` at each re-pin.
3. **Single session log**: HANDOFF owns history; PLAN §7 keeps a pointer
   ("see HANDOFF.md") + current numbers only. New entries go to HANDOFF
   alone; pre-commit hook requirement updated to match.
4. **Archive read-once docs**: `docs/board-review-*.md`,
   `docs/long-horizon-*.md` older than current goal → `docs/archive/`
   with pointer lines left behind.

## Gate rule

Implement only on unanimous BUILD with all material blockers closed
against live sources.
