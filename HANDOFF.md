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

Ten units across four crates, all red-first, 74 new parity tests:

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

Batch 2: `tools/skill_provenance.py` → `hermes-tools::skill_provenance`
stub filled (5 tests; scoped thread = `copy_context().run`; token carries
the prior value like `ContextVar.reset`) and `gateway/session_stall.py` →
`hermes-gateway::session_stall` (8 tests; numeric-string `float()` accepts,
non-finite fall-through, negative-idle clamp; the `GatewayRunner`
notify-once loop stays PENDING with `gateway.run`).

Batch 3: `gateway/readiness.py` → `hermes-gateway::readiness` (9 tests
mirroring both cases in `tests/gateway/test_readiness.py` plus
source-derived probe cases: corrupt db degrades without repair, empty YAML
= defaults, draining-is-ok gateway states, `state or status` connected
counting, negative-counter clamps). The crate gains `rusqlite` and
`serde_yaml` for the probes; `hermes-state` remains dev-only.

Batch 4: `tools/open_preview_tool.py` → `hermes-tools::open_preview_tool`
(7 tests; registry entry oracle `desktop_ui`/no check_fn, emitter-panic
except-arm reproduction, no-emitter desktop-only arm, and the
`_normalize_target` grammar — bare-domain https, localhost/loopback http
with re.I and `[::1]`, sub-2-char TLDs bare, path/scheme/`file:`/`~`
pass-through, backtick/whitespace stripping).

Batch 5: `hermes_cli/timefmt.py` → `hermes-cli::timefmt` (6 source-derived
tests, gap noted; falsy-ts → "?", truncating minute/hour/day divisions,
24–48h "yesterday" band, future timestamps read "just now", local
`%Y-%m-%d` past a week; `chrono` added to the crate) and
`agent/iteration_budget.py` → `hermes-agent::iteration_budget` (6
source-derived tests, gap noted; consume/refund contract, remaining clamp
under a shrunk public `max_total`, independent per-agent budgets, 8-thread
race stays exactly at the cap).

`hermes-gateway` also gains `hermes-cli`, `hermes-constants`, `hermes-time`,
`serde_json`, `libc` — all still below the agent/tools layers.
`tools.close_terminal_tool` was skipped: blocked on the 2,937-LOC
`tools.process_registry`.

## Exact working-tree state

Session 4da **changed the ledger**: 99 done / 15 partial / 3,768 missing
tracked modules and 99 done / 15 partial / 989 missing production modules —
**2.65%** tracked and **9.34%** production strict completion — regenerated
against the pinned `b9aa928` worktree (commands above), with
`cargo build --workspace` and the serialized workspace run green at 1,401
tests / 6 ignored (the 6th is skill_provenance's intentional `ignore` doc
example).

## Next actions, in order

1. Keep taking the small-module batch. Same-shape candidates: the
   remaining small oracle-backed `tools/`/`gateway/`/`hermes_cli` leaves
   (`tools.close_terminal_tool`'s siblings). The big
   `tools.process_registry` (2,937 LOC) unblocks `tools.close_terminal_tool`
   and `tools.read_terminal_tool` callers when taken.
2. Continue the remaining `agent.auxiliary_client` request/response lifecycle
   and the deferred higher-layer seams (desktop/gateway emitter wiring,
   plugin registry).
3. Keep ownership disjoint; commit and publish each logical unit immediately.

## Current conversion ledger

99 done / 15 partial / 3,768 missing tracked (**2.55%** strict completion);
103 done / 15 partial / 985 missing production (**9.34%**). Regenerated via
`tools/inventory.sh` (pinned worktree) + `python3 tools/conversion_ledger.py`.

## Fidelity notes

- Session 4da (batch 5): `timefmt.relative_time` reproduces Python falsiness
  (`None`/`0.0` → "?"), truncating `int(delta/x)` divisions, the
  `delta < 60` arm that makes future timestamps read "just now", and the
  local-zone `%Y-%m-%d` date branch. `IterationBudget.max_total` stays a
  plain public field (upstream mutates it), so `remaining`'s
  `max(0, ...)` clamp is observable.
- Session 4da (batch 4): `open_preview_tool`'s `_normalize_target` pins both
  upstream regexes verbatim (LazyLock) — re.I matters (LOCALHOST:3000 gains
  http://), `\w` stays unicode-aware, and the strip order is
  whitespace → backticks → whitespace, matching
  `raw.strip().strip("`").strip()`. The emitter except arm is reproduced
  via catch_unwind with the panic message surfaced, per read_preview_tool.
- Session 4da (batch 3): `readiness` opens the state db read-only via URI
  with a 1s busy timeout so probes never compete with writers; empty YAML
  documents count as defaults (`safe_load` → None); `disk_usage` reproduces
  `shutil.disk_usage` (`free` = `f_bavail`); error details carry short type
  labels only, never messages.
- Session 4da (batch 2): `session_stall` idle resolution mirrors Python
  `float()` semantics — numeric strings parse, non-numeric/non-finite
  `seconds_since_activity` falls through to `last_activity_at`/`_ts`,
  negative values clamp to 0.0, and an empty mapping is falsy like
  `if not activity`. `skill_provenance`'s set-boundary coercion is
  `origin or "foreground"`.
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
- `cargo test -p hermes-gateway -p hermes-cli -p hermes-tools -p
  hermes-agent` — 74 new tests green.
- Serialized `/home/mustbearnold/.cargo/bin/cargo test --workspace --
  --test-threads=1` — 1,401 passed, 0 failed, 6 intentional ignores.
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
