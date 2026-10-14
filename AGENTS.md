# Project instructions

Sokobanchy is a native Rust/egui game with Omarchy Quattro application-launcher integration.

## Reference-file safety

`shared/` contains user-provided reference files. Preserve them unchanged.
The Delphi `.dfm` files can be misidentified as JPEG XL by Linux MIME tools;
thumbnail generation previously exhausted memory and swap on this machine.
Never open `shared/YASC_1_690_src/` in a graphical file browser, generate its
thumbnails, or send `.dfm` files to image decoders. Inspect reference files
using binary-safe command-line tools. Explicitly exclude `.dfm` files from
all image-processing inputs instead of trusting detected MIME types.
`shared/` is excluded from version control and release artifacts.

## Validation

Run `cargo fmt --all --check`, `cargo clippy --all-targets --locked -- -D warnings`,
`cargo test --locked`, and `omarchy plugin validate .` after relevant changes.
The portable manifest check is `python3 scripts/check-manifest.py`.
Use `--fresh` for temporary GUI smoke tests. Never change a desktop theme
just to test the game; use `--theme` with a temporary palette file.
Keep generated Cargo output outside the installed plugin checkout to avoid
triggering shell reloads. Read RELEASING.md before preparing publication.
