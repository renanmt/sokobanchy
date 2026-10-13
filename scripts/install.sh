#!/usr/bin/env bash
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
install_prefix="${PREFIX:-$HOME/.local}"
data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"
cargo build --release --locked --manifest-path "$project_dir/Cargo.toml"
build_dir="${CARGO_TARGET_DIR:-$project_dir/target}"
install -Dm755 "$build_dir/release/sokobanchy" "$install_prefix/bin/sokobanchy"
install -Dm644 "$project_dir/assets/sokobanchy.svg" "$data_dir/icons/hicolor/scalable/apps/sokobanchy.svg"
# An absolute quoted Exec works even when ~/.local/bin is not on the desktop PATH.
python3 - "$project_dir/packaging/sokobanchy.desktop" "$data_dir/applications/sokobanchy.desktop" "$install_prefix/bin/sokobanchy" <<'PY'
import pathlib, sys
source, target, binary = sys.argv[1:]
escaped = binary.replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$').replace('%', '%%')
path = pathlib.Path(target)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(pathlib.Path(source).read_text().replace('Exec=sokobanchy', 'Exec="' + escaped + '"'))
PY
command -v update-desktop-database >/dev/null && update-desktop-database "$data_dir/applications" || true
printf 'Installed %s\n' "$install_prefix/bin/sokobanchy"
