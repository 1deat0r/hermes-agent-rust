#!/usr/bin/env python3
"""Regenerate tools/inventory.json from the upstream Hermes Agent repo.

Baseline (module path, lines, imports, package) is regenerated from upstream.
Port status is preserved from tools/port_status.json (agent-maintained) and
merged; modules absent from the overlay default to {"status": "missing"}.

Usage:
    python3 tools/inventory.py /path/to/hermes-agent  [--out tools/inventory.json]
"""
import argparse, json, os, re, sys
from collections import defaultdict

EXCLUDE_DIRS = {'.git', 'node_modules', '.venv', 'venv', 'web', 'website'}

# TypeScript/JavaScript sources for the Hermes Agent Desktop Linux app
# (apps/desktop Electron main + React renderer, apps/shared, Tauri
# bootstrap-installer) are tracked as parity modules with a `ts:` prefix so
# they stay disjoint from Python module names. Line counts are raw lines.
TS_EXTS = {'.ts', '.tsx', '.mts', '.cts', '.js', '.jsx', '.mjs', '.cjs'}
TS_SCOPE_DIRS = {'apps', 'tests-js'}
TS_TEST_RE = re.compile(r'(^|\.)(test|spec)(\.[a-z]+)?$')
# The parity target is the Hermes Agent Desktop app itself
# (apps/desktop Electron main + React renderer, apps/shared contract lib,
# apps/bootstrap-installer Tauri setup UI) plus root desktop-gating JS tests.
# Sibling web/, ui-tui/, and per-package e2e/diag/perf harnesses are not
# desktop parity surface. Vendored shims and fixture helpers are excluded.
TS_SKIP_PARTS = {
    '__fixtures__', 'e2e', 'pr-assets', 'public',
    'diag-', 'perf', 'repro', 'rehearsal', 'click-session',
    'connector-rehearsal', 'find-in-page-native-fixture',
}
TS_SKIP_SUFFIX = (
    '.d.ts', '.d.mts', '.d.cts',
    '.e2e.ts', '.e2e.mts', '.e2e.mjs', '.e2e.js',
    '.test.mjs', '.test.cjs',
    '.min.js', '.bundle.js',
)
# Test-only support that is not desktop product surface.
TS_ORACLE_HINTS = ('test-harness', 'test-utils', 'test-desktop')

def build_inventory(root):
    global test_mods
    mods = {}
    test_mods = set()
    imports = defaultdict(set)
    ts_mods = {}
    for dirpath, dirnames, filenames in os.walk(root):
        rel = os.path.relpath(dirpath, root)
        dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS and not d.startswith('.')]
        for fn in filenames:
            full = os.path.join(dirpath, fn)
            if rel == '.':
                mod = fn[:-3]
            else:
                parts = rel.split(os.sep) + [fn[:-3]]
                mod = '.'.join(parts)
            if fn.endswith('.py'):
                n = sum(1 for _ in open(full, encoding='utf-8', errors='replace'))
                mods[mod] = n
            else:
                _root = rel.split(os.sep)[0] if rel != '.' else ''
                _, ext = os.path.splitext(fn)
                if _root in TS_SCOPE_DIRS and ext in TS_EXTS:
                    if any(part in TS_SKIP_PARTS or part.startswith('diag-')
                           for part in rel.split(os.sep)):
                        continue
                    if fn.endswith(TS_SKIP_SUFFIX):
                        continue
                    stem = fn[:-len(ext)] if ext else fn
                    tmod = 'ts:' + '.'.join(rel.split(os.sep) + [stem]) if rel != '.' else 'ts:' + stem
                    n = sum(1 for _ in open(full, encoding='utf-8', errors='replace'))
                    ts_mods[tmod] = n
                    if TS_TEST_RE.search(stem) or any(
                        part in ('__tests__', 'e2e') for part in rel.split(os.sep)
                    ) or any(h in stem for h in TS_ORACLE_HINTS):
                        test_mods.add(tmod)
                continue
            if any(part == 'tests' or part.startswith('test_') for part in rel.split(os.sep) + [fn]):
                test_mods.add(mod)
            try:
                txt = open(full, encoding='utf-8', errors='replace').read()
            except Exception:
                continue
            for m in re.finditer(r'^\s*(?:from\s+([\w.]+)\s+import|import\s+([\w.]+))', txt, re.M):
                top = (m.group(1) or m.group(2)).split('.')[0]
                if top in mods and top != mod:
                    imports[mod].add(m.group(1) or m.group(2))
    return mods, imports, ts_mods

def main():
    # module -> is-test flag captured during build
    global test_mods
    test_mods = set()
    ap = argparse.ArgumentParser()
    ap.add_argument('upstream')
    ap.add_argument('--out', default='tools/inventory.json')
    args = ap.parse_args()
    root = os.path.abspath(args.upstream)

    mods, imports, ts_mods = build_inventory(root)
    mods = dict(mods)
    mods.update(ts_mods)
    # package aggregation
    TOP_PKGS = {'agent','gateway','hermes_cli','plugins','tools','cron','acp_adapter','tui_gateway'}
    def pkg_of(m):
        if m.startswith('ts:apps.desktop'):
            return 'desktop'
        if m.startswith('ts:apps.'):
            return 'ts:apps.shared+installer'
        if m.startswith('ts:'):
            return 'ts:web+tui+tests'
        parts = m.split('.')
        if len(parts) == 1:
            return m
        return parts[0] if parts[0] in TOP_PKGS else '.'.join(parts[:2])

    pkgs = defaultdict(lambda: {'lines': 0, 'modules': 0, 'status': {'missing': 0}})
    inv = {}
    overlay_path = os.path.join(os.path.dirname(args.out), 'port_status.json')
    overlay = {}
    if os.path.exists(overlay_path):
        overlay = json.load(open(overlay_path))
    for m, n in sorted(mods.items(), key=lambda kv: -kv[1]):
        st = overlay.get(m, {'status': 'missing'})
        inv[m] = {'lines': n, 'imports': sorted(imports.get(m, [])), 'port_status': st['status'],
                  'is_test': m in test_mods}
        p = pkg_of(m)
        pkgs[p]['lines'] += n
        pkgs[p]['modules'] += 1
        pkgs[p]['status'][st['status']] = pkgs[p]['status'].get(st['status'], 0) + 1

    prod = {m: v for m, v in inv.items() if not v['is_test']}
    summary = {
        'modules': len(inv),
        'total_lines': sum(v['lines'] for v in inv.values()),
        'production_modules': len(prod),
        'production_lines': sum(v['lines'] for v in prod.values()),
        'status_counts': {},
        'prod_status_counts': {},
        'packages': dict(pkgs),
    }
    for s in ('done', 'partial', 'missing'):
        summary['status_counts'][s] = sum(1 for v in inv.values() if v['port_status'] == s)
        summary['prod_status_counts'][s] = sum(1 for v in prod.values() if v['port_status'] == s)

    out = {'generated_from': root, 'generated_at': __import__('time').strftime('%Y-%m-%dT%H:%M:%S'), 'summary': summary, 'modules': inv}
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, 'w') as f:
        json.dump(out, f, indent=1)
    print(f"modules={summary['modules']} total_lines={summary['total_lines']} "
          f"prod={summary['production_modules']}/{summary['production_lines']} "
          f"status={summary['status_counts']}")

if __name__ == '__main__':
    main()
