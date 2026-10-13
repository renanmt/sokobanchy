#!/usr/bin/env bash
set -euo pipefail
install_prefix="${PREFIX:-$HOME/.local}"
data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"
rm -f -- "$install_prefix/bin/sokobanchy" "$data_dir/applications/sokobanchy.desktop" "$data_dir/icons/hicolor/scalable/apps/sokobanchy.svg"
command -v update-desktop-database >/dev/null && update-desktop-database "$data_dir/applications" || true
printf '%s\n' 'Removed the standalone installation. Progress and build cache have been preserved.'
