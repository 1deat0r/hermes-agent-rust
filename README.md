# Hermes Agent in Rust ☤

A **1:1 port** of [NousResearch/hermes-agent](https://github.com/NousResearch/hermes-agent)
to idiomatic Rust — same CLI surface, same on-disk formats, same wire
protocols, same observable behavior. Different implementation language.

> **Status: P0/P1 foundation.** Upstream: `b9aa928` (~843,792 production LOC,
> 1,103 modules). Live ledger: [`PLAN.md`](PLAN.md) ·
> [`tools/inventory.json`](tools/inventory.json) · [`AGENTS.md`](AGENTS.md).

## Why this project exists

The original Hermes Agent is the self-improving AI agent by Nous Research
(skills, agent-curated memory, FTS5 session search, cron, delegation, seven
terminal backends, multi-provider LLM support, a multi-platform messaging
gateway). This repo re-implements it in Rust so it can run as a single static
binary with no Python runtime — while remaining behaviorally identical.

## Fidelity contract

"1:1" is defined at the *observable contract* level (not line-for-line
transpilation):

- CLI surface — same `hermes` / `hermes-agent` / `hermes-acp` entry points
- On-disk formats — `config.yaml`, `~/.hermes` layout, state DB, session files
- Wire protocols — gateway platforms, MCP, ACP
- Provider/tool behavior — same message shapes, tool names and outputs
- Fail-open semantics — same fallbacks, same env precedence, same caching

Every ported behavior is pinned by parity tests whose cases derive from
upstream tests (golden fixtures under `upstream/`). Guessing at upstream
behavior is a defect, not a shortcut.

## Workspace layout

```
crates/
  hermes-constants/   # hermes_constants.py  — foundational subset ✅
  hermes-time/        # hermes_time.py       — full port ✅
  ...                 # hermes-utils, logging, state, agent, cli, gateway…
                      # (see PLAN.md §3 for the full crate map)
tools/                # inventory ledger + parity helpers
upstream/             # vendored golden fixtures (read-only)
```

## Governance

The standing process rules live in [`PLAN.md`](PLAN.md) §0 and
[`AGENTS.md`](AGENTS.md). In short: every phase ends with the plan updated,
evidence carries a tier (`unit` / `mock` / `live`), and every commit builds
green.

## Building

```bash
cargo build --workspace
cargo test --workspace    # 74 tests, including upstream parity oracles
```

## Upstream reference

Local pinned clone: `/home/mustbearn/Projects/Research/hermes-agent-repo`
(remote: `https://github.com/NousResearch/hermes-agent`, commit `b9aa928`).

## License

MIT — see [LICENSE](LICENSE). Original Hermes Agent © Nous Research.
