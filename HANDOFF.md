# Hermes Agent Rust — Next-session handoff

Date: 2026-08-31 (Pacific/Auckland), session 4da.

## Resume point

Repository: `/run/media/mustbearnold/Projects/AI Agents/Hermes-Agent-Rust`

Pinned upstream commit: `b9aa928`. The checkout actually used and validated is
`/run/media/mustbearnold/Projects/Research/hermes-agent-repo` (AGENTS.md and
`tools/inventory.sh` were corrected to this path this session). **Upstream
HEAD has advanced far past the pin** — always regenerate the inventory against
a pinned worktree:

```bash
git -C /run/media/mustbearnold/Projects/Research/hermes-agent-repo \
    worktree add --detach /tmp/hermes-upstream-b9aa928 b9aa928
HERMES_UPSTREAM=/tmp/hermes-upstream-b9aa928 bash tools/inventory.sh
python3 tools/conversion_ledger.py
git -C /run/media/mustbearnold/Projects/Research/hermes-agent-repo \
    worktree remove /tmp/hermes-upstream-b9aa928
```

Current branch/HEAD: `main` carries this session's four units committed with
this documentation checkpoint. Previous: `d138528` (4d9's
`browser_camofox_state`/`focus_pane_tool`/`cwd_placeholder`), `e6079b9`,
`a06d8fb`, `aedb1f2`, `2468dc7`.

## What landed this session (4da)

Four units across two crates, all red-first, 34 new parity tests:

- `hermes_cli/main.py` `_read_packed_ref` + `_read_git_revision_fingerprint`
  → `hermes-cli::git_revision` (8 tests, source-derived — no dedicated
  upstream test file; gap noted). Real `.git` fixtures: detached HEAD, loose
  refs, packed-refs (comments/peel lines skipped), the `unresolved` marker,
  worktree `.git`-file indirection with `commondir`, OSError fail-open.
  `hermes_cli.main` is marked `partial` (two private helpers of a ~5k-line
  module).
- `gateway/code_skew.py` → `hermes-gateway::code_skew` (10 tests mirroring
  `tests/test_code_skew.py`; `TestModelSwitchSkewGuard` stays PENDING with
  the unported `gateway.slash_commands`). Test seams
  (`set_fingerprint_override_for_tests`/`reset_for_tests`) stand in for the
  upstream `monkeypatch` fixture.
- `gateway/cgroup_cleanup.py` → `hermes-gateway::cgroup_cleanup` (8 tests
  mirroring `tests/gateway/test_cgroup_cleanup.py`, incl. one live SIGKILL of
  a spawned `sleep` child). Only successful kills count toward the tally;
  `reap_cgroup(None)` is never exercised in tests (it would kill the test
  process's own cgroup).
- `gateway/rich_sent_store.py` → `hermes-gateway::rich_sent_store` (8 tests,
  source-derived; gap noted). Python-falsiness guards, fail-open lookup
  (missing/corrupt/non-dict/falsy-`t`), 1000-entry trim by oldest `ts` with
  a stable sort (same-second timestamps trim in insertion order).

`hermes-gateway` gains `hermes-cli`, `hermes-constants`, `hermes-time`,
`serde_json`, `libc` — all still below the agent/tools layers.

## Exact working-tree state

Session 4da **changed the ledger**: 97 done / 15 partial / 3,770 missing
tracked modules and 97 done / 15 partial / 991 missing production modules —
**2.50%** tracked and **8.79%** production strict completion — regenerated
against the pinned `b9aa928` worktree (commands above), with
`cargo build --workspace` and the serialized workspace run green at 1,361
tests / 5 ignored.

## Next actions, in order

1. Keep taking the small-module batch. Same-shape candidates:
   `tools.close_terminal_tool` (62 LOC), `tools.skill_provenance` (78 LOC),
   `gateway.session_stall` (121 LOC), `gateway.readiness` (138 LOC), and the
   remaining small oracle-backed `tools/`/`gateway/`/`hermes_cli` leaves.
2. Continue the remaining `agent.auxiliary_client` request/response lifecycle
   and the deferred higher-layer seams (desktop/gateway emitter wiring,
   plugin registry).
3. Keep ownership disjoint; commit and publish each logical unit immediately.

## Current conversion ledger

97 done / 15 partial / 3,770 missing tracked (**2.50%** strict completion);
97 done / 15 partial / 991 missing production (**8.79%**). Regenerated via
`tools/inventory.sh` (pinned worktree) + `python3 tools/conversion_ledger.py`.

## Fidelity notes

- Session 4da: `code_skew._short` keeps Python's `sha or fingerprint`
  empty-string fallback; `record_boot_fingerprint` is idempotent like the
  upstream module global. `cgroup_cleanup` per-PID kills map
  ProcessLookupError/PermissionError to skip-without-counting;
  `parse_own_cgroup_path` reproduces the `^0::(.+)$` anchor exactly (bare
  `0::` is not a match, whitespace-only is). `rich_sent_store`'s
  `record` guards run before any disk touch, and `lookup`'s non-dict
  document arm reproduces upstream's `AttributeError`. `git_revision`
  uses lossy reads for upstream's `errors="replace"` and canonicalizes
  non-strictly like `Path.resolve()`.
- Layering: `hermes-gateway` importing `hermes-cli` matches upstream's
  `gateway.code_skew` importing `hermes_cli.main`; both stay below the
  agent/tools layers.

## Verification evidence

- `/home/mustbearnold/.cargo/bin/cargo build --workspace` — green.
- `cargo test -p hermes-gateway -p hermes-cli` — 34 new tests green (86
  total across the two crates).
- Serialized `/home/mustbearnold/.cargo/bin/cargo test --workspace --
  --test-threads=1` — 1,361 passed, 0 failed, 5 intentional ignores.
- `cargo clippy -p hermes-gateway -p hermes-cli --all-targets` — clean on
  all new code.
- `rustfmt --edition 2021 --check` — clean on all changed files.
- `git diff --check` — clean.

## First command tomorrow

```bash
git status --short
git fetch origin main
git rev-parse origin/main
git log --oneline -5
git ls-remote origin refs/heads/main
```
