use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pos(pub i32, pub i32);
impl Pos {
    pub fn step(self, d: Direction) -> Self {
        let (x, y) = d.delta();
        Self(self.0 + x, self.1 + y)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
impl Direction {
    pub fn delta(self) -> (i32, i32) {
        match self {
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            'u' => Some(Self::Up),
            'd' => Some(Self::Down),
            'l' => Some(Self::Left),
            'r' => Some(Self::Right),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Void,
    Wall,
    Floor,
    Goal,
}
#[derive(Clone)]
pub struct Level {
    pub id: String,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>,
    pub player: Pos,
    pub crates: BTreeSet<Pos>,
}
impl Level {
    pub fn parse(name: &str, rows: &[&str]) -> Result<Self, String> {
        let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let height = rows.len();
        if width < 3 || height < 3 || width > 128 || height > 128 {
            return Err("Board must be between 3×3 and 128×128".into());
        }
        let mut tiles = vec![Tile::Void; width * height];
        let mut crates = BTreeSet::new();
        let mut players = Vec::new();
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                let pos = Pos(x as i32, y as i32);
                tiles[y * width + x] = match c {
                    '#' => Tile::Wall,
                    '.' | '+' | '*' => Tile::Goal,
                    ' ' | '-' | '_' | '@' | '$' => Tile::Floor,
                    _ => return Err(format!("Unknown board character: {c}")),
                };
                if c == '@' || c == '+' {
                    players.push(pos);
                }
                if c == '$' || c == '*' {
                    crates.insert(pos);
                }
            }
        }
        if players.len() != 1 {
            return Err("A board needs exactly one player".into());
        }
        let goals = tiles.iter().filter(|&&t| t == Tile::Goal).count();
        if crates.is_empty() || crates.len() != goals {
            return Err("A board needs an equal, nonzero number of crates and goals".into());
        }
        // Flood exterior padding so ragged XSB boards render without an enclosing rectangle.
        let mut outside = BTreeSet::new();
        let mut queue = VecDeque::new();
        for y in 0..height {
            for x in 0..width {
                if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    queue.push_back(Pos(x as i32, y as i32));
                }
            }
        }
        while let Some(p) = queue.pop_front() {
            if p.0 < 0
                || p.1 < 0
                || p.0 >= width as i32
                || p.1 >= height as i32
                || outside.contains(&p)
            {
                continue;
            }
            let i = p.1 as usize * width + p.0 as usize;
            if tiles[i] == Tile::Wall {
                continue;
            }
            if tiles[i] == Tile::Goal || crates.contains(&p) || players.contains(&p) {
                return Err("The board must be enclosed by walls".into());
            }
            outside.insert(p);
            tiles[i] = Tile::Void;
            for d in [
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                queue.push_back(p.step(d));
            }
        }
        // A stable content fingerprint keeps progress separate for imported collections.
        let mut hash = 0xcbf29ce484222325u64;
        for b in rows.join("\n").bytes() {
            hash = (hash ^ b as u64).wrapping_mul(0x100000001b3);
        }
        Ok(Self {
            id: format!("{hash:016x}"),
            name: name.into(),
            width,
            height,
            tiles,
            player: players[0],
            crates,
        })
    }
    pub fn tile(&self, p: Pos) -> Tile {
        if p.0 < 0 || p.1 < 0 || p.0 >= self.width as i32 || p.1 >= self.height as i32 {
            Tile::Void
        } else {
            self.tiles[p.1 as usize * self.width + p.0 as usize]
        }
    }
    pub fn walkable(&self, p: Pos) -> bool {
        matches!(self.tile(p), Tile::Floor | Tile::Goal)
    }
}

/// Read UTF-8 or the Windows-1252 text used by older YASC collections.
pub fn parse_collection_bytes(data: &[u8]) -> Result<Vec<Level>, String> {
    if data.len() > 8 * 1024 * 1024 {
        return Err("Collection is larger than 8 MiB".into());
    }
    let text = match std::str::from_utf8(data) {
        Ok(text) => std::borrow::Cow::Borrowed(text),
        Err(_) => {
            const EXTENDED: [char; 32] = [
                '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}',
                'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›',
                'œ', '\u{9d}', 'ž', 'Ÿ',
            ];
            std::borrow::Cow::Owned(
                data.iter()
                    .map(|&b| {
                        if (0x80..=0x9f).contains(&b) {
                            EXTENDED[(b - 0x80) as usize]
                        } else {
                            char::from(b)
                        }
                    })
                    .collect::<String>(),
            )
        }
    };
    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err("Collection must be a text file".into());
    }
    parse_collection(text.trim_start_matches('\u{feff}'))
}

