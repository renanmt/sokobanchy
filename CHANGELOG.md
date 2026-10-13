# Changelog

## 0.1.0

Initial public release preparation.

- Native Rust/egui Sokoban with live Omarchy theme colors and a Quattro bar launcher.
- **The night shift**: 35 unique original puzzles in seven chapters, ordered by verified minimum pushes.
- Eight credited community collections containing 1,984 entries; support for opening UTF-8 and Windows-1252 collection files.
- Keyboard movement, undo/redo, puzzle browsing, help, and persistent shortcut hints.
- Puzzle sidebar beside the board; About, Help, Open, and reset-all confirmation inside the main body. Single-puzzle restart uses a centered floating Yes/No dialog with Y/N shortcuts.
- Automatic local saves, resumable puzzles, and best scores.
- **Reset all** in the bottom bar (Ctrl+Shift+R) clears saved progress and scores across all collections after explicit confirmation with Y; N/Escape cancels. Tab/Shift+Tab and Enter/Space also work, with Cancel focused first. The single-puzzle restart remains R, then Y or Enter, preserving scores. Failed writes preserve existing progress.
- Author and collection credits, distribution notices, README screenshots, and source plus Linux binary packaging.
