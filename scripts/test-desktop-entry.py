#!/usr/bin/env python3
"""Exercise desktop registration, real Exec parsing, reloads, and owned cleanup."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

from gi.repository import Gio, GioUnix

ROOT = Path(__file__).resolve().parent.parent
helper = (ROOT / 'scripts/desktop-entry.py').read_text()

with tempfile.TemporaryDirectory(prefix='sokobanchy-desktop-') as directory:
    temporary = Path(directory)
    plugin = temporary / 'plugin with spaces $cash `ticks` "quotes" %field \\slash'
    for name in ('bin', 'assets', 'packaging'):
        (plugin / name).mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / 'packaging/sokobanchy.desktop', plugin / 'packaging/sokobanchy.desktop')
    shutil.copy2(ROOT / 'assets/sokobanchy.svg', plugin / 'assets/sokobanchy.svg')
    binary = plugin / 'bin/sokobanchy'
    binary.write_text('#!/usr/bin/python3\nimport os, pathlib, sys, json\n'
                      'pathlib.Path(os.environ["SOKOBANCHY_LAUNCH_RESULT"]).write_text(json.dumps(sys.argv))\n')
    binary.chmod(0o755)
    result = temporary / 'launched.json'
    data = temporary / 'data with spaces'
    env = dict(os.environ, XDG_DATA_HOME=str(data))
    entry = data / 'applications/sokobanchy.desktop'

    def run(action, owner, *extra):
        subprocess.run([sys.executable, '-c', helper, action, str(plugin), owner, *extra],
                       env=env, check=True)

    run('register', 'first')
    subprocess.run(['desktop-file-validate', str(entry)], check=True)
    app = GioUnix.DesktopAppInfo.new_from_filename(str(entry))
    assert app and app.get_name() == 'Sokobanchy'
    assert app.get_icon().to_string() == str(plugin / 'assets/sokobanchy.svg')
    context = Gio.AppLaunchContext()
    context.setenv('SOKOBANCHY_LAUNCH_RESULT', str(result))
    assert app.launch([], context)
    deadline = time.monotonic() + 5
    while not result.exists() and time.monotonic() < deadline:
        time.sleep(0.02)
    assert json.loads(result.read_text()) == [str(binary)], 'Desktop Exec did not preserve the exact path'

    run('register', 'second')
    run('remove', 'first')
    assert 'X-Sokobanchy-Instance=second' in entry.read_text(), 'Old cleanup removed a new entry'
    run('remove', 'second')
    assert not entry.exists()

    custom = '[Desktop Entry]\nType=Application\nName=My Sokobanchy\nExec=custom-game\n'
    entry.write_text(custom)
    run('register', 'third')
    run('remove', 'third')
    assert entry.read_text() == custom, 'Overwrote a standalone or user entry'
    entry.unlink()
    other = temporary / 'user.desktop'
    other.write_text(custom)
    entry.symlink_to(other)
    run('register', 'fourth')
    run('remove', 'fourth')
    assert entry.is_symlink() and other.read_text() == custom
    entry.unlink()

    run('register', 'removed-checkout')
    shutil.rmtree(plugin)
    try:
        removed = GioUnix.DesktopAppInfo.new_from_filename(str(entry))
    except TypeError:
        removed = None
    assert removed is None, 'TryExec should hide a removed game'
    run('remove', 'removed-checkout')
    assert not entry.exists(), 'Cleanup depends on a checkout that may already be removed'

print('Desktop entry passed: launch, special paths, icon, reload ownership, user entries, removed checkout')
