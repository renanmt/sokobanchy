#!/usr/bin/env bash
# Omarchy installs the executable with the plugin; launching needs no build tools.
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if [[ $(uname -s) != Linux || $(uname -m) != x86_64 ]]; then
    printf '%s\n' 'This Sokobanchy plugin build requires Omarchy on x86_64.' >&2
    exit 1
fi
binary="$project_dir/bin/sokobanchy"
if [[ ! -x "$binary" ]]; then
    printf '%s\n' 'The bundled Sokobanchy executable is missing. Reinstall or update the plugin.' >&2
    exit 1
fi
exec "$binary" "$@"
