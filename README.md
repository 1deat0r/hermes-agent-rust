# Hermes Agent in Rust ☤

An in-progress **1:1 Rust port of [NousResearch/hermes-agent](https://github.com/NousResearch/hermes-agent)**. The target is the same CLI surface, on-disk formats, wire protocols, provider/tool behavior, and observable semantics in an idiomatic Rust implementation.

## Current status

The live conversion ledger currently reports:

- **All tracked modules:** **2.34%** — `91 / 3,882` done, `14` partial,
  `3,777` missing.
- **Production modules:** **8.25%** — `91 / 1,103` done, `14` partial,
  `998` missing.

Only `done` rows receive credit; partial rows remain zero-credit until their
parity seams are closed. Regenerate the inventory and ledger with:

```bash
HERMES_UPSTREAM=/path/to/hermes-agent-repo tools/inventory.sh
python3 tools/conversion_ledger.py
```

P0 infrastructure/governance is complete. Foundation and provider-support work
is landed in several crates, while the agent core, remaining tools, CLI,
gateway, platform plugins, cron, TUI, ACP, and the full parity-oracle surface
remain in progress. The percentage is intentionally strict and is not a claim
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

Pinned upstream commit: `b9aa928`.

## License

MIT — see [LICENSE](LICENSE). Original Hermes Agent © Nous Research.
