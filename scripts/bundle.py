#!/usr/bin/env python3
"""Maintain the executable and notices distributed with the Omarchy plugin."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent
BUNDLE = ROOT / 'bin'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_files():
    # Rust sources and embedded data, including the main program's license text.
    paths = {ROOT / name for name in ('Cargo.toml', 'Cargo.lock', 'LICENSE')}
    paths.add(ROOT / 'assets/sokobanchy.png')
    paths.update((ROOT / 'src').glob('*.rs'))
    paths.update(p for p in (ROOT / 'assets').rglob('*')
                 if p.is_file() and (p.suffix in ('.xsb', '.txt') or p.name == 'OMARCHY-LICENSE'))
    return {str(p.relative_to(ROOT)): digest(p) for p in sorted(paths)}


def prepare(binary, licenses):
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        sys.exit('Build this bundle on Omarchy x86_64.')
    BUNDLE.mkdir(exist_ok=True)
    # Atomic replacement also works while the previous executable is running.
    temporary = BUNDLE / 'sokobanchy.new'
    shutil.copyfile(binary, temporary)
    temporary.chmod(0o755)
    temporary.replace(BUNDLE / 'sokobanchy')
    notices = BUNDLE / 'THIRD_PARTY'
    if notices.exists():
        shutil.rmtree(notices)
    notices.mkdir()
    for path in sorted(licenses.iterdir()):
        # Those notices already ship under assets/ in the plugin repository.
        if path.name in ('collections', 'OMARCHY-LICENSE'):
            continue
        if path.is_dir():
            shutil.copytree(path, notices / path.name)
        else:
            shutil.copyfile(path, notices / path.name)
    metadata = {
        'version': tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version'],
        'platform': 'omarchy-linux-x86_64',
        'compiler': subprocess.check_output(['rustc', '--version'], text=True).strip(),
        'source_sha256': source_files(),
        'files_sha256': {str(p.relative_to(BUNDLE)): digest(p)
                         for p in sorted(BUNDLE.rglob('*'))
                         if p.is_file() and p.name != 'BUILD.json'},
    }
    (BUNDLE / 'BUILD.json').write_text(json.dumps(metadata, indent=2) + '\n')
    check()


def check():
    metadata = json.loads((BUNDLE / 'BUILD.json').read_text())
    assert metadata['source_sha256'] == source_files(), 'Source changed: rebuild with scripts/package.sh'
    assert metadata['version'] == json.loads((ROOT / 'manifest.json').read_text())['version']
    assert metadata['platform'] == 'omarchy-linux-x86_64'
    files = {str(p.relative_to(BUNDLE)): digest(p) for p in sorted(BUNDLE.rglob('*'))
             if p.is_file() and p.name != 'BUILD.json'}
    assert metadata['files_sha256'] == files, 'Bundle files do not match BUILD.json'
    binary = BUNDLE / 'sokobanchy'
    header = binary.read_bytes()[:20]
    assert header[:6] == b'\x7fELF\x02\x01' and header[18:20] == b'\x3e\x00', 'Expected x86_64 ELF'
    assert os.access(binary, os.X_OK), 'Bundled game must be executable'
    assert not any(p.is_symlink() for p in BUNDLE.rglob('*')), 'Bundle must not contain symlinks'
    assert (BUNDLE / 'THIRD_PARTY/INDEX.txt').is_file(), 'Dependency notices are required'
    print('Bundled Omarchy executable, source identity, and notices verified')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('prepare', 'check'))
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--licenses', type=Path)
    args = parser.parse_args()
    if args.action == 'prepare':
        if not args.binary or not args.licenses:
            parser.error('prepare requires --binary and --licenses')
        prepare(args.binary, args.licenses)
    else:
        check()
