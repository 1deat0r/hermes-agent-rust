# Hermes Agent in Rust ☤

An in-progress **1:1 Rust port of [NousResearch/hermes-agent](https://github.com/NousResearch/hermes-agent) @ `5d59366` — including the Hermes Agent Desktop Linux app**. The target is the same CLI surface, on-disk formats, wire protocols, provider/tool behavior, observable semantics, and Linux desktop shell (AppImage/deb/rpm) in an idiomatic Rust implementation. The desktop strategy is a `hermes-desktop` crate via Tauri (upstream's own bootstrap-installer precedent); renderer strategy per-module.

## Current status

The live conversion ledger currently reports:

- **All tracked modules:** **1.83%** — `163 / 8,895` done, `39` partial,
  `8,693` missing.
- **Production modules:** **4.68%** — `163 / 3,481` done, `39` partial,
  `3,279` missing.

Only `done` rows receive credit; partial rows remain zero-credit until their
parity seams are closed. Regenerate the inventory and ledger with:

```bash
HERMES_UPSTREAM=/path/to/hermes-agent-repo tools/inventory.sh
python3 tools/conversion_ledger.py
```

P0 infrastructure/governance is complete. Foundation and provider-support work
is landed in several crates, while the agent core, remaining tools, CLI,
gateway, platform plugins, cron, TUI, ACP, the new P6 desktop surface
(2,470 `apps/desktop` + 55 shared/installer TS modules), and the full
parity-oracle surface remain in progress. Retargeted 2026-09-16 from
`b9aa928` to `5d59366`: 162 drifted Python rows were honestly demoted to
`partial` for re-certification (see PLAN.md), which is why the strict
percentage reset. The percentage is intentionally strict and is not a claim
that the runtime is feature-complete.

The first `hermes-agent::auxiliary_client` routing/wire-predicate slice is
partial; its client construction, credential, async transport, cancellation,
and fallback-chain sections remain to be ported.

## Why this project exists

Hermes Agent is a self-improving AI agent with skills, curated memory, FTS5
session search, cron scheduling, delegation, multiple terminal backends,
multi-provider model support, and a multi-platform gateway. This repository
reimplements those observable contracts in Rust, with Python behavior and
upstream tests serving as the oracle.

## Fidelity contract

The port is evaluated at the observable-contract level:

- CLI entry points: `hermes`, `hermes-agent`, and `hermes-acp`
- On-disk formats: configuration, `~/.hermes`, state DB, and session files
- Wire protocols: gateway platforms, MCP, and ACP
- Provider and tool names, inputs, outputs, and streaming behavior
- Environment precedence, caching, lifecycle, and fail-open semantics

Intentional divergences must be documented and signed off. A compiling crate
alone does not count as parity.

## Workspace layout

```text
crates/
  hermes-constants/   platform, paths, values, and reasoning constants
  hermes-time/        time and timezone behavior
  hermes-utils/       shared utility behavior
  hermes-logging/     logging and redaction
  hermes-state/       SQLite state, sessions, search, routing, and portability
  hermes-toolsets/    tool schemas and distributions
  hermes-providers/   provider registry and bundled profiles
  hermes-agent/       agent loop and runtime
  hermes-tools/       tool implementations and safety helpers
  hermes-batch/       batch and trajectory helpers
  hermes-cli/         `hermes` command-line surface
  hermes-gateway/     messaging gateway
  hermes-platforms/   platform plugins
  hermes-cron/        scheduling
  hermes-tui/         terminal UI
  hermes-acp/         Agent Client Protocol

tools/                inventory and parity helpers
upstream/             pinned golden fixtures, read-only
```

## Project documents

- [`PLAN.md`](PLAN.md) — governance, phases, parity matrix, evidence, and next
  dependency-safe unit.
- [`CONVERSION-LEDGER.md`](CONVERSION-LEDGER.md) — generated strict module and
  oracle/test ledger.
- [`HANDOFF.md`](HANDOFF.md) — current checkpoint, exact validation, blockers,
  next action, and completion percentages.
- [`AGENTS.md`](AGENTS.md) — mandatory Codex documentation and commit/push gate.

Each Codex task must update or verify the ledger, plan, and handoff before it
reports completion. One logical unit is committed and pushed at a time; remote
synchronization must be verified rather than assumed.

## Commit hooks and GitHub metadata

Install the tracked hooks once per checkout:

```bash
tools/install_hooks.sh
```

The `pre-commit` hook refreshes `tools/inventory.json`,
`CONVERSION-LEDGER.md`, and the README status snapshot whenever source,
parity, inventory, or hook files are staged. It also requires staged updates to
`PLAN.md` and `HANDOFF.md` for those changes. The `.github/repository-description.txt`
file is the reviewed source for the GitHub repository description.

The `post-commit` hook synchronizes that description through the GitHub API and
verifies that the remote `README.md` matches the committed file. README changes
are intentionally published by the normal local/remote commit mirror or push,
so the hook does not manufacture a second README commit. Set
`HERMES_GITHUB_TOKEN`, `GH_TOKEN`, or `GITHUB_TOKEN` for metadata access; use
`tools/install_hooks.sh --strict` when missing credentials or a remote mismatch
should be treated as an error. To explicitly update a remote README through the
Contents API, run `HERMES_GITHUB_README_MODE=sync python3
tools/sync_github_metadata.py` outside the exact-mirror commit path.

## Build and test

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

The exact commands and evidence tier for each checkpoint belong in `PLAN.md` and
`HANDOFF.md`. Keep upstream-derived parity tests green and do not silently
convert partial or missing inventory rows into completion.

## Upstream reference

Target repository: [NousResearch/hermes-agent](https://github.com/NousResearch/hermes-agent)

Pinned upstream commit: `5d59366` (`5d59366010640c1d6b8f170d8a4ee109db2bbdef`, latest `origin/main` as of 2026-09-16 NZ; pinned worktree `.upstream-pin/5d59366`).

## License

MIT — see [LICENSE](LICENSE). Original Hermes Agent © Nous Research.
