#!/usr/bin/env python3
"""Portable smoke check; the installed Omarchy validator is authoritative."""
import json
from pathlib import Path
root = Path(__file__).resolve().parent.parent
m = json.loads((root / 'manifest.json').read_text())
assert m['schemaVersion'] == 1
assert m['id'] and not m['id'].startswith('omarchy.')
assert m['kinds'] == ['service']
assert m['entryPoints'].get('service')
assert 'barWidget' not in m
for name in m['entryPoints'].values():
    p = Path(name)
    assert not p.is_absolute() and '..' not in name
    assert (root / p).is_file() and not (root / p).is_symlink()
assert m['version'] == '0.1.0'
print('Manifest smoke check passed')
