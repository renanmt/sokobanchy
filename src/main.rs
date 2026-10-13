mod art;
mod campaign;

#[cfg(test)]
mod screenshots;

use eframe::egui::{self, Color32, Key, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use sokobanchy::{
    collections::COLLECTIONS,
    game::{Direction, Game, Level, Pos, Tile, parse_collection, parse_collection_bytes},
    progress::Progress,
    theme::Theme,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct Options {
    levels: Option<PathBuf>,
    theme: Option<PathBuf>,
    fresh: bool,
}
fn options() -> Result<Option<Options>, String> {
    let mut out = Options {
        levels: None,
        theme: None,
        fresh: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "Sokobanchy — a Sokoban game for Omarchy\n\nUsage: sokobanchy [--levels FILE] [--theme COLORS.toml] [--fresh]\n\n  --levels FILE   Open an XSB or plain-text SOK collection\n  --theme FILE    Read and live-reload a specific Omarchy palette\n  --fresh         Start with temporary progress; do not read or write saves\n  --version       Print version\n\nMove: arrows / WASD / HJKL   Undo: Ctrl+Z / U / Z   Redo: Ctrl+Shift+Z / Ctrl+Y / Y\nRestart: R, then Y/Enter (N/Escape cancels)   Reset all: Ctrl+Shift+R, then Y (N/Escape cancels)\nPuzzles: Tab / [ / ]   Next solved level: Enter\nSidebar: arrows / J K, Home / End, PgUp / PgDn, Enter to play\nOpen: O / Ctrl+O   Help: F1   About: F2   Back: Escape"
                );
                return Ok(None);
            }
            "--version" | "-V" => {
                println!("sokobanchy {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--levels" => out.levels = Some(args.next().ok_or("--levels needs a file")?.into()),
            "--theme" => out.theme = Some(args.next().ok_or("--theme needs a file")?.into()),
            "--fresh" => out.fresh = true,
            _ => return Err(format!("Unknown argument: {arg}. Try --help.")),
        }
    }
    if let Some(p) = &out.theme {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        if Theme::parse(&text, String::new()).is_none() {
            return Err("Theme file is not valid TOML".into());
        }
    }
    Ok(Some(out))
}
fn read_levels(path: &std::path::Path) -> Result<Vec<Level>, String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_collection_bytes(&data)
}
fn main() -> eframe::Result {
    let opts = match options() {
        Ok(Some(o)) => o,
        Ok(None) => return Ok(()),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let levels = match &opts.levels {
        Some(path) => read_levels(path),
        None => parse_collection(include_str!("../assets/levels.xsb")),
    };
    let levels = match levels {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    eframe::run_native(
        "Sokobanchy",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([880.0, 680.0])
                .with_min_inner_size([600.0, 460.0])
                .with_app_id("sokobanchy"),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(App::new(&cc.egui_ctx, opts, levels)))),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Page {
    Game,
    About,
    Help,
    Collections,
    Puzzles,
    Restart,
    ResetAll,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Self::Game => "Puzzle",
            Self::About => "About Sokobanchy",
            Self::Help => "How to play",
            Self::Collections => "Open a collection",
            Self::Puzzles => "Puzzles",
            Self::Restart => "A fresh start?",
            Self::ResetAll => "Reset all puzzles and scores?",
        }
    }
}

struct App {
    levels: Vec<Level>,
    selected: usize,
    game: Game,
    progress: Progress,
    theme: Theme,
    theme_path: Option<PathBuf>,
    theme_checked: Instant,
    save_path: Option<PathBuf>,
    fresh: bool,
    collection: String,
    campaign: bool,
    page: Page,
    restart_focus_pending: bool,
    reset_all_focus_pending: bool,
    puzzle_cursor: usize,
    scroll_to_puzzle: bool,
    board_rect: Rect,
    tile_size: f32,
    import_path: String,
    error: Option<String>,
    smile_until: Option<Instant>,
}
impl App {
    fn new(ctx: &egui::Context, opts: Options, levels: Vec<Level>) -> Self {
        let mut style = (*ctx.style()).clone();
        for font in style.text_styles.values_mut() {
            font.family = egui::FontFamily::Monospace;
        }
        style.spacing.button_padding = Vec2::new(10.0, 6.0);
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        ctx.set_style(style);
        // Prefer the user's desktop monospace face when available; bundled egui fonts are the fallback.
        for path in [
            "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf",
            "/usr/share/fonts/TTF/JetBrainsMono-Regular.ttf",
        ] {
            if let Ok(data) = std::fs::read(path) {
                let mut fonts = egui::FontDefinitions::default();
                fonts.font_data.insert(
                    "desktop-mono".into(),
                    egui::FontData::from_owned(data).into(),
                );
                fonts
                    .families
                    .get_mut(&egui::FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "desktop-mono".into());
                ctx.set_fonts(fonts);
                break;
            }
        }
        let mut save_path = (!opts.fresh).then(Progress::path);
        let (progress, error) = match save_path.as_ref().map(|p| Progress::load(p)).transpose() {
            Ok(p) => (p.unwrap_or_default(), None),
            Err(e) => {
                save_path = None;
                (
                    Progress::default(),
                    Some(format!(
                        "{e}. Saving disabled to preserve the existing file."
                    )),
                )
            }
        };
        let selected = levels
            .iter()
            .position(|l| l.id == progress.current)
            .unwrap_or(0);
        let mut game = Game::new(levels[selected].clone());
        progress.restore(&mut game);
        let theme = Theme::load(opts.theme.as_deref());
        theme.apply(ctx);
        Self {
            levels,
            selected,
            game,
            progress,
            theme,
            theme_path: opts.theme,
            theme_checked: Instant::now(),
            save_path,
            fresh: opts.fresh,
            campaign: opts.levels.is_none(),
            page: Page::Game,
            restart_focus_pending: false,
            reset_all_focus_pending: false,
            puzzle_cursor: selected,
            scroll_to_puzzle: false,
            board_rect: Rect::NOTHING,
            tile_size: 0.0,
            collection: opts
                .levels
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_else(|| campaign::NAME.into()),
            import_path: String::new(),
            error,
            smile_until: None,
        }
    }
    fn persist(&mut self) {
        self.progress.capture(&self.game);
        if let Some(path) = &self.save_path
            && let Err(e) = self.progress.save(path)
        {
            self.error = Some(format!("Could not save progress: {e}"));
        }
    }
    fn select(&mut self, index: usize) {
        if index >= self.levels.len() {
            return;
        }
        self.persist();
        self.selected = index;
        self.game = Game::new(self.levels[index].clone());
        self.smile_until = None;
        self.progress.restore(&mut self.game);
        self.persist();
    }
    fn smiling_at(&self, now: Instant) -> bool {
        self.game.solved() || self.smile_until.is_some_and(|until| now < until)
    }
    fn celebrate_delivery(&mut self, previous: &std::collections::BTreeSet<Pos>) {
        // Compare positions rather than the count: pushing from one goal to another
        // is a fresh delivery too, even when the number of occupied goals is unchanged.
        if self
            .game
            .crates
            .difference(previous)
            .any(|&pos| self.game.level.tile(pos) == Tile::Goal)
        {
            self.smile_until = Some(Instant::now() + Duration::from_millis(850));
        }
    }
    fn move_player(&mut self, direction: Direction) {
        if self.game.solved() {
            return;
        }
        let previous = self.game.crates.clone();
        if self.game.step(direction) {
            self.celebrate_delivery(&previous);
            self.persist();
        }
    }
    fn undo(&mut self) {
        if self.game.undo() {
            self.smile_until = None;
            self.persist();
        }
    }
    fn redo(&mut self) {
        let previous = self.game.crates.clone();
        if self.game.redo() {
            self.celebrate_delivery(&previous);
            self.persist();
        }
    }
    fn restart_now(&mut self) {
        self.game = Game::new(self.levels[self.selected].clone());
        self.smile_until = None;
        self.page = Page::Game;
        self.persist();
    }
    fn open_restart(&mut self) {
        self.page = Page::Restart;
        self.restart_focus_pending = true;
    }
    fn toggle_puzzles(&mut self) {
        if self.page == Page::Puzzles {
            self.page = Page::Game;
        } else {
            self.page = Page::Puzzles;
            self.puzzle_cursor = self.selected;
            self.scroll_to_puzzle = true;
        }
    }

