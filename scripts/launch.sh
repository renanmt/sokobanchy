#!/usr/bin/env bash
# Build only on a deliberate click/run. Never runs when the widget loads.
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/sokobanchy"
mkdir -p -- "$cache_dir"
# Serialize clicks while compiling; release the lock before starting the game.
exec 9>"$cache_dir/build.lock"
if ! flock -n 9; then
    command -v notify-send >/dev/null && notify-send Sokobanchy 'The game is already building. It will open when ready.' || true
    exit 0
fi
export CARGO_TARGET_DIR="$cache_dir/target"
if ! command -v cargo >/dev/null; then
    if [[ -x "$HOME/.cargo/bin/cargo" ]]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        command -v notify-send >/dev/null && notify-send Sokobanchy 'Rust is required for the first build. Install rust and base-devel, then try again.' || true
        printf '%s\n' 'Rust is required. On Omarchy: omarchy pkg add rust base-devel' >&2
        exit 1
    fi
fi
# Cargo checks source changes after plugin updates. Target files stay outside the watched plugin tree.
if [[ ! -x "$CARGO_TARGET_DIR/release/sokobanchy" ]]; then
    command -v notify-send >/dev/null && notify-send Sokobanchy 'Building the game for the first time. This can take a few minutes.' || true
fi
if ! cargo build --release --locked --jobs "${CARGO_BUILD_JOBS:-2}" --manifest-path "$project_dir/Cargo.toml" >"$cache_dir/build.log" 2>&1; then
    command -v notify-send >/dev/null && notify-send Sokobanchy "Build failed. See $cache_dir/build.log" || true
    cat -- "$cache_dir/build.log" >&2
    exit 1
fi
flock -u 9
exec 9>&-
exec "$CARGO_TARGET_DIR/release/sokobanchy" "$@"
