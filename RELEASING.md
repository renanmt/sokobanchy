# Releasing Sokobanchy

Release version: **0.1.0**. Author: **Renan** (`renanmt`). Plugin ID: `renan.sokobanchy`.
Public repository: <https://github.com/renanmt/sokobanchy>.

The repository URL, GitHub release, and marketplace listing must be verified after publication. Local validation does not mean marketplace acceptance.

## Local release checks

Keep generated build output outside the plugin checkout:

```sh
export CARGO_TARGET_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/sokobanchy/target"
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
omarchy plugin validate .
python3 scripts/check-manifest.py
bash -n scripts/*.sh packaging/*.sh
./scripts/package.sh
python3 scripts/bundle.py check
python3 scripts/test-launch.py
(cd dist && sha256sum -c SHA256SUMS)
```

Review `dist/sokobanchy-0.1.0-linux-*.tar.gz` and `dist/sokobanchy-0.1.0-source.tar.gz`. The binary archive includes the matching source, app and dependency licenses, and original collection notices under `THIRD_PARTY/collections`. `shared/`, Cargo target output, Python caches, and personal progress must not appear in either archive. Build on current Omarchy x86_64. The plugin includes its executable under `bin/`, and the source archive mirrors that complete checkout. Omarchy installs and launches the bundled executable without build tools.

The README and marketplace preview use captures of the actual egui renderer. Refresh them using the opt-in capture command in README. Never change the desktop theme for a capture or include personal desktop content.

Before submitting, smoke-test the bar launcher on Omarchy Quattro without Rust or Cargo on PATH, plus theme reload, keyboard controls, and removal. Use `--fresh` for gameplay tests. Test persistent reset using a temporary `XDG_STATE_HOME`, never a real player's save. The offscreen captures and automated tests do not replace a real compositor/bar test.

## Bundled executable

`./scripts/package.sh` builds the game and refreshes `bin/sokobanchy`, `bin/THIRD_PARTY`, and `bin/BUILD.json`. Commit these together with source changes. The metadata records the compiler, platform, embedded source hashes, and all bundle file hashes; CI rejects stale bundles. The launcher test runs on Arch Linux with a restricted PATH containing only Bash, dirname, and uname, using the real bundled executable.

The executable is an ordinary tracked ELF file, not a Git LFS pointer or a downloaded runtime payload. The marketplace scanner reports bundled executable capability for maintainer review. This is separate from manual setup: the standard plugin command produces a working installation on supported Omarchy x86_64.

## GitHub publication

From the intended Git checkout, review the staged file list before committing. Do not include `shared/`, `target/`, `dist/`, credentials, or local progress. Preserve any existing repository history; never force-push to establish a release.

If the repository does not exist, create it as public and push the reviewed commit:

```sh
gh repo create renanmt/sokobanchy --public --source . --remote origin --push
```

If it already exists, fetch its history and integrate the release changes before pushing. After CI passes, tag the reviewed commit and create a draft release:

```sh
git tag -a v0.1.0 -m 'Sokobanchy 0.1.0'
git push origin v0.1.0
gh release create v0.1.0 --repo renanmt/sokobanchy --verify-tag --draft \
  --title 'Sokobanchy 0.1.0' --notes-file CHANGELOG.md \
  dist/sokobanchy-0.1.0-linux-*.tar.gz \
  dist/sokobanchy-0.1.0-source.tar.gz dist/SHA256SUMS
```

Inspect the draft's files, source/notice completeness, and README images before publishing it. Update the marketplace status only after acceptance.

## Marketplace submission draft

Follow the [official publishing guide](https://plugins.omarchy.org/publish.html) and its [submission form](https://github.com/omacom/omarchy-plugin-marketplace/issues/new?template=submit-plugin.yml). Marketplace maintainers review submissions. The repository must be public and include the root manifest, README, license, and safe install/removal behavior. See the [plugin reference](https://github.com/omacom/omarchy/blob/quattro/manual/32-shell-plugins.md).

- **Name:** Sokobanchy
- **Repository:** https://github.com/renanmt/sokobanchy
- **Author:** Renan
- **Category:** Other
- **Tags:** Games, Bar, Launcher
- **Preview:** root `preview.png`
- **Summary:** A native Sokoban game with a 35-puzzle campaign, eight credited community collections, keyboard controls, and live Omarchy colors.

The marketplace form has no Games category; use Other with the Games, Bar, and Launcher tags. The widget's own bar category remains Games.

**Description:** Guide an AI agent and push code libraries onto placeholders. Undo and redo freely, browse puzzles beside the board, open your own collections, and resume saved games with best scores. Help, About, credits, and reset-all confirmation stay in the main body. Single-puzzle restart uses a centered floating Yes/No dialog with Y/N shortcuts. A persistent bottom bar shows keyboard shortcuts.

**Installation/runtime notes:** Requires Omarchy Quattro on x86_64 with its standard OpenGL-capable Wayland/X11 desktop. Install with `omarchy plugin add https://github.com/renanmt/sokobanchy --enable`, then click the bar icon. The native executable, source, build hashes, dependency licenses, and font notices are in the repository. No Rust installation, compilation, package installation, executable download, privileged command, or build-cache write occurs on activation or launch. Progress is under `${XDG_STATE_HOME:-~/.local/state}/sokobanchy`. Theme palettes are read only. Removing the plugin preserves progress; the optional standalone installer has its own uninstaller. The bundled ELF requires maintainer review under the marketplace's binary capability policy.

**License:** GPL-3.0-or-later for the application. Omarchy icon geometry and community collections retain the notices and permissions shipped with the project. Collection provenance and inclusion decisions are documented in `assets/collections/README.md`.