    fn open_reset_all(&mut self) {
        if self.page != Page::ResetAll {
            self.page = Page::ResetAll;
            self.reset_all_focus_pending = true;
        }
    }

    fn reset_all_progress(&mut self) {
        if self.save_path.is_none() && !self.fresh {
            self.error = Some("Reset unavailable while saving is disabled to protect an unreadable progress file.".into());
            return;
        }
        let reset = Progress {
            current: self.levels[0].id.clone(),
            ..Progress::default()
        };
        // Commit the empty save before changing the live game. A failed write
        // must leave both the old scores and the current puzzle recoverable.
        if let Some(path) = &self.save_path
            && let Err(error) = reset.save(path)
        {
            self.error = Some(format!(
                "Could not reset progress: {error}. Your progress was kept."
            ));
            return;
        }
        self.progress = reset;
        self.selected = 0;
        self.game = Game::new(self.levels[0].clone());
        self.puzzle_cursor = 0;
        self.scroll_to_puzzle = false;
        self.smile_until = None;
        self.error = None;
        self.page = Page::Game;
    }

    fn input(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            self.page = Page::Game;
            return;
        }
        for (key, page) in [(Key::F1, Page::Help), (Key::F2, Page::About)] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                // Function keys open their page; key repeat must not close it again.
                self.page = page;
                return;
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, Key::R))
        {
            self.open_reset_all();
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, Key::O)) {
            self.page = Page::Collections;
            return;
        }
        if self.page == Page::Collections
            && ctx.memory(|m| m.has_focus(egui::Id::new("collection_path")))
        {
            return;
        }
        if matches!(self.page, Page::Game | Page::Puzzles)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Tab))
        {
            self.toggle_puzzles();
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::O)) {
            self.page = if self.page == Page::Collections {
                Page::Game
            } else {
                Page::Collections
            };
            return;
        }
        if self.page == Page::ResetAll {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::N)) {
                self.page = Page::Game;
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Y)) {
                self.reset_all_progress();
            }
            return;
        }
        if self.page == Page::Restart {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::N)) {
                self.page = Page::Game;
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Y))
                || (!ctx.wants_keyboard_input()
                    && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter)))
            {
                self.restart_now();
            }
            return;
        }
        if self.page == Page::Puzzles {
            let previous = self.puzzle_cursor;
            for (key, delta) in [
                (Key::ArrowUp, -1),
                (Key::K, -1),
                (Key::ArrowDown, 1),
                (Key::J, 1),
                (Key::PageUp, -10),
                (Key::PageDown, 10),
            ] {
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                    self.puzzle_cursor = self
                        .puzzle_cursor
                        .saturating_add_signed(delta)
                        .min(self.levels.len() - 1);
                }
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Home)) {
                self.puzzle_cursor = 0;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::End)) {
                self.puzzle_cursor = self.levels.len() - 1;
            }
            self.scroll_to_puzzle |= previous != self.puzzle_cursor;
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter)) {
                self.select(self.puzzle_cursor);
                self.page = Page::Game;
            }
            return;
        }
        if self.page != Page::Game {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                repeat,
                ..
            } = event
            else {
                continue;
            };
            if modifiers.alt {
                continue;
            }
            if modifiers.command || modifiers.ctrl {
                match key {
                    Key::Z if modifiers.shift => self.redo(),
                    Key::Z => self.undo(),
                    Key::Y => self.redo(),
                    _ => {}
                }
                continue;
            }
            match key {
                Key::ArrowUp | Key::W | Key::K => self.move_player(Direction::Up),
                Key::ArrowDown | Key::S | Key::J => self.move_player(Direction::Down),
                Key::ArrowLeft | Key::A | Key::H => self.move_player(Direction::Left),
                Key::ArrowRight | Key::D | Key::L => self.move_player(Direction::Right),
                Key::U | Key::Z => self.undo(),
                Key::Y => self.redo(),
                Key::R if !repeat && self.game.moves > 0 => {
                    self.open_restart();
                    return;
                }
                Key::OpenBracket if !repeat && self.selected > 0 => self.select(self.selected - 1),
                Key::CloseBracket if !repeat && self.selected + 1 < self.levels.len() => {
                    self.select(self.selected + 1)
                }
                Key::Enter
                    if !repeat && self.game.solved() && self.selected + 1 < self.levels.len() =>
                {
                    self.select(self.selected + 1)
                }
                _ => {}
            }
        }
    }
    fn logo(ui: &mut egui::Ui, color: Color32, max_width: f32) {
        let lines: Vec<_> = include_str!("../assets/logo.txt").lines().collect();
        let columns = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1) as f32;
        let cell = (ui.available_width().min(max_width) / columns).min(5.5);
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(columns * cell, lines.len() as f32 * cell * 1.5),
            Sense::hover(),
        );
        for (y, line) in lines.iter().enumerate() {
            for (x, c) in line.chars().enumerate() {
                let (offset, height) = match c {
                    '█' => (0.0, 1.0),
                    '▀' => (0.0, 0.5),
                    '▄' => (0.5, 0.5),
                    '▐' => (0.0, 1.0),
                    _ => continue,
                };
                let narrow = if c == '▐' { 0.5 } else { 0.0 };
                let min = rect.min
                    + Vec2::new((x as f32 + narrow) * cell, (y as f32 + offset) * cell * 1.5);
                ui.painter().rect_filled(
                    Rect::from_min_size(min, Vec2::new(cell * (1.0 - narrow), cell * 1.5 * height)),
                    0,
                    color,
                );
            }
        }
    }
    fn board(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, Sense::click());
        if response.clicked() {
            response.request_focus();
        }
        let p = ui.painter_at(rect);
        let now = Instant::now();
        let smiling = self.smiling_at(now);
        if !self.game.solved()
            && let Some(until) = self.smile_until.filter(|&until| until > now)
        {
            // Repaint when the smile expires, even if the player stops pressing keys.
            ui.ctx().request_repaint_after(until.duration_since(now));
        }
        let t = &self.theme;
        let tile_size = ((available.x - 16.0) / self.game.level.width as f32)
            .min((available.y - 16.0) / self.game.level.height as f32)
            .clamp(0.5, 100.0);
        self.tile_size = tile_size;
        let size = Vec2::new(
            self.game.level.width as f32 * tile_size,
            self.game.level.height as f32 * tile_size,
        );
        let origin = rect.center() - size / 2.0;
        self.board_rect = Rect::from_min_size(origin, size);
        // Fine background dots keep the board grounded without distracting from the puzzle.
        let spacing = 22.0;
        for y in 0..(available.y / spacing) as i32 {
            for x in 0..(available.x / spacing) as i32 {
                p.circle_filled(
                    rect.min + Vec2::new(x as f32 * spacing + 10.0, y as f32 * spacing + 10.0),
                    0.7,
                    t.surface,
                );
            }
        }
        for y in 0..self.game.level.height {
            for x in 0..self.game.level.width {
                let pos = Pos(x as i32, y as i32);
                let tile = self.game.level.tile(pos);
                let r = Rect::from_min_size(
                    origin + Vec2::new(x as f32 * tile_size, y as f32 * tile_size),
                    Vec2::splat(tile_size),
                )
                .shrink((tile_size * 0.025).min(1.5));
                if tile == Tile::Void {
                    continue;
                }
                if tile == Tile::Wall {
                    p.rect_filled(r, 2, t.surface);
                    let line = blend(t.surface, t.muted, 0.22);
                    p.line_segment(
                        [
                            r.left_top() + Vec2::new(2.0, 2.0),
                            r.right_top() + Vec2::new(-2.0, 2.0),
                        ],
                        Stroke::new(1.0_f32, line),
                    );
                    p.line_segment(
                        [
                            r.center_top() + Vec2::new(0.0, 6.0),
                            r.center_bottom() - Vec2::new(0.0, 6.0),
                        ],
                        Stroke::new(1.0_f32, blend(t.surface, t.muted, 0.10)),
                    );
                    continue;
                }
                p.rect_filled(r, 1, blend(t.background, t.panel, 0.6));
                p.rect_stroke(
                    r,
                    1,
                    Stroke::new(1.0_f32, blend(t.background, t.surface, 0.55)),
                    StrokeKind::Inside,
                );
                if self.game.crates.contains(&pos) {
                    art::library(&p, r, t, tile == Tile::Goal);
                } else if tile == Tile::Goal {
                    art::goal(&p, r, t);
                }
                if self.game.player == pos {
                    art::agent(&p, r, t, smiling);
                }
            }
        }
    }
    fn page_body(&mut self, ui: &mut egui::Ui) {
        match self.page {
            Page::About => {
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.label(
                    RichText::new(format!("Created by {}", env!("CARGO_PKG_AUTHORS"))).strong(),
                );
                ui.label("Copyright © 2026 Renan and contributors.");
                ui.label("Built with Rust and egui.");
                if ui.button("Reset all progress…").clicked() {
                    self.open_reset_all();
                }
                ui.label("Sokobanchy is free software under GPL-3.0-or-later, without warranty. Collection authors retain their own copyrights and terms.");
                ui.collapsing("Application license", |ui| {
                    ui.label(include_str!("../LICENSE"));
                });
                ui.separator();
                ui.heading("Thanks & inspiration");
                ui.label("Sokoban was created by Hiroyuki Imabayashi.");
                ui.hyperlink_to(
                    "Sokoban YASC — Brian Damgaard",
                    "https://sourceforge.net/projects/sokobanyasc/",
                );
                ui.label("YASC inspired this app and supplied the credited collections below.");
                ui.label("Omarchy mark — David Heinemeier Hansson, MIT license.");
                ui.collapsing("Omarchy license", |ui| {
                    ui.label(include_str!("../assets/OMARCHY-LICENSE"));
                });
                ui.separator();
                ui.heading("Puzzle collections");
                ui.label(format!(
                    "{} · {} puzzles",
                    campaign::NAME,
                    campaign::LEVEL_COUNT
                ));
                ui.small(format!(
                    "by {} · Original Sokobanchy campaign",
                    env!("CARGO_PKG_AUTHORS")
                ));
                for collection in COLLECTIONS {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!(
                            "{} · {} puzzles",
                            collection.name, collection.count
                        ))
                        .strong(),
                    );
                    ui.small(format!("by {}", collection.author));
                    ui.hyperlink_to("Original website", collection.website);
                    ui.push_id(collection.id, |ui| {
                        ui.collapsing("Copyright, permissions & author notes", |ui| {
                            ui.label(collection.notes);
                        });
                    });
                }
            }
            Page::Help => {
                Self::logo(ui, self.theme.accent, 320.0);
                ui.add_space(10.0);
                ui.heading("A little room. A little patience.");
                ui.label("Guide the AI agent and push each code library into an outlined goal. It becomes an Omarchy logo when placed, and a library again if pushed off. Push one at a time; you cannot pull. Every move can be undone.");
                art::legend(ui, &self.theme);
                ui.add_space(10.0);
                egui::Grid::new("keys")
                    .spacing([32.0, 12.0])
                    .show(ui, |ui| {
                        for (key, action) in [
                            ("Arrows / WASD / HJKL", "Move & push"),
                            ("Ctrl+Z / U / Z", "Undo"),
                            ("Ctrl+Shift+Z / Ctrl+Y / Y", "Redo"),
                            ("R, then Y / N", "Restart this puzzle / cancel"),
                            ("Ctrl+Shift+R", "Reset ALL puzzles & scores"),
                            ("Y / N or Esc", "Confirm / cancel reset all"),
                            ("[ / ]", "Previous / next puzzle"),
                            ("Tab", "Show / hide puzzle sidebar"),
                            ("↑ ↓ / J K", "Browse sidebar"),
                            ("Home / End", "First / last in sidebar"),
                            ("PgUp / PgDn", "Jump 10 in sidebar"),
                            ("Enter", "Play highlighted puzzle"),
                            ("Enter", "Continue after solving"),
                            ("O / Ctrl+O", "Open a level collection"),
                            ("F1", "Show this guide"),
                            ("F2", "About & credits"),
                            ("Tab / Shift+Tab", "Focus controls on pages"),
                            ("Esc", "Back to the puzzle"),
                        ] {
                            ui.monospace(key);
                            ui.label(action);
                            ui.end_row();
                        }
                    });
                ui.add_space(10.0);
                ui.label("A corner warning detects libraries trapped against walls. Other deadlocks are still possible — undo is always there.");
                if ui.button("About & credits").clicked() {
                    self.page = Page::About;
                }
            }
            Page::ResetAll => {
                ui.label("This removes all completed puzzles, best scores, saved moves, and undo history across every campaign and collection, including imported collections.");
                ui.label(
                    "Every puzzle starts at zero again. The collection files themselves are kept.",
                );
                if self.fresh {
                    ui.label("This is a temporary session: only temporary progress will be reset.");
                }
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    let cancel = ui.button("Keep my progress · N / Esc");
                    if self.reset_all_focus_pending {
                        cancel.request_focus();
                        self.reset_all_focus_pending = false;
                    }
                    if cancel.clicked() {
                        self.page = Page::Game;
                    }
                    if ui
                        .button(
                            RichText::new("Reset all puzzles and scores · Y").color(self.theme.red),
                        )
                        .clicked()
                    {
                        self.reset_all_progress();
                    }
                });
                if let Some(error) = &self.error {
                    ui.colored_label(self.theme.red, error);
                }
            }
            Page::Collections => {
                ui.label("Open an .xsb or .sok file (UTF-8 or Windows-1252).");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.import_path)
                        .id(egui::Id::new("collection_path"))
                        .desired_width(f32::INFINITY)
                        .hint_text("/home/you/puzzles.xsb"),
                );
                ui.horizontal(|ui| {
                    if ui.button("Open collection").clicked()
                        || (response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                    {
                        let path = self.import_path.trim();
                        let expanded = if let Some(rest) = path.strip_prefix("~/") {
                            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(rest)
                        } else {
                            PathBuf::from(path)
                        };
                        match read_levels(&expanded) {
                            Ok(levels) => {
                                self.persist();
                                self.levels = levels;
                                self.selected = 0;
                                self.game = Game::new(self.levels[0].clone());
                                self.smile_until = None;
                                self.progress.restore(&mut self.game);
                                self.campaign = false;
                                self.collection = expanded
                                    .file_stem()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned();
                                self.page = Page::Game;
                                self.error = None;
                                self.persist();
                            }
                            Err(e) => self.error = Some(e),
                        }
                    }
                });
                if let Some(error) = &self.error {
                    ui.colored_label(self.theme.red, error);
                }
                ui.add_space(16.0);
                ui.separator();
                ui.heading("Built-in collections");
                if ui
                    .button(format!(
                        "{} · {} puzzles",
                        campaign::NAME,
                        campaign::LEVEL_COUNT
                    ))
                    .clicked()
                {
                    self.load_builtin();
                }
                ui.small(
                    RichText::new(format!("by {}", env!("CARGO_PKG_AUTHORS")))
                        .color(self.theme.muted),
                );
                for (index, collection) in COLLECTIONS.iter().enumerate() {
                    ui.add_space(8.0);
                    if ui
                        .button(format!(
                            "{} · {} puzzles",
                            collection.name, collection.count
                        ))
                        .clicked()
                    {
                        self.load_collection(index);
                    }
                    ui.small(
                        RichText::new(format!("by {}", collection.author)).color(self.theme.muted),
                    );
                }
            }
            Page::Puzzles | Page::Game | Page::Restart => {}
        }
    }

    fn content_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(self.page.title());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Back to puzzle · Esc").clicked() {
                    self.page = Page::Game;
                }
            });
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt(self.page)
            .auto_shrink([false, false])
            .show(ui, |ui| self.page_body(ui));
    }

    fn restart_confirmation(&mut self, ctx: &egui::Context) {
        let id = egui::Id::new("restart_confirmation");
        let area = egui::Modal::default_area(id).anchor(
            egui::Align2::CENTER_CENTER,
            self.board_rect.center() - ctx.content_rect().center(),
        );
        egui::Modal::new(id).area(area).show(ctx, |ui| {
            ui.set_width(300.0);
            ui.heading("Restart this puzzle?");
            ui.label("Your best score and completed puzzles stay saved.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let yes = ui.button("Yes · Y");
                if self.restart_focus_pending {
                    yes.request_focus();
                    self.restart_focus_pending = false;
                }
                if yes.clicked() {
                    self.restart_now();
                }
                if ui.button("No · N").clicked() {
                    self.page = Page::Game;
                }
            });
        });
    }
}
fn blend(a: Color32, b: Color32, f: f32) -> Color32 {
    Color32::from_rgb(
        (a.r() as f32 * (1.0 - f) + b.r() as f32 * f) as u8,
        (a.g() as f32 * (1.0 - f) + b.g() as f32 * f) as u8,
        (a.b() as f32 * (1.0 - f) + b.b() as f32 * f) as u8,
    )
}
impl App {
    fn load_builtin(&mut self) {
        let levels = parse_collection(include_str!("../assets/levels.xsb"))
            .expect("validated bundled campaign");
        self.activate_collection(levels, campaign::NAME, true);
    }

