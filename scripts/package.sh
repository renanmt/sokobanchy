#!/usr/bin/env bash
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$project_dir"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/sokobanchy/target}"
cargo build --release --locked
version=$(python3 -c 'import json; print(json.load(open("manifest.json"))["version"])')
archive_name="sokobanchy-$version-linux-$(uname -m)"
staging=$(mktemp -d)
trap 'rm -rf -- "$staging"' EXIT
release_dir="$staging/$archive_name"
mkdir -p -- "$release_dir/THIRD_PARTY" dist
install -m755 "${CARGO_TARGET_DIR:-target}/release/sokobanchy" "$release_dir/sokobanchy"
install -m755 packaging/install-release.sh "$release_dir/install.sh"
install -m755 scripts/uninstall.sh "$release_dir/uninstall.sh"
cp -- assets/OMARCHY-LICENSE "$release_dir/THIRD_PARTY/OMARCHY-LICENSE"
cp -R -- assets/collections "$release_dir/THIRD_PARTY/collections"
cp -- README.md LICENSE NOTICE preview.png "$release_dir/"
mkdir -p -- "$release_dir/assets"
cp -R -- assets/screenshots "$release_dir/assets/screenshots"
cp -- assets/sokobanchy.svg packaging/sokobanchy.desktop "$release_dir/"
cargo metadata --locked --format-version 1 --filter-platform "$(rustc -vV | sed -n 's/^host: //p')" > "$staging/metadata.json"
cargo tree --locked --edges normal,build --prefix none --format '{p}' > "$staging/dependencies.txt"
python3 - "$staging/metadata.json" "$release_dir/THIRD_PARTY" "$staging/dependencies.txt" <<'PY'
import json, pathlib, shutil, sys
metadata = json.loads(pathlib.Path(sys.argv[1]).read_text())
dest = pathlib.Path(sys.argv[2])
index = []
used = {(parts[0], parts[1].removeprefix('v'))
        for line in pathlib.Path(sys.argv[3]).read_text().splitlines()
        if len(parts := line.split()) >= 2}
# These exact crate releases omit their workspace license files.
fallbacks = {f'{name}-0.33.3': 'egui-MIT.txt' for name in
             ('ecolor', 'eframe', 'egui', 'egui-winit', 'egui_glow', 'emath',
              'epaint', 'epaint_default_fonts')}
fallbacks.update({'profiling-1.0.18': 'profiling-MIT.txt',
                  'gl_generator-0.14.0': 'gl-rs-Apache-2.0.txt',
                  'khronos_api-3.1.0': 'gl-rs-Apache-2.0.txt'})
for package in metadata['packages']:
    if package['source'] is None or (package['name'], package['version']) not in used:
        continue
    name = package['name'] + '-' + package['version']
    source = pathlib.Path(package['manifest_path']).parent
    out = dest / name
    out.mkdir()
    index.append(name + ': ' + (package['license'] or 'see license file'))
    files = set()
    for pattern in ('LICENSE*', 'LICENCE*', 'COPYING*', 'NOTICE*'):
        files.update(source.glob(pattern))
    if package.get('license_file'):
        files.add(source / package['license_file'])
    if name in fallbacks:
        shutil.copyfile(pathlib.Path('assets/dependency-licenses') / fallbacks[name],
                        out / fallbacks[name])
    if package['name'] == 'epaint_default_fonts':
        # All four bundled font families carry their own notices.
        for f in (source / 'fonts').glob('*.txt'):
            shutil.copyfile(f, out / f.name)
    for f in files:
        if f.is_file():
            shutil.copyfile(f, out / f.name)
        elif f.is_dir():
            shutil.copytree(f, out / f.name, dirs_exist_ok=True)
    if not any(out.iterdir()):
        raise SystemExit(f'Missing license notices for {name}; review before packaging')
shutil.copyfile('assets/dependency-licenses/README.md', dest / 'LICENSE-SOURCES.md')
(dest / 'INDEX.txt').write_text('\n'.join(sorted(index)) + '\n')
PY
python3 scripts/bundle.py prepare --binary "$release_dir/sokobanchy" --licenses "$release_dir/THIRD_PARTY"
# Include the exact application source with every binary archive. Use an allowlist;
# never traverse shared/ or hand source-reference files to desktop thumbnailers.
tar --exclude='__pycache__' --exclude='*.pyc' -czf "$release_dir/source.tar.gz" Cargo.toml Cargo.lock src assets bin tests scripts examples packaging Widget.qml manifest.json README.md RELEASING.md CHANGELOG.md LICENSE NOTICE AGENTS.md preview.png .github .gitignore .gitattributes
cp -- "$release_dir/source.tar.gz" "dist/sokobanchy-$version-source.tar.gz"
tar -C "$staging" -czf "dist/$archive_name.tar.gz" "$archive_name"
(cd dist && sha256sum "$archive_name.tar.gz" "sokobanchy-$version-source.tar.gz") > dist/SHA256SUMS
printf 'Created dist/%s.tar.gz\n' "$archive_name"
