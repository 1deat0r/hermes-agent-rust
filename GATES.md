# Gates: complete hermes-agent to hermes-agent-rust conversion

OWNS: crates/**, tools/**, scripts/**, examples/**, upstream/**, PLAN.md, HANDOFF.md, README.md, CONVERSION-LEDGER.md

Scope: finish and verify the full pinned `hermes-agent` @ `5d59366` conversion (Python agent/CLI/gateway surface **plus the Hermes Agent Desktop Linux app**: `apps/desktop` Electron 40 main + React renderer with Linux AppImage/deb/rpm packaging, `apps/shared`, Tauri `apps/bootstrap-installer`, root `tests-js` desktop gates), including implementation, parity evidence, integration, documentation, and exact local/GitHub publication.

## Depth tree

The conversion is decomposed into these dependency-ordered branches:

1. Foundation: constants, time, utilities, logging, and state.
2. Agent core: toolsets, model tools, `run_agent`, `agent/`, `tools/`, and batch.
3. CLI: `cli.py`, `hermes_cli/`, configuration, auth, and entry points.
4. Integrations: providers, plugins/platforms, gateway, and cron.
5. Surfaces: TUI, ACP, scripts, bundled skills, and remaining top-level modules.
6. Desktop (P6): `hermes-desktop` crate via Tauri — Electron main-process TS modules ported to Rust backend commands, `apps/shared` contract lib mirrored, React renderer strategy decided per-module, Linux AppImage/deb/rpm packaging parity.
7. Root integration: full-workspace tests, live/mock contract checks, docs/ledger closure, and local/GitHub mirror verification.

The active leaves are Agent core → config discovery and Providers → Z.AI
endpoint chooser. Both current units are transport-neutral explicit-input
boundaries; their next dependency-safe work is full merged config loading and
concrete Z.AI HTTP/cache integration. Later leaves must not be marked verified
while a lower-layer contract remains partial.

- [ ] G1: every tracked upstream module is marked done in the generated inventory
  CHECK: /usr/bin/python3 -c "import json; s=json.load(open('tools/inventory.json', encoding='utf-8'))['summary']; assert s['modules'] == 8895 and s['status_counts'] == {'done': 8895}, s; assert s['production_modules'] == 3481 and s['prod_status_counts'] == {'done': 3481}, s; print('inventory closure passed')"
  EXPECT: inventory closure passed
  EVIDENCE: pending (retarget 2026-09-16: 5 done / 164 partial / 8726 missing tracked; 5 / 164 / 3312 production)

- [ ] G2: the complete workspace builds and all active Rust tests pass serially
  CHECK: cargo test --workspace -- --test-threads=1
  EXPECT: test result: ok
  EVIDENCE: pending

- [ ] G3: the final workspace is formatted and documentation hooks remain valid
  CHECK: bash -n .githooks/pre-commit .githooks/post-commit tools/install_hooks.sh && PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -c "from pathlib import Path; [compile(p.read_text(encoding='utf-8'), str(p), 'exec') for p in (Path('tools/pre_commit_docs.py'), Path('tools/refresh_docs.py'), Path('tools/sync_github_metadata.py')]; print('documentation hooks passed')" && cargo fmt --all -- --check
  EXPECT: documentation hooks passed
  EVIDENCE: pending

- [ ] G4: the generated ledger and README are refreshed from the pinned upstream checkout and contain no stale completion snapshot
  CHECK: UP=/tmp/hermes-upstream-5d59366; git -C /run/media/its1deat0r/Projects/Research/hermes-agent-repo worktree list 2>/dev/null | grep -q "$UP" || git -C /run/media/its1deat0r/Projects/Research/hermes-agent-repo worktree add --detach "$UP" 5d59366; test "$(git -C "$UP" rev-parse HEAD)" = 5d59366010640c1d6b8f170d8a4ee109db2bbdef && HERMES_UPSTREAM="$UP" /usr/bin/python3 tools/refresh_docs.py --upstream "$UP" && git diff --exit-code -- tools/inventory.json CONVERSION-LEDGER.md README.md && echo documentation snapshot passed
  EXPECT: documentation snapshot passed
  EVIDENCE: pending

- [ ] G5: every implemented module has source-derived parity tests or an explicit reviewed live/mock evidence record
  EVIDENCE: pending

- [ ] G6: the final local `main` commit and GitHub `main` commit have identical recursive trees, modes, and blob SHAs, and every logical commit was mirrored immediately
  EVIDENCE: pending

- [ ] G7: end-to-end CLI, provider, tool, state, gateway, TUI, ACP, and live-platform contracts have been reviewed against the pinned upstream behavior
  EVIDENCE: pending