    fn load_collection(&mut self, index: usize) {
        let collection = &COLLECTIONS[index];
        let levels = parse_collection(collection.boards).expect("validated bundled collection");
        self.activate_collection(levels, collection.name, false);
    }

    fn activate_collection(&mut self, levels: Vec<Level>, name: &str, campaign: bool) {
        self.persist();
        self.levels = levels;
        self.campaign = campaign;
        self.collection = name.into();
        self.selected = self
            .levels
            .iter()
            .position(|l| {
                self.progress
                    .records
                    .get(&l.id)
                    .is_none_or(|r| r.best.is_none())
            })
            .unwrap_or(0);
        self.game = Game::new(self.levels[self.selected].clone());
        self.smile_until = None;
        self.progress.restore(&mut self.game);
        self.page = Page::Game;
        self.error = None;
        self.persist();
    }

    fn chapter(&self, index: usize) -> &'static str {
        if !self.campaign {
            return "";
        }
        campaign::CHAPTERS
            .get(index / campaign::CHAPTER_SIZE)
            .copied()
            .unwrap_or("")
    }

    fn level_list(&mut self, ui: &mut egui::Ui) {
        let t = self.theme.clone();
        ui.label(RichText::new(&self.collection).strong());
        let completed = self
            .levels
            .iter()
            .filter(|l| {
                self.progress
                    .records
                    .get(&l.id)
                    .is_some_and(|r| r.best.is_some())
            })
            .count();
        ui.label(
            RichText::new(format!("{completed} / {} complete", self.levels.len()))
                .color(t.muted)
                .size(11.0),
        );
        ui.add(
            egui::ProgressBar::new(completed as f32 / self.levels.len() as f32)
                .fill(t.accent)
                .desired_height(3.0),
        );
        let mut choice = None;
        for (i, level) in self.levels.iter().enumerate() {
            if self.campaign && i % campaign::CHAPTER_SIZE == 0 {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{} / {}",
                        i / campaign::CHAPTER_SIZE + 1,
                        self.chapter(i)
                    ))
                    .size(11.0)
                    .color(t.muted),
                );
            }
            let solved = self
                .progress
                .records
                .get(&level.id)
                .is_some_and(|r| r.best.is_some());
            let text = format!(
                "{} {:02}  {}",
                if solved { "✓" } else { " " },
                i + 1,
                level.name
            );
            let button = egui::Button::new(RichText::new(text).size(13.0).color(
                if i == self.puzzle_cursor {
                    t.accent
                } else if solved {
                    t.green
                } else {
                    t.foreground
                },
            ))
            .fill(if i == self.puzzle_cursor {
                t.surface
            } else {
                Color32::TRANSPARENT
            });
            let response = ui.add_sized([ui.available_width(), 27.0], button);
            if i == self.puzzle_cursor && self.scroll_to_puzzle {
                response.scroll_to_me(Some(egui::Align::Center));
            }
            if response.clicked() {
                choice = Some(i);
            }
        }
        self.scroll_to_puzzle = false;
        if let Some(i) = choice {
            self.select(i);
            self.page = Page::Game;
        }
        ui.separator();
        if ui.button("Open another collection…").clicked() {
            self.page = Page::Collections;
        }
    }

    fn draw(&mut self, ctx: &egui::Context) {
        ctx.request_repaint_after(Duration::from_secs(1));
        if self.theme_checked.elapsed() >= Duration::from_secs(1) {
            let theme = Theme::load(self.theme_path.as_deref());
            if theme != self.theme {
                theme.apply(ctx);
                self.theme = theme;
            }
            self.theme_checked = Instant::now();
        }
        self.input(ctx);
        let t = self.theme.clone();
        let compact = ctx.content_rect().width() < 720.0;
        egui::TopBottomPanel::top("brand")
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| {
                if self.page == Page::Restart {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    Self::logo(ui, t.accent, if compact { 190.0 } else { 235.0 });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("About")
                            .on_hover_text("Author, credits & licenses · F2")
                            .clicked()
                        {
                            self.page = Page::About;
                        }
                        if ui.button("?").on_hover_text("How to play · F1").clicked() {
                            self.page = Page::Help;
                        }
                        if ui
                            .button("Open")
                            .on_hover_text("Open a collection · O / Ctrl+O")
                            .clicked()
                        {
                            self.page = Page::Collections;
                        }
                        if ui
                            .selectable_label(self.page == Page::Puzzles, "Puzzles  ⇥")
                            .on_hover_text("Show or hide the puzzle list · Tab")
                            .clicked()
                        {
                            self.toggle_puzzles();
                        }
                        if !compact {
                            ui.label(RichText::new("●").color(t.accent))
                                .on_hover_text(format!("Live Omarchy theme: {}", t.name));
                        }
                    });
                });
            });
        egui::TopBottomPanel::bottom("shortcuts")
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(12, 2)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().button_padding = Vec2::new(3.0, 1.0);
                ui.spacing_mut().interact_size.y = 14.0;
                let shortcuts = if self.page == Page::Game {
                    "F1 Help   F2 About   O Open   Tab Puzzles   [ ] Levels"
                } else if self.page == Page::Puzzles {
                    "F1 Help   ↑↓ / JK Browse   Enter Play   Tab / Esc Close"
                } else if self.page == Page::ResetAll {
                    "Y Reset ALL   N / Esc Cancel   Tab / Shift+Tab Focus   Enter / Space Select"
                } else if self.page == Page::Restart {
                    "Y Yes   N / Esc No   Tab / Shift+Tab Focus   Enter / Space Select"
                } else {
                    "F1 Help   F2 About   Ctrl+O Open   Tab Focus   Esc Back"
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(shortcuts).size(10.0).color(t.muted));
                    if !matches!(self.page, Page::ResetAll | Page::Restart)
                        && ui
                            .button(RichText::new("Reset all · Ctrl+Shift+R").size(10.0))
                            .clicked()
                    {
                        self.open_reset_all();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(if self.save_path.is_none() {
                                "TEMPORARY"
                            } else if self.error.is_some() {
                                "SAVE ERROR"
                            } else {
                                "AUTOSAVED"
                            })
                            .size(9.0)
                            .color(t.muted),
                        );
                    });
                });
            });
        if !matches!(self.page, Page::Game | Page::Puzzles | Page::Restart) {
            self.board_rect = Rect::NOTHING;
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(t.background)
                        .inner_margin(egui::Margin::same(16)),
                )
                .show(ctx, |ui| self.content_page(ui));
            return;
        }
        egui::TopBottomPanel::top("puzzle_header")
            .frame(
                egui::Frame::new()
                    .fill(t.background)
                    .inner_margin(egui::Margin::symmetric(14, 7)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:02}/{:02}", self.selected + 1, self.levels.len()))
                            .size(12.0)
                            .color(t.muted),
                    );
                    let title_width =
                        (ui.available_width() - if compact { 230.0 } else { 295.0 }).max(100.0);
                    ui.add_sized(
                        [title_width, 24.0],
                        egui::Label::new(
                            RichText::new(&self.game.level.name)
                                .size(if compact { 16.0 } else { 19.0 })
                                .strong(),
                        )
                        .truncate(),
                    )
                    .on_hover_text(format!(
                        "{} · {}",
                        self.collection,
                        self.chapter(self.selected)
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{}/{} goals",
                                self.game.placed(),
                                self.game.crates.len()
                            ))
                            .size(12.0)
                            .color(t.green),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new(format!("{} pushes", self.game.pushes))
                                .size(12.0)
                                .color(t.accent),
                        );
                        ui.separator();
                        ui.label(RichText::new(format!("{} moves", self.game.moves)).size(12.0));
                    });
                });
            });
        egui::TopBottomPanel::bottom("controls")
            .frame(
                egui::Frame::new()
                    .fill(t.panel)
                    .inner_margin(egui::Margin::symmetric(12, 7)),
            )
            .show(ctx, |ui| {
                if self.page == Page::Restart {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.game.can_undo(), egui::Button::new("Undo  U"))
                        .clicked()
                    {
                        self.undo();
                    }
                    if ui
                        .add_enabled(self.game.can_redo(), egui::Button::new("Redo  Y"))
                        .clicked()
                    {
                        self.redo();
                    }
                    if ui.button("Restart  R").clicked() && self.game.moves > 0 {
                        self.open_restart();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for (label, d) in [
                            ("→", Direction::Right),
                            ("↓", Direction::Down),
                            ("↑", Direction::Up),
                            ("←", Direction::Left),
                        ] {
                            if ui
                                .add_sized([30.0, 28.0], egui::Button::new(label))
                                .on_hover_text("Move: arrows / WASD / HJKL")
                                .clicked()
                            {
                                self.move_player(d);
                            }
                        }
                        ui.label(RichText::new("WASD / HJKL").size(9.0).color(t.muted));
                    });
                });
                if self.game.solved() || self.game.corner_deadlock() {
                    ui.horizontal(|ui| {
                        if self.game.solved() {
                            let all_solved = self.levels.iter().all(|l| {
                                self.progress
                                    .records
                                    .get(&l.id)
                                    .is_some_and(|r| r.best.is_some())
                            });
                            ui.label(
                                RichText::new(if all_solved {
                                    "✓ Collection complete!"
                                } else {
                                    "✓ Everything in its place."
                                })
                                .size(12.0)
                                .color(t.green),
                            );
                            if self.selected + 1 < self.levels.len()
                                && ui.small_button("Next puzzle  ↵").clicked()
                            {
                                self.select(self.selected + 1);
                            } else if self.selected + 1 == self.levels.len()
                                && ui.small_button("All puzzles").clicked()
                            {
                                self.toggle_puzzles();
                            }
                        } else if self.game.corner_deadlock() {
                            ui.label(
                                RichText::new("Library in a corner. U to undo.")
                                    .size(11.0)
                                    .color(t.yellow),
                            );
                        }
                    });
                }
                if let Some(error) = &self.error {
                    ui.colored_label(t.red, error);
                }
            });
        if self.page == Page::Puzzles {
            egui::SidePanel::left("puzzle_sidebar")
                .resizable(false)
                .exact_width(if compact { 200.0 } else { 260.0 })
                .frame(
                    egui::Frame::new()
                        .fill(t.panel)
                        .inner_margin(egui::Margin::same(10)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Puzzles");
                        if ui.small_button("Close · Tab").clicked() {
                            self.page = Page::Game;
                        }
                    });
                    ui.small("↑ ↓ / J K · Enter to play");
                    egui::ScrollArea::vertical()
                        .id_salt("puzzle_sidebar_scroll")
                        .show(ui, |ui| self.level_list(ui));
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(t.background)
                    .inner_margin(egui::Margin::same(4)),
            )
            .show(ctx, |ui| {
                if self.page == Page::Restart {
                    ui.disable();
                }
                self.board(ui);
            });
        if self.page == Page::Restart {
            self.restart_confirmation(ctx);
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.draw(ctx);
    }
}

