#!/usr/bin/env python3
"""Exercise the actual bundled launcher without compilers or download tools."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='sokobanchy-launch-') as directory:
    temporary = Path(directory)
    plugin = temporary / 'plugin with spaces'
    (plugin / 'scripts').mkdir(parents=True)
    (plugin / 'bin').mkdir()
    shutil.copy2(root / 'scripts/launch.sh', plugin / 'scripts/launch.sh')
    shutil.copy2(root / 'bin/sokobanchy', plugin / 'bin/sokobanchy')
    commands = temporary / 'commands'
    commands.mkdir()
    for name in ('bash', 'dirname', 'uname'):
        (commands / name).symlink_to(shutil.which(name))
    env = dict(os.environ, PATH=str(commands), XDG_CACHE_HOME=str(temporary / 'cache'),
               XDG_STATE_HOME=str(temporary / 'state'))
    launcher = str(plugin / 'scripts/launch.sh')
    for flag, expected in [('--version', '0.1.0'), ('--help', 'Usage: sokobanchy')]:
        result = subprocess.run([launcher, flag], env=env, text=True, capture_output=True)
        assert result.returncode == 0, result.stderr
        assert expected in result.stdout, result.stdout
    assert not (temporary / 'cache').exists(), 'Launcher must not build or download anything'
    assert not (temporary / 'state').exists(), 'CLI smoke checks must not touch saves'
    (plugin / 'bin/sokobanchy').unlink()
    result = subprocess.run([launcher, '--version'], env=env, text=True, capture_output=True)
    assert result.returncode == 1 and 'missing' in result.stderr
    (commands / 'uname').unlink()
    (commands / 'uname').write_text('#!/bin/sh\ncase "$1" in -s) echo Linux;; -m) echo aarch64;; esac\n')
    (commands / 'uname').chmod(0o755)
    result = subprocess.run([launcher], env=env, text=True, capture_output=True)
    assert result.returncode == 1 and 'x86_64' in result.stderr
print('Launcher passed: bundled executable, paths with spaces, argument forwarding, no build tools, clear errors')
