# Board review R1 — retarget + re-cert batch 1 (spec r1)

Date: 2026-09-16. Under review: commits `24c4a0c` (retarget to 5d59366 +
P6 desktop scope) and `03b2f6f` (re-cert batch 1: 22 promotions + TDD port
of `classify_jwks_lookup_error`), plus the recorded long-horizon plan.

## Gate rule

Proceed to the long-horizon 100% plan only when every seat returns BUILD
(or CONDITIONAL with all items closed) **and** every material blocker has a
verified closure against live sources. Unanimity necessary, not sufficient.

## Claims under review

1. Pin `5d59366` is latest `origin/main` as of 2026-09-16 NZ.
2. Inventory: 8,895 tracked / 3,481 prod; 27 done / 142 partial; P6 desktop
   TS tracking (2,539 total = 1,506 prod @ ~381k LOC + 1,033 oracles) with
   stated exclusions.
3. 18 promotions are AST-identical-modulo-docstrings vs `b9aa928`.
4. 4 promotions are refactor-only (behavior-identical) drifts.
5. `classify_jwks_lookup_error` port is faithful to the oracle
   (`test_opaque_bearer_not_unreachable.py`), TDD (RED observed).
6. Workspace green: 1,786 passed / 0 failed; `git diff --check` clean;
   3 warnings pre-existing.
7. Version literals 0.21.3 / 2026.9.14 correct at the new pin.
8. Long-horizon 100% plan (to be set after gate) is credible.

## Board record

| Round | Seats | Verdicts | Tally |
|---|---|---|---|
| R1 | S1 rust-parity, S2 statistics/ledger, S3 adversarial, S4 desktop/tauri, S5 test-oracle | S1 BUILD; S2 CONDITIONAL (1); S3 REJECT (4); S4 CONDITIONAL (3); S5 CONDITIONAL (4) | 1B/3C/1R — revise required |
| R2 | same five, verify-by-quote + S6 cold-read | S1 BUILD; S2 BUILD; S3 BLOCK (1 wording); S4 BUILD; S5 APPROVE; S6 CONDITIONAL (record items) | 4B/1 block/1C — one relabel + reconciliation |
| R3 | S3 wording confirm + S6 record confirm | S3 BUILD; S6 BUILD | **UNANIMOUS BUILD — GATE PASSED** |

## Adjudication log

- S2-B1 (GATES G1 stale evidence line) — CONFIRMED. Fixed: evidence line → 27/142.
- S3-B1 ("AST-identical" label) — PARTLY. Raw-AST check confirms `pass`-removal deltas in 8/18 rows; re-proof shows 10 RAW-IDENT + 8 PASS-ONLY (redundant-`pass` in docstring bodies, behavior-neutral). No substantive deltas. Ruling: promotions stand; 8 notes relabeled to the precise wording. Live evidence: `git diff b9aa928..5d59366 -- agent/errors.py` + classifier script output (session log).
- S3-B2 (refactor-only proof) — PARTLY. `cwd_placeholder._truthy_env` was dead code at old pin (def-only, zero call sites); docker fold is logic-identical conjunction merge. Camofox compat block verified (`check_compat_pointers.py` gate). Ruling: promotions stand with recorded proofs.
- S3-B3 (JWKS taxonomy) — CONFIRMED in substance. `PyJWKError` MRO verified live (`PyJWTError`, neither client nor token branch) → upstream final-arm ProviderError. Fixed: `JwksLookupFailure::KeyMaterial` → Provider + test. Cancellation half REFUTED-as-designed: enum seam cannot represent the fold; documented as caller-owned.
- S3-B4 (stub-passing suites) — REFUTED. `parity_fireworks.rs` holds 2 tests / ~20 assertions (identity, headers, aliases, fallbacks, router-exclusion). S3 counted `#[test]` attributes, not assertions.
- S4-B1 (fixtures wording) — PARTLY. No literal `__fixtures__/` dir exists upstream; reworded doc to actual behavior (defensive entry + oracle-hint tracking).
- S4-B2 (no hermes-desktop crate) — CONFIRMED. Fixed: scaffold crate + parity_scaffold test, PLAN P6 linkage.
- S4-B3 (path provenance) — CONFIRMED. Fixed: mapping rule documented in `tools/inventory.py`.
- S5-B1 (prefix asserts) — CONFIRMED. Fixed: `startswith` asserts on both arms.
- S5-B2 (order-sensitivity test) — CONFIRMED. Fixed: `subclass_failures_classify_before_parent_kinds`.
- S5-B3 (RED evidence) — CONFIRMED. Fixed: honest TDD note in test header (compile-fail observed, no log artifact).
- S5-B4 (1786 count) — CLOSED live: full serial run EXIT=0, 191 suites ok, 1786/0 (an intermediate "943/1" was a truncated-pipe awk artifact; rerun clean twice).
