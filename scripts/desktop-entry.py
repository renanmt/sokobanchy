#!/usr/bin/env python3
"""Register the plugin's app entry; remove only the entry owned by this instance."""
import fcntl
import os
from pathlib import Path
import select
import sys
import tempfile

MARKER = 'X-Sokobanchy-Plugin=renan.sokobanchy'


def value(text):
    return text.replace('\\', '\\\\').replace('\n', '\\n').replace('\r', '\\r').replace('\t', '\\t')


def executable(path):
    # Exec quoting is parsed after the desktop entry's general string escaping.
    quoted = str(path).replace('%', '%%')
    for char in ('\\', '"', '`', '$'):
        quoted = quoted.replace(char, '\\' + char)
    return value('"' + quoted + '"')


def main():
    action, directory, owner = sys.argv[1:4]
    if action not in ('register', 'remove'):
        raise ValueError('Expected register or remove')
    if action == 'remove' and len(sys.argv) > 4 and int(sys.argv[4]):
        # A destroyed QML Process is killed by Quickshell. Wait for any in-flight
        # registration before cleanup, without relying on the deleted checkout.
        try:
            fd = os.pidfd_open(int(sys.argv[4]))
        except ProcessLookupError:
            pass
        else:
            try:
                if not select.select([fd], [], [], 10)[0]:
                    raise TimeoutError('Desktop registration did not exit before cleanup')
            finally:
                os.close(fd)

    applications = Path(os.environ.get('XDG_DATA_HOME') or Path.home() / '.local/share') / 'applications'
    applications.mkdir(parents=True, exist_ok=True)
    target = applications / 'sokobanchy.desktop'
    ownership = 'X-Sokobanchy-Instance=' + owner
    # Serialize reload cleanup with registration; an old instance must not remove
    # a newer instance's entry. Never overwrite a standalone/user-created entry.
    with (applications / '.sokobanchy-plugin.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if target.is_symlink():
            return
        existing = target.read_text() if target.exists() else ''
        if action == 'remove':
            if MARKER in existing.splitlines() and ownership in existing.splitlines():
                target.unlink()
            return
        if target.exists() and MARKER not in existing.splitlines():
            return
        root = Path(directory).absolute()
        binary = root / 'bin/sokobanchy'
        if not os.access(binary, os.X_OK):
            raise FileNotFoundError('Bundled Sokobanchy executable is missing or not executable')
        desktop = (root / 'packaging/sokobanchy.desktop').read_text()
        # Keep the executable token simple for desktop parsers; env forwards the
        # quoted path as one argument, including literal percent signs.
        desktop = desktop.replace('Exec=sokobanchy', 'Exec=/usr/bin/env ' + executable(binary))
        desktop = desktop.replace('Icon=sokobanchy', 'Icon=' + value(str(root / 'assets/sokobanchy.svg')))
        # A stale entry after an abrupt shell crash cannot expose a removed game.
        desktop += 'TryExec=' + value(str(binary)) + '\n' + MARKER + '\n' + ownership + '\n'
        fd, temporary = tempfile.mkstemp(prefix='.sokobanchy-', dir=applications)
        try:
            with os.fdopen(fd, 'w') as stream:
                stream.write(desktop)
                os.fchmod(stream.fileno(), 0o644)
            os.replace(temporary, target)
        finally:
            Path(temporary).unlink(missing_ok=True)


if __name__ == '__main__':
    main()