#[cfg(test)]
mod ui_tests {
    use super::*;
    use eframe::egui::Pos2;

    fn app(ctx: &egui::Context) -> App {
        let levels = parse_collection("; Window test\n########\n#      #\n# . $  #\n#   #  #\n# $ .  #\n#   @  #\n#      #\n########\n").unwrap();
        App::new(
            ctx,
            Options {
                levels: None,
                theme: None,
                fresh: true,
            },
            levels,
        )
    }
    fn frame(
        app: &mut App,
        ctx: &egui::Context,
        width: f32,
        height: f32,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, height))),
            events,
            ..Default::default()
        };
        ctx.run(input, |ctx| app.draw(ctx))
    }
    #[test]
    fn builtin_campaign_chapters_match_metadata() {
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        app.campaign = false;
        app.load_builtin();
        let metadata: serde_json::Value =
            serde_json::from_str(include_str!("../assets/campaign.json")).unwrap();
        assert!(app.campaign);
        assert_eq!(app.collection, metadata["name"].as_str().unwrap());
        assert_eq!(app.levels.len(), campaign::LEVEL_COUNT);
        for (i, level) in metadata["levels"].as_array().unwrap().iter().enumerate() {
            assert_eq!(app.chapter(i), level["chapter"].as_str().unwrap());
        }
        app.campaign = false;
        assert_eq!(app.chapter(0), "");
    }

    #[test]
    fn community_collections_switch_without_losing_campaign_progress() {
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        app.load_builtin();
        app.move_player(Direction::Right);
        assert!(app.game.solved());
        for (index, collection) in COLLECTIONS.iter().enumerate() {
            app.load_collection(index);
            assert!(!app.campaign);
            assert_eq!(app.collection, collection.name);
            assert_eq!(app.levels.len(), collection.count);
            assert_eq!(app.chapter(0), "");
        }
        app.load_builtin();
        app.select(0);
        assert!(app.game.solved());
        assert_eq!((app.game.moves, app.game.pushes), (1, 1));
        assert!(app.game.can_undo());
    }

    fn app_with_scores(ctx: &egui::Context) -> App {
        let mut app = app(ctx);
        app.move_player(Direction::Right); // An imported collection's saved path.
        app.load_builtin();
        app.move_player(Direction::Right); // Completed campaign puzzle and best score.
        app.load_collection(0);
        app.move_player(Direction::Up);
        app.load_builtin();
        app.select(1);
        app.page = Page::ResetAll;
        app
    }

    #[test]
    fn reset_all_clears_every_collection_and_persists_empty_scores() {
        let ctx = egui::Context::default();
        let mut app = app_with_scores(&ctx);
        assert!(app.progress.records.len() >= 3);
        assert!(
            app.progress
                .records
                .values()
                .any(|record| record.best.is_some())
        );
        let directory =
            std::env::temp_dir().join(format!("sokobanchy-reset-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("progress.json");
        app.progress.save(&path).unwrap();
        app.save_path = Some(path.clone());
        app.reset_all_progress();
        assert_eq!(app.page, Page::Game);
        assert_eq!(app.selected, 0);
        assert_eq!((app.game.moves, app.game.pushes), (0, 0));
        assert!(!app.game.can_undo() && !app.game.can_redo());
        assert!(app.progress.records.is_empty());
        let saved = Progress::load(&path).unwrap();
        assert!(saved.records.is_empty());
        assert_eq!(saved.current, app.levels[0].id);
        app.load_collection(0);
        assert_eq!(app.game.moves, 0);
        assert!(
            app.progress
                .records
                .values()
                .all(|record| record.best.is_none())
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn reset_confirmation_keeps_progress_until_explicit_action() {
        let ctx = egui::Context::default();
        let mut app = app_with_scores(&ctx);
        let before = serde_json::to_value(&app.progress).unwrap();
        frame(&mut app, &ctx, 640.0, 480.0, vec![key(Key::Enter)]);
        assert_eq!(app.page, Page::ResetAll);
        assert_eq!(serde_json::to_value(&app.progress).unwrap(), before);
        frame(&mut app, &ctx, 640.0, 480.0, vec![key(Key::Escape)]);
        assert_eq!(app.page, Page::Game);
        assert_eq!(serde_json::to_value(&app.progress).unwrap(), before);
        // Fresh sessions reset only their in-memory state.
        assert!(app.save_path.is_none());
        app.error = Some("An unrelated collection import failed".into());
        app.reset_all_progress();
        assert!(app.progress.records.is_empty());
    }

    #[test]
    fn restart_only_affects_current_puzzle_and_keeps_scores() {
        let ctx = egui::Context::default();
        let mut app = app_with_scores(&ctx);
        app.select(0);
        app.page = Page::Game;
        assert!(app.game.solved());
        let id = app.game.level.id.clone();
        let before = serde_json::to_value(&app.progress).unwrap();
        frame(&mut app, &ctx, 640.0, 480.0, vec![key(Key::R)]);
        assert_eq!(app.page, Page::Restart);
        frame(&mut app, &ctx, 640.0, 480.0, vec![key(Key::Enter)]);
        assert_eq!(app.game.moves, 0);
        assert_eq!(app.progress.records[&id].best, Some((1, 1)));
        assert_eq!(
            app.progress.records.len(),
            before["records"].as_object().unwrap().len()
        );
        for (other_id, record) in &app.progress.records {
            if other_id != &id {
                assert_eq!(
                    serde_json::to_value(record).unwrap(),
                    before["records"][other_id]
                );
            }
        }
    }

    #[test]
    fn reset_all_is_accessible_and_confirmable_without_a_mouse() {
        for action in [Key::Escape, Key::N, Key::Enter, Key::Y, Key::Space] {
            let ctx = egui::Context::default();
            let mut app = app_with_scores(&ctx);
            app.page = Page::Game;
            let before = serde_json::to_value(&app.progress).unwrap();
            let mut shortcut = key(Key::R);
            if let egui::Event::Key { modifiers, .. } = &mut shortcut {
                *modifiers = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
            }
            frame(&mut app, &ctx, 600.0, 460.0, vec![shortcut]);
            assert_eq!(app.page, Page::ResetAll);
            assert_eq!(serde_json::to_value(&app.progress).unwrap(), before);
            // Default focus is Cancel. Tab selects the explicit reset button.
            if action == Key::Space {
                frame(&mut app, &ctx, 600.0, 460.0, vec![key(Key::Tab)]);
            }
            frame(&mut app, &ctx, 600.0, 460.0, vec![key(action)]);
            assert_eq!(app.page, Page::Game, "action {action:?}");
            if matches!(action, Key::Y | Key::Space) {
                assert!(app.progress.records.is_empty(), "action {action:?}");
                assert_eq!(app.selected, 0);
            } else {
                assert_eq!(serde_json::to_value(&app.progress).unwrap(), before);
            }
        }
    }

    #[test]
    fn failed_reset_preserves_existing_progress_and_game() {
        let ctx = egui::Context::default();
        let mut app = app_with_scores(&ctx);
        let before = serde_json::to_value(&app.progress).unwrap();
        let selected = app.selected;
        let blocker =
            std::env::temp_dir().join(format!("sokobanchy-reset-blocker-{}", std::process::id()));
        std::fs::write(&blocker, b"keep this file").unwrap();
        app.save_path = Some(blocker.join("progress.json"));
        app.reset_all_progress();
        assert_eq!(serde_json::to_value(&app.progress).unwrap(), before);
        assert_eq!(app.selected, selected);
        assert_eq!(app.page, Page::ResetAll);
        assert!(
            app.error
                .as_ref()
                .unwrap()
                .contains("Your progress was kept")
        );
        assert_eq!(std::fs::read(&blocker).unwrap(), b"keep this file");
        std::fs::remove_file(blocker).unwrap();
        app.save_path = None;
        app.fresh = false;
        // Clearing an unrelated error must not re-enable an unreadable save.
        app.error = None;
        app.reset_all_progress();
        assert_eq!(serde_json::to_value(&app.progress).unwrap(), before);
        assert!(
            app.error
                .as_ref()
                .unwrap()
                .contains("protect an unreadable")
        );
    }

    #[test]
    fn delivery_smile_expires_but_completion_smile_persists() {
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        app.levels = parse_collection(include_str!("../assets/levels.xsb")).unwrap();
        app.select(2); // Two deliveries: one push, a walk around, then another push.
        assert!(!app.smiling_at(Instant::now()));
        app.move_player(Direction::Up);
        assert!(app.smiling_at(Instant::now()));
        assert!(!app.smiling_at(Instant::now() + Duration::from_secs(2)));
        let deadline = app.smile_until;
        app.move_player(Direction::Down);
        assert_eq!(
            app.smile_until, deadline,
            "walking must not renew the smile"
        );
        app.undo();
        app.undo();
        assert!(!app.smiling_at(Instant::now()));
        app.redo();
        assert!(
            app.smiling_at(Instant::now()),
            "redoing a delivery should smile"
        );
        for c in "drrru".chars() {
            app.move_player(Direction::from_char(c).unwrap());
        }
        assert!(app.game.solved());
        assert!(app.smiling_at(Instant::now() + Duration::from_secs(3600)));
        app.undo();
        assert!(!app.game.solved());
        assert!(!app.smiling_at(Instant::now()));
        app.redo();
        app.select(0);
        assert!(
            !app.smiling_at(Instant::now()),
            "new puzzle must not inherit a smile"
        );
        app.select(2);
        assert!(
            app.smiling_at(Instant::now()),
            "resumed completed puzzles keep smiling"
        );
        app.restart_now();
        assert!(!app.smiling_at(Instant::now()));
    }

    #[test]
    fn pushing_between_goals_also_triggers_a_smile() {
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        app.game = Game::new(
            Level::parse(
                "Adjacent goals",
                &["########", "#      #", "# @*.$ #", "#      #", "########"],
            )
            .unwrap(),
        );
        let placed = app.game.placed();
        app.move_player(Direction::Up);
        assert!(!app.smiling_at(Instant::now()), "walking is not a delivery");
        app.move_player(Direction::Down);
        app.move_player(Direction::Right);
        assert_eq!(app.game.placed(), placed);
        assert!(app.smiling_at(Instant::now()));
        let deadline = app.smile_until;
        app.move_player(Direction::Right); // Blocked by the second library.
        assert_eq!(app.smile_until, deadline);
    }

    #[test]
    fn board_remains_large_in_small_windows() {
        for (width, height, minimum_tile) in [
            (600.0, 460.0, 36.0),
            (640.0, 480.0, 38.0),
            (800.0, 600.0, 53.0),
            (880.0, 680.0, 63.0),
        ] {
            let ctx = egui::Context::default();
            let mut app = app(&ctx);
            for _ in 0..3 {
                frame(&mut app, &ctx, width, height, vec![]);
            }
            assert!(
                app.tile_size >= minimum_tile,
                "{width}×{height}: tile = {}",
                app.tile_size
            );
            assert!(app.board_rect.top() >= 0.0 && app.board_rect.bottom() <= height);
            assert!(app.board_rect.left() >= 0.0 && app.board_rect.right() <= width);
            assert!(
                app.board_rect.height() >= height * 0.62,
                "{width}×{height}: board {} tall, tile {}",
                app.board_rect.height(),
                app.tile_size
            );
        }
    }
    fn key(key: Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn pages_use_the_main_body_and_preserve_gameplay() {
        for page in [Page::About, Page::Help, Page::Collections, Page::ResetAll] {
            for (width, height) in [(600.0, 460.0), (1100.0, 760.0)] {
                for light in [false, true] {
                    let ctx = egui::Context::default();
                    let mut app = app(&ctx);
                    if light {
                        app.theme = Theme::parse(
                            "background = '#eff1f5'\nforeground = '#4c4f69'\nmode = 'light'",
                            "Light test".into(),
                        )
                        .unwrap();
                        app.theme.apply(&ctx);
                    }
                    app.move_player(Direction::Right);
                    let (player, moves) = (app.game.player, app.game.moves);
                    app.page = page;
                    for _ in 0..3 {
                        frame(&mut app, &ctx, width, height, vec![]);
                    }
                    let output = frame(&mut app, &ctx, width, height, vec![key(Key::ArrowLeft)]);
                    assert_eq!(app.page, page);
                    assert_eq!((app.game.player, app.game.moves), (player, moves));
                    assert_eq!(
                        app.board_rect,
                        Rect::NOTHING,
                        "hidden board must not remain interactive"
                    );
                    let rect = ctx.used_rect();
                    assert!(
                        rect.left() >= 0.0 && rect.right() <= width,
                        "{page:?}: {rect:?}"
                    );
                    assert!(
                        rect.top() >= 0.0 && rect.bottom() <= height,
                        "{page:?}: {rect:?}"
                    );
                    let texts: Vec<_> = output
                        .shapes
                        .iter()
                        .filter_map(|shape| {
                            if let egui::Shape::Text(text) = &shape.shape {
                                Some(text.galley.text())
                            } else {
                                None
                            }
                        })
                        .collect();
                    assert!(
                        texts.contains(&page.title()),
                        "missing page content: {page:?}"
                    );
                    assert!(texts.contains(&"Back to puzzle · Esc"));
                    let hint = match page {
                        Page::ResetAll => "Y Reset ALL",
                        _ => "F1 Help   F2 About",
                    };
                    assert!(texts.iter().any(|text| text.starts_with(hint)));
                    assert!(
                        !texts.contains(&"Undo  U"),
                        "game controls must not appear on pages"
                    );
                    if page == Page::Help {
                        assert!(texts.contains(&"A little room. A little patience."));
                    }
                    frame(&mut app, &ctx, width, height, vec![key(Key::Escape)]);
                    assert_eq!(app.page, Page::Game);
                    assert_eq!((app.game.player, app.game.moves), (player, moves));
                    assert!(app.board_rect.is_positive());
                }
            }
        }
    }

    #[test]
    fn typing_collection_paths_does_not_trigger_navigation() {
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        app.page = Page::Collections;
        frame(&mut app, &ctx, 600.0, 460.0, vec![]);
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("collection_path")));
        frame(
            &mut app,
            &ctx,
            600.0,
            460.0,
            vec![key(Key::O), egui::Event::Text("/tmp/sokoban.xsb".into())],
        );
        assert_eq!(app.page, Page::Collections);
        assert_eq!(app.import_path, "/tmp/sokoban.xsb");
        frame(&mut app, &ctx, 600.0, 460.0, vec![key(Key::Escape)]);
        assert_eq!(app.page, Page::Game);
        assert_eq!(app.game.moves, 0);
    }

    #[test]
    fn sidebar_keyboard_browses_without_moving_the_player() {
        for (width, height) in [(600.0, 460.0), (1100.0, 760.0)] {
            let ctx = egui::Context::default();
            let mut app = app(&ctx);
            app.load_builtin();
            let player = app.game.player;
            frame(&mut app, &ctx, width, height, vec![key(Key::Tab)]);
            assert_eq!(app.page, Page::Puzzles);
            assert!(app.board_rect.left() >= 200.0);
            assert!(app.board_rect.right() <= width);
            assert!(app.board_rect.is_positive());
            for (pressed, expected) in [
                (Key::ArrowDown, 1),
                (Key::J, 2),
                (Key::K, 1),
                (Key::PageDown, 11),
                (Key::End, 34),
                (Key::Home, 0),
                (Key::ArrowUp, 0),
            ] {
                frame(&mut app, &ctx, width, height, vec![key(pressed)]);
                assert_eq!(app.puzzle_cursor, expected);
                assert_eq!((app.game.player, app.game.moves), (player, 0));
                assert_eq!(app.selected, 0);
            }
            frame(&mut app, &ctx, width, height, vec![key(Key::ArrowDown)]);
            frame(&mut app, &ctx, width, height, vec![key(Key::Enter)]);
            assert_eq!(app.page, Page::Game);
            assert_eq!(app.selected, 1);
            assert_eq!(app.game.moves, 0);
        }
    }

    #[test]
    fn keyboard_undo_redo_and_restart_preserve_confirmation() {
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        let modified = |key_code, shift| {
            let mut event = key(key_code);
            if let egui::Event::Key { modifiers, .. } = &mut event {
                modifiers.ctrl = true;
                modifiers.shift = shift;
            }
            event
        };
        frame(&mut app, &ctx, 640.0, 480.0, vec![key(Key::ArrowRight)]);
        assert_eq!(app.game.moves, 1);
        frame(&mut app, &ctx, 640.0, 480.0, vec![modified(Key::Z, false)]);
        assert_eq!(app.game.moves, 0);
        frame(&mut app, &ctx, 640.0, 480.0, vec![modified(Key::Z, true)]);
        assert_eq!(app.game.moves, 1);
        frame(&mut app, &ctx, 640.0, 480.0, vec![key(Key::U)]);
        frame(&mut app, &ctx, 640.0, 480.0, vec![modified(Key::Y, false)]);
        assert_eq!(app.game.moves, 1);
        frame(&mut app, &ctx, 640.0, 480.0, vec![key(Key::R)]);
        assert_eq!(app.page, Page::Restart);
        assert_eq!(app.game.moves, 1);
        frame(&mut app, &ctx, 640.0, 480.0, vec![key(Key::Escape)]);
        assert_eq!(app.game.moves, 1);
        let mut release = key(Key::R);
        if let egui::Event::Key { pressed, .. } = &mut release {
            *pressed = false;
        }
        frame(&mut app, &ctx, 640.0, 480.0, vec![release, key(Key::R)]);
        assert_eq!(app.page, Page::Restart);
        frame(&mut app, &ctx, 640.0, 480.0, vec![key(Key::Enter)]);
        assert_eq!(app.page, Page::Game);
        assert_eq!(app.game.moves, 0);
    }

    #[test]
    fn restart_floats_over_board_and_supports_yes_no_keyboard_controls() {
        for (width, height) in [(600.0, 460.0), (1100.0, 760.0)] {
            for action in [Key::Y, Key::N, Key::Escape, Key::Enter, Key::Space] {
                let ctx = egui::Context::default();
                let mut app = app(&ctx);
                app.move_player(Direction::Right);
                let player = app.game.player;
                frame(&mut app, &ctx, width, height, vec![key(Key::R)]);
                for _ in 0..3 {
                    frame(&mut app, &ctx, width, height, vec![]);
                }
                let modal = ctx
                    .memory(|m| m.area_rect(egui::Id::new("restart_confirmation")))
                    .unwrap();
                assert!(app.board_rect.is_positive(), "board stays visible");
                assert!(modal.center().distance(app.board_rect.center()) < 1.0);
                assert!(modal.width() < width && modal.height() < height / 2.0);
                frame(&mut app, &ctx, width, height, vec![key(Key::W)]);
                assert_eq!((app.game.player, app.game.moves), (player, 1));
                // Tab moves from Yes to No; Space activates No.
                if action == Key::Space {
                    frame(&mut app, &ctx, width, height, vec![key(Key::Tab)]);
                }
                frame(&mut app, &ctx, width, height, vec![key(action)]);
                assert_eq!(app.page, Page::Game, "{action:?}");
                let confirmed = matches!(action, Key::Y | Key::Enter);
                assert_eq!(app.game.moves, if confirmed { 0 } else { 1 }, "{action:?}");
            }
        }
    }

    #[test]
    fn help_shortcut_stays_open_on_repeated_presses() {
        for page in [
            Page::Game,
            Page::Puzzles,
            Page::Collections,
            Page::About,
            Page::Restart,
        ] {
            let ctx = egui::Context::default();
            let mut app = app(&ctx);
            app.page = page;
            frame(&mut app, &ctx, 600.0, 460.0, vec![]);
            if page == Page::Collections {
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("collection_path")));
            }
            for _ in 0..3 {
                frame(&mut app, &ctx, 600.0, 460.0, vec![key(Key::F1)]);
                assert_eq!(app.page, Page::Help);
            }
            assert_eq!(app.game.moves, 0);
            frame(&mut app, &ctx, 600.0, 460.0, vec![key(Key::Escape)]);
            assert_eq!(app.page, Page::Game);
        }
    }

    #[test]
    fn navigation_replaces_pages_and_returns_to_full_size_board() {
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        for _ in 0..3 {
            frame(&mut app, &ctx, 640.0, 480.0, vec![]);
        }
        let normal = app.tile_size;
        for (pressed, expected) in [
            (Key::Tab, Page::Puzzles),
            (Key::F1, Page::Help),
            (Key::Tab, Page::Help),
            (Key::F2, Page::About),
            (Key::O, Page::Collections),
            (Key::Escape, Page::Game),
        ] {
            frame(&mut app, &ctx, 640.0, 480.0, vec![key(pressed)]);
            assert_eq!(app.page, expected);
        }
        assert!((app.tile_size - normal).abs() < 0.1);
        assert_eq!(app.game.moves, 0);
    }
}
