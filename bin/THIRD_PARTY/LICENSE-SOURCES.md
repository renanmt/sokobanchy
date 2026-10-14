# Dependency license supplements

These upstream workspace licenses are omitted from the corresponding crates.io source archives. `scripts/package.sh` uses these reviewed, version-specific supplements and copies the other license/notice files from Cargo's registry sources. It includes the actual normal/build dependency tree and fails if any package has no notice files. Recheck these mappings when upgrading dependencies.

- **egui family 0.33.3**: [upstream MIT license](https://raw.githubusercontent.com/emilk/egui/0.33.3/LICENSE-MIT). Covers ecolor, eframe, egui, egui-winit, egui_glow, emath, epaint, and epaint_default_fonts. The MIT alternative is used here. The fonts additionally require their own notices; packaging copies `Hack-Regular.txt`, `OFL.txt`, `UFL.txt`, and `emoji-icon-font-mit-license.txt` from epaint_default_fonts' `fonts/` directory.
- **profiling 1.0.18**: [upstream MIT license at its recorded revision](https://raw.githubusercontent.com/aclysma/profiling/8271551172eb6fa4cba47369aedd93790c623df9/LICENSE-MIT). The MIT alternative is used here.
- **gl_generator 0.14.0** and **khronos_api 3.1.0**: gl-rs Apache-2.0 license, [gl_generator revision](https://raw.githubusercontent.com/brendanzab/gl-rs/ea503e8d5fb6d73c6030e6191ce738cd3bf3433e/LICENSE). khronos_api's recorded source revision is `f150967b1c44ae888e6676f93f639ebc82771bdc` in the same repository; both crates declare Apache-2.0.

The stored text preserves the upstream notices; line wrapping follows the retrieved text. These notices apply to dependencies, independently of Sokobanchy's GPL-3.0-or-later license.
