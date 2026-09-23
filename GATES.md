# Gates: hermes_logging re-cert @ 5d59366

OWNS: crates/hermes-logging/**, crates/hermes-constants/src/home.rs, GATES.md, PLAN.md, HANDOFF.md, tools/port_status.json, tools/inventory.json, CONVERSION-LEDGER.md

Scope: Re-certify hermes_logging against the 5d59366 oracle (profile/second-home routing, hermes_home record stamp, EIO/unavailable-stream recovery, external-rotation + dedup fixes, verbose idempotency) and move the ledger row partial → done.

- [x] G1: workspace build compiles the new routing/EIO surface
  CHECK: cargo build --workspace 2>&1 | tail -5
  EXPECT: Finished
  EVIDENCE: automatic-evidence=v1; definition-sha256=545edaf83bd2820288db17ffd238555e9f9a48686cf449a5bab1a13839ab19ec; exit=0; EXPECT=matched; output-sha256=c3e25e05870567f1463700fa950acb48b09c77d2dccce85d1b2e0a2ec3ff28d3; output-bytes=268; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust; path=1ff3da1d9906/16 entries

- [x] G2: full workspace test suite is green serially (0 failed)
  CHECK: cargo test --workspace -- --test-threads=1 2>&1 | tail -40
  EXPECT: 0 failed
  EVIDENCE: automatic-evidence=v1; definition-sha256=a25ff46a65a7e207d474f2216bc870a6f565e2016537e9bf366267c77c6c6c75; exit=0; EXPECT=matched; output-sha256=ba49544233d4e3d2c0f9184fea970662ccee8fb3f30ef47af74ae88431f0092c; output-bytes=1291; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust; path=1ff3da1d9906/16 entries

- [x] G3: formatting and whitespace checks pass
  CHECK: cargo fmt --all --check && git diff --check && echo CHECKS_CLEAN
  EXPECT: CHECKS_CLEAN
  EVIDENCE: automatic-evidence=v1; definition-sha256=5d3b3e51113d5d45026430e96eea42ec3746d1030faf1857f5b89a79df473917; exit=0; EXPECT=matched; output-sha256=ea0f5d39e575fbabb589d4bbd8b702f0656f0771d5a53bcfe390f02c69f5b023; output-bytes=13; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust; path=1ff3da1d9906/16 entries

- [x] G4: upstream oracle test file still green at the pin (python3 system env)
  CHECK: python3 -m pytest tests/test_hermes_logging.py tests/test_log_isolation.py -q 2>&1 | tail -3
  EXPECT: passed
  CWD: .upstream-pin/5d59366
  EVIDENCE: automatic-evidence=v1; definition-sha256=43c017b549baa683698c3130ccc20bf1005e27010499035b1a0c7e18e6a20eac; exit=0; EXPECT=matched; output-sha256=aed32a2ed5bbdc501c9f3aabe987fe9e4116d93dd552e757926e24b2f45fdce5; output-bytes=110; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust/.upstream-pin/5d59366; path=1ff3da1d9906/16 entries

- [x] G5: ledger regenerated and hermes_logging row is done
  CHECK: bash tools/inventory.sh && python3 tools/conversion_ledger.py && python3 -c "import json;d=json.load(open('tools/port_status.json'));r=d['hermes_logging'];assert r['status']=='done',r;print('hermes_logging',r['status'])"
  EXPECT: hermes_logging done
  EVIDENCE: automatic-evidence=v1; definition-sha256=3739cb33a2cecef74bfcb03e58482a62a73c7eae7590bc1cc4a089d1bd1d90ef; exit=0; EXPECT=matched; output-sha256=96c699fc80f5d2f9c83136d26e28a8366a57e8bf3548cebf3c140d52bb930133; output-bytes=254; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust; path=1ff3da1d9906/16 entries

- [x] G6: documentation checkpoint agrees across PLAN/HANDOFF/ledger
  CHECK: grep -c "hermes_logging" PLAN.md HANDOFF.md && python3 -c "import json;s=json.load(open('tools/inventory.json'))['summary'];print(s['status_counts'],s['prod_status_counts'])"
  EXPECT: {
  EVIDENCE: automatic-evidence=v1; definition-sha256=4e41ea97ad92d94072dedcddf19104daa00e1326d5e8f7f6ac3cde83ad4d4e86; exit=0; EXPECT=matched; output-sha256=21c6395b441549313c87d77efe155c496e5150fac550d7416f43ef1cabe2b5fc; output-bytes=115; shell=/bin/sh; cwd=/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust; path=1ff3da1d9906/16 entries
