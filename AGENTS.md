# AGENTS.md — Operating Protocol for Agent Turns

> This file is the turn-to-turn playbook. Any agent (or human) working in this
> repo follows it. It exists so that every turn ends with the next turn being
> easier. **A stale PLAN/inventory is a process failure — never end a turn with
> them stale.**

## 1. Fixed reference points (do not rediscover)

- Upstream: `/home/mustbearn/Projects/Research/hermes-agent-repo` (pinned @ `b9aa928`)
- Master plan + parity matrix: `PLAN.md`
- Machine-readable ledger: `tools/inventory.json` (regenerate: `tools/inventory.sh`)
- Rust workspace root: this directory. Crates under `crates/`.

## 2. Turn protocol (in order)

1. **Load state.** Read `PLAN.md` §5 (parity matrix), `tools/inventory.json`'s
   `port_status` summary, and the session log. Determine the *next unit of work*
   (module or module-section in the current phase). Do not re-derive from the
   upstream repo what the ledger already records.
2. **TDD against the oracle.** For the chosen unit:
   a. Read the upstream module **and its tests**.
   b. Write the Rust parity tests first (mirroring upstream cases),
      marking the tier (`unit`|`mock`|`live`).
   c. Implement the port until tests pass.
3. **Line-by-line self-review.** Compare Rust impl against upstream line by
   line: error paths, fail-open behaviors, precedence orders, caching
   semantics. Note any intentional divergence in code comment + PLAN.
4. **Build + test.** `cargo build --workspace` then `cargo test --workspace`.
   Both must be green. Record the commands in the session log.
5. **Update the ledger.** `tools/inventory.sh` (or manual update if not yet
   scriptable) to mark the module ported; update PLAN.md parity matrix +
   evidence; append to §7 session log.
6. **Commit.** One logical unit per commit. Message references the
   module(s) and upstream commit. Never commit a red build.

## 2A. Mandatory documentation checkpoint

A Codex task is not complete until its repository state is documented, even if
it ends blocked or with a failing test:

1. Update `tools/port_status.json` for every module-status change, regenerate
   `tools/inventory.json` with `tools/inventory.sh`, and regenerate
   `CONVERSION-LEDGER.md` with `python3 tools/conversion_ledger.py`.
2. Update `PLAN.md` with phase status, parity evidence tier (`unit`, `mock`, or
   `live`), exact validation commands, issues, and the next dependency-safe
   unit.
3. Update `HANDOFF.md` with the current branch/worktree state, completed
   milestone, exact tests/checks, blockers, next action, and the generated
   strict percentages for all tracked and production modules.
4. If no module changed, explicitly record “no ledger change” plus the current
   ledger counts and validation result in `HANDOFF.md`; do not fabricate a
   `done` row merely to touch the file.
5. Before the final response or commit, run `cargo build --workspace`,
   `cargo test --workspace`, and `git diff --check` as applicable to the task,
   then verify `PLAN.md`, `HANDOFF.md`, `tools/inventory.json`, and
   `CONVERSION-LEDGER.md` agree.

The final Codex response must name the documentation files updated or verified,
quote the current ledger percentages, list exact validation commands, and state
the next unit or blocker. A session that stops before this gate must leave a
partial-work checkpoint in `HANDOFF.md`.

## 3. Porting conventions

- One crate per upstream package/area (see PLAN §3). Dependency direction is
  bottom-up; no upward imports.
- Public API mirrors upstream names (`get_hermes_home`, `now`, ...). Keep a
  `// PARITY:` comment on non-obvious functions naming the upstream lines.
- Errors: `thiserror`; functions that upstream make fail-open return
  `Option`/defaults rather than Result where that matches upstream.
- Caching semantics are part of the contract (module-level caches, reset
  hooks, `once_cell::sync::Lazy` for the Rust equivalents).
- Env-var / home resolution go through the same helpers as upstream — never
  duplicate resolution logic in a caller.
- Platform detection (`is_wsl`, `is_container`, ...) is cached for process
  lifetime exactly like upstream's module globals.

## 4. Fidelity rules — the hard line

- When upstream semantics are ambiguous, the upstream **test** is the oracle.
  When the test is missing, upstream **code** is the oracle; note the
  missing-test gap in the ledger.
- Do not "improve" behavior. 1:1 means 1:1, including fail-open fallbacks
  that look like mistakes. Divergence requires an explicit PLAN-note and
  user sign-off.
- Vendored upstream files go under `upstream/` (read-only). Golden oracles
  derive from upstream outputs, never invented.