pub fn parse_collection(text: &str) -> Result<Vec<Level>, String> {
    let mut levels = Vec::new();
    let mut rows = Vec::new();
    let mut title = String::new();
    let finish =
        |rows: &mut Vec<&str>, title: &str, levels: &mut Vec<Level>| -> Result<(), String> {
            if !rows.is_empty() {
                let name = if title.is_empty() {
                    format!("Puzzle {:02}", levels.len() + 1)
                } else {
                    title.to_string()
                };
                levels.push(Level::parse(&name, rows).map_err(|e| format!("{name}: {e}"))?);
                rows.clear();
            }
            Ok(())
        };
    for line in text.lines().chain(std::iter::once("")) {
        let line = line.trim_end_matches('\r');
        let board = !line.trim().is_empty() && line.chars().all(|c| " #.@+$*-_".contains(c));
        if board {
            rows.push(line);
        } else {
            let had_board = !rows.is_empty();
            finish(&mut rows, &title, &mut levels)?;
            if had_board {
                title.clear();
            }
            let trimmed = line.trim().trim_start_matches(';').trim();
            if !trimmed.is_empty()
                && !line.starts_with("::")
                && !trimmed.chars().all(|c| "udlrUDLR".contains(c))
            {
                title = trimmed.into();
            }
        }
    }
    if levels.is_empty() {
        Err("No XSB/Sokoban boards found".into())
    } else {
        Ok(levels)
    }
}

#[derive(Clone)]
struct Move {
    from: Pos,
    direction: Direction,
    pushed: bool,
}
pub struct Game {
    pub level: Level,
    pub player: Pos,
    pub crates: BTreeSet<Pos>,
    pub moves: usize,
    pub pushes: usize,
    history: Vec<Move>,
    future: Vec<Direction>,
}
impl Game {
    pub fn new(level: Level) -> Self {
        Self {
            player: level.player,
            crates: level.crates.clone(),
            level,
            moves: 0,
            pushes: 0,
            history: vec![],
            future: vec![],
        }
    }
    pub fn step(&mut self, d: Direction) -> bool {
        let next = self.player.step(d);
        if !self.level.walkable(next) {
            return false;
        }
        let pushed = self.crates.contains(&next);
        if pushed {
            let beyond = next.step(d);
            if !self.level.walkable(beyond) || self.crates.contains(&beyond) {
                return false;
            }
            self.crates.remove(&next);
            self.crates.insert(beyond);
            self.pushes += 1;
        }
        self.history.push(Move {
            from: self.player,
            direction: d,
            pushed,
        });
        self.future.clear();
        self.player = next;
        self.moves += 1;
        true
    }
    pub fn undo(&mut self) -> bool {
        let Some(m) = self.history.pop() else {
            return false;
        };
        if m.pushed {
            self.crates.remove(&self.player.step(m.direction));
            self.crates.insert(self.player);
            self.pushes -= 1;
        }
        self.player = m.from;
        self.moves -= 1;
        self.future.push(m.direction);
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(d) = self.future.pop() else {
            return false;
        };
        let future = self.future.clone();
        let changed = self.step(d);
        self.future = future;
        changed
    }
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
    pub fn solution(&self) -> Vec<Direction> {
        self.history.iter().map(|m| m.direction).collect()
    }
    pub fn solved(&self) -> bool {
        self.crates
            .iter()
            .all(|&p| self.level.tile(p) == Tile::Goal)
    }
    pub fn placed(&self) -> usize {
        self.crates
            .iter()
            .filter(|&&p| self.level.tile(p) == Tile::Goal)
            .count()
    }
    pub fn corner_deadlock(&self) -> bool {
        self.crates.iter().any(|&p| {
            self.level.tile(p) != Tile::Goal
                && (!self.level.walkable(p.step(Direction::Up))
                    || !self.level.walkable(p.step(Direction::Down)))
                && (!self.level.walkable(p.step(Direction::Left))
                    || !self.level.walkable(p.step(Direction::Right)))
        })
    }
}
