# Gates: hermes_state_common re-cert @ 5d59366

OWNS: crates/hermes-state/src/common.rs, crates/hermes-state/src/common_constants.rs, crates/hermes-state/src/fts_lock.rs, crates/hermes-state/src/lib.rs, crates/hermes-state/src/locks.rs, crates/hermes-state/src/search.rs, crates/hermes-state/src/compression_prefix.rs, crates/hermes-state/tests/parity_state_common.rs, crates/hermes-state/tests/parity_state_compression_locks.rs, tools/gen_state_common_constants.py, tools/golden_state_common.py, upstream/golden_state_common.json, GATES.md, PLAN.md, HANDOFF.md, tools/port_status.json, tools/inventory.json, CONVERSION-LEDGER.md

Scope: Re-certify hermes_state_common (614 → 1,218 LOC) against the 5d59366 oracle: regen SQL constants (SCHEMA_VERSION 30 / FTS_STORAGE_VERSION 2), compaction-aware preview suite, end-reason taxonomy + is_automatic_end_reason wired into publish_compression_child heal, reset-aware child SQL, FTS tool high-water + trigram session predicate, fts_rebuild_admission flock authority, and move the ledger row partial → done.

- [x] G1: workspace build compiles the new common surface
  CHECK: cargo build --workspace 2>&1 | tail -5
  EXPECT: Finished
  EVIDENCE: automatic-evidence=v1; definition-sha256=545edaf83bd2820288db17ffd238555e9f9a48686cf449a5bab1a13839ab19ec; exit=0; EXPECT=matched; output-sha256=c65a5eb59830e87424e03144339d8e85f51f102587598e830121c1f652169c3b; output-bytes=328; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust; path=5a495bace11d/16 entries

- [x] G2: full workspace test suite is green serially (0 failed)
  CHECK: cargo test --workspace -- --test-threads=1 2>&1 | tail -40
  EXPECT: 0 failed
  EVIDENCE: automatic-evidence=v1; definition-sha256=a25ff46a65a7e207d474f2216bc870a6f565e2016537e9bf366267c77c6c6c75; exit=0; EXPECT=matched; output-sha256=13d6321b0928c060cb13db9d4d293e49b944e73433aa1b8df745a46528720f8b; output-bytes=1909; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust; path=5a495bace11d/16 entries

- [x] G3: formatting and whitespace checks pass
  CHECK: cargo fmt --all --check && git diff --check && echo CHECKS_CLEAN
  EXPECT: CHECKS_CLEAN
  EVIDENCE: automatic-evidence=v1; definition-sha256=5d3b3e51113d5d45026430e96eea42ec3746d1030faf1857f5b89a79df473917; exit=0; EXPECT=matched; output-sha256=ea0f5d39e575fbabb589d4bbd8b702f0656f0771d5a53bcfe390f02c69f5b023; output-bytes=13; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust; path=5a495bace11d/16 entries

- [x] G4: upstream common-module oracles green at the pin (system python3)
  CHECK: python3 -m pytest tests/hermes_state/test_automatic_ended_stamp.py tests/hermes_state/test_state_db_lock_fail_closed.py tests/hermes_state/test_fts_rebuild_admission.py tests/hermes_state/test_fts_tool_write_bounds.py tests/hermes_state/test_fts_trigram_subagent_exclusion.py tests/hermes_state/test_fts_trigram_cron_exclusion.py -q 2>&1 | tail -3
  EXPECT: passed
  CWD: .upstream-pin/5d59366
  EVIDENCE: automatic-evidence=v1; definition-sha256=2a3dbdf20de91daf535589874731ae460e7c2d5e92f3cc374c5b501794671783; exit=0; EXPECT=matched; output-sha256=469acd4181fd2186fe0d92397804d4eac1f5aa74a403479895781764087cb3c0; output-bytes=244; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust/.upstream-pin/5d59366; path=5a495bace11d/16 entries

- [x] G5: ledger regenerated and hermes_state_common row is done
  CHECK: bash tools/inventory.sh && python3 tools/conversion_ledger.py && python3 -c "import json;d=json.load(open('tools/port_status.json'));r=d['hermes_state_common'];assert r['status']=='done',r;print('hermes_state_common',r['status'])"
  EXPECT: hermes_state_common done
  EVIDENCE: automatic-evidence=v1; definition-sha256=978d7d5205c22d75d90549e6406e2da93518ac884caadf7a71a92f1e1af1b9d3; exit=0; EXPECT=matched; output-sha256=39c89a58d46e54c25c5a33a1f76ff15d297e21dba2076bda696c6a0208b3138e; output-bytes=259; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust; path=5a495bace11d/16 entries

- [x] G6: documentation checkpoint agrees across PLAN/HANDOFF/ledger
  CHECK: grep -c "hermes_state_common" PLAN.md HANDOFF.md && python3 -c "import json;s=json.load(open('tools/inventory.json'))['summary'];print(s['status_counts'],s['prod_status_counts'])"
  EXPECT: {
  EVIDENCE: automatic-evidence=v1; definition-sha256=5f9618ba3ac1ecb094f73803fd05b190ece9ecfe73dd88762fbe560861a992ac; exit=0; EXPECT=matched; output-sha256=ace45458a66296393b22124631385ecbb3b5762a385bcc15243866aea2f04c05; output-bytes=115; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust; path=5a495bace11d/16 entries
