//! Offline deterministic puzzle generation and exact push-BFS. Reads no reference collections.
//! `cargo run --release --example campaign_lab -- generate` writes candidates to analysis/.
//! `cargo run --release --example campaign_lab -- verify FILE.xsb` checks selected boards.
use serde::Serialize;
use sokobanchy::game::{Direction, Game, Level, parse_collection};
use std::collections::{HashMap, VecDeque};

const CHARS: [char; 4] = ['u', 'd', 'l', 'r'];
const LIMIT: usize = 180_000;
fn shift(bits: u128, d: usize) -> u128 {
    match d {
        0 => bits >> 16,
        1 => bits << 16,
        2 => (bits & !0x00010001000100010001000100010001) >> 1,
        _ => (bits & !0x80008000800080008000800080008000) << 1,
    }
}
fn region(player: u128, free: u128) -> u128 {
    let mut r = player;
    loop {
        let n = r | ((shift(r, 0) | shift(r, 1) | shift(r, 2) | shift(r, 3)) & free);
        if n == r {
            return r;
        }
        r = n;
    }
}
fn bits(mut mask: u128) -> impl Iterator<Item = u128> {
    std::iter::from_fn(move || {
        if mask == 0 {
            None
        } else {
            let b = mask & mask.wrapping_neg();
            mask ^= b;
            Some(b)
        }
    })
}
fn key(boxes: u128, player: u128, floor: u128) -> (u128, u32) {
    (boxes, region(player, floor & !boxes).trailing_zeros())
}
#[derive(Clone, Copy)]
struct Board {
    width: usize,
    height: usize,
    floor: u128,
    goals: u128,
    boxes: u128,
    player: u128,
}
impl Board {
    fn rows(self) -> Vec<String> {
        (0..self.height)
            .map(|y| {
                (0..self.width)
                    .map(|x| {
                        let b = 1u128 << (y * 16 + x);
                        if self.player & b != 0 {
                            if self.goals & b != 0 { '+' } else { '@' }
                        } else if self.boxes & b != 0 {
                            if self.goals & b != 0 { '*' } else { '$' }
                        } else if self.goals & b != 0 {
                            '.'
                        } else if self.floor & b != 0 {
                            ' '
                        } else {
                            '#'
                        }
                    })
                    .collect()
            })
            .collect()
    }
    fn from_level(l: &Level) -> Self {
        assert!(l.width <= 16 && l.height <= 8, "lab boards must fit 16×8");
        let mut floor = 0;
        let mut goals = 0;
        for y in 0..l.height {
            for x in 0..l.width {
                let p = sokobanchy::game::Pos(x as i32, y as i32);
                if l.walkable(p) {
                    floor |= 1u128 << (y * 16 + x);
                }
                if l.tile(p) == sokobanchy::game::Tile::Goal {
                    goals |= 1u128 << (y * 16 + x);
                }
            }
        }
        Self {
            width: l.width,
            height: l.height,
            floor,
            goals,
            boxes: l
                .crates
                .iter()
                .fold(0, |v, p| v | (1u128 << (p.1 * 16 + p.0))),
            player: 1u128 << (l.player.1 * 16 + l.player.0),
        }
    }
    fn distances(self) -> Vec<[u16; 128]> {
        bits(self.goals)
            .map(|g| {
                let mut dist = [u16::MAX; 128];
                dist[g.trailing_zeros() as usize] = 0;
                let mut todo = VecDeque::from([g]);
                while let Some(p) = todo.pop_front() {
                    for d in 0..4 {
                        let prev = shift(p, d);
                        let behind = shift(prev, d);
                        if prev & self.floor == 0 || behind & self.floor == 0 {
                            continue;
                        }
                        let idx = prev.trailing_zeros() as usize;
                        if dist[idx] == u16::MAX {
                            dist[idx] = dist[p.trailing_zeros() as usize] + 1;
                            todo.push_back(prev);
                        }
                    }
                }
                dist
            })
            .collect()
    }
    fn bound(self, distances: &[[u16; 128]]) -> u16 {
        let n = distances.len();
        let mut costs = vec![u16::MAX; 1 << n];
        costs[0] = 0;
        for b in bits(self.boxes) {
            let mut next = vec![u16::MAX; 1 << n];
            for (mask, &cost) in costs.iter().enumerate() {
                if cost == u16::MAX {
                    continue;
                }
                for (i, dist) in distances.iter().enumerate() {
                    if mask & (1 << i) == 0 {
                        let v = cost.saturating_add(dist[b.trailing_zeros() as usize]);
                        let m = mask | 1 << i;
                        next[m] = next[m].min(v);
                    }
                }
            }
            costs = next;
        }
        costs[(1 << n) - 1]
    }
}
#[derive(Clone, Copy)]
struct Node {
    boxes: u128,
    player: u128,
    parent: usize,
    box_from: u128,
    d: usize,
    depth: u16,
}
#[derive(Serialize)]
struct Solved {
    pushes: u16,
    lower_bound: u16,
    overhead: u16,
    states: usize,
    solution: String,
}
fn walk(player: u128, target: u128, free: u128) -> String {
    let mut prev = [None; 128];
    let mut queue = VecDeque::from([player]);
    let mut seen = player;
    while let Some(p) = queue.pop_front() {
        if p == target {
            break;
        }
        for d in 0..4 {
            let n = shift(p, d);
            if n & free != 0 && n & seen == 0 {
                seen |= n;
                prev[n.trailing_zeros() as usize] = Some((p, d));
                queue.push_back(n);
            }
        }
    }
    assert!(seen & target != 0);
    let mut path = Vec::new();
    let mut p = target;
    while p != player {
        let (before, d) = prev[p.trailing_zeros() as usize].unwrap();
        path.push(CHARS[d]);
        p = before;
    }
    path.into_iter().rev().collect()
}
fn solve(board: Board, limit: usize) -> Option<Solved> {
    let distances = board.distances();
    let bound = board.bound(&distances);
    let live = (0..128)
        .filter(|&i| distances.iter().any(|d| d[i] != u16::MAX))
        .fold(0, |v, i| v | 1u128 << i);
    let mut nodes = vec![Node {
        boxes: board.boxes,
        player: board.player,
        parent: 0,
        box_from: 0,
        d: 0,
        depth: 0,
    }];
    let mut seen = HashMap::from([(key(board.boxes, board.player, board.floor), 0usize)]);
    let mut cursor = 0;
    while cursor < nodes.len() {
        let node = nodes[cursor];
        if node.boxes == board.goals {
            let mut pushes = Vec::new();
            let mut i = cursor;
            while i != 0 {
                let n = nodes[i];
                pushes.push((n.box_from, n.d));
                i = n.parent;
            }
            let mut boxes = board.boxes;
            let mut player = board.player;
            let mut path = String::new();
            for (b, d) in pushes.into_iter().rev() {
                path.push_str(&walk(player, shift(b, d ^ 1), board.floor & !boxes));
                path.push(CHARS[d].to_ascii_uppercase());
                player = b;
                boxes ^= b | shift(b, d);
            }
            return Some(Solved {
                pushes: node.depth,
                lower_bound: bound,
                overhead: node.depth - bound,
                states: cursor + 1,
                solution: path,
            });
        }
        let reach = region(node.player, board.floor & !node.boxes);
        for b in bits(node.boxes) {
            for d in 0..4 {
                let dest = shift(b, d);
                if shift(b, d ^ 1) & reach == 0 || dest & live == 0 || dest & node.boxes != 0 {
                    continue;
                }
                let boxes = node.boxes ^ b ^ dest;
                let k = key(boxes, b, board.floor);
                if seen.contains_key(&k) {
                    continue;
                }
                if nodes.len() >= limit {
                    return None;
                }
                seen.insert(k, nodes.len());
                nodes.push(Node {
                    boxes,
                    player: b,
                    parent: cursor,
                    box_from: b,
                    d,
                    depth: node.depth + 1,
                });
            }
        }
        cursor += 1;
    }
    None
}
struct Rng(u64);
impl Rng {
    fn next(&mut self, n: usize) -> usize {
        let mut v = self.0;
        v ^= v << 13;
        v ^= v >> 7;
        v ^= v << 17;
        self.0 = v;
        (v as usize) % n
    }
}
#[derive(Serialize)]
struct Candidate {
    seed: usize,
    rows: Vec<String>,
    #[serde(flatten)]
    solved: Solved,
}
fn verify(rows: &[String], solution: &str, pushes: u16) {
    let refs: Vec<_> = rows.iter().map(String::as_str).collect();
    let level = Level::parse("candidate", &refs).unwrap();
    let mut game = Game::new(level);
    for c in solution.chars() {
        assert!(game.step(Direction::from_char(c).unwrap()));
    }
    assert!(game.solved());
    assert_eq!(game.pushes, pushes as usize);
}
fn generate() {
    let mut rng = Rng(2026101302);
    let mut found = Vec::new();
    let mut unique = std::collections::HashSet::new();
    for attempt in 0..5000 {
        let mut floor = 0u128;
        for y in 1..7 {
            for x in 1..7 {
                floor |= 1u128 << (y * 16 + x);
            }
        }
        for _ in 0..(3 + rng.next(7)) {
            floor &= !(1u128 << ((1 + rng.next(6)) * 16 + 1 + rng.next(6)));
        }
        if region(floor & floor.wrapping_neg(), floor) != floor {
            continue;
        }
        let cells: Vec<_> = bits(floor).collect();
        let count = if attempt % 3 == 0 { 4 } else { 3 };
        let mut goals = 0u128;
        while goals.count_ones() < count {
            goals |= cells[rng.next(cells.len())];
        }
        let free: Vec<_> = bits(floor & !goals).collect();
        let mut player = free[rng.next(free.len())];
        let mut boxes = goals;
        let mut candidates = Vec::new();
        let mut visited = std::collections::HashSet::new();
        for step in 0..160 {
            let free = floor & !boxes;
            let reach = region(player, free);
            let mut choices = Vec::new();
            for b in bits(boxes) {
                for d in 0..4 {
                    let dest = shift(b, d);
                    let next_player = shift(dest, d);
                    if dest & reach != 0 && next_player & free != 0 {
                        let next_boxes = boxes ^ b ^ dest;
                        let fresh = !visited.contains(&key(next_boxes, next_player, floor));
                        choices.push((next_boxes, next_player, fresh));
                    }
                }
            }
            if choices.is_empty() {
                break;
            }
            let fresh: Vec<_> = choices.iter().filter(|c| c.2).copied().collect();
            let choice = if fresh.is_empty() {
                choices[rng.next(choices.len())]
            } else {
                fresh[rng.next(fresh.len())]
            };
            boxes = choice.0;
            player = choice.1;
            visited.insert(key(boxes, player, floor));
            if step % 20 == 19 && (boxes & !goals).count_ones() >= 2 {
                candidates.push(Board {
                    width: 8,
                    height: 8,
                    floor,
                    goals,
                    boxes,
                    player,
                });
            }
        }
        // Take the most separated sampled position; evaluate real interaction overhead with BFS.
        let Some(board) = candidates
            .into_iter()
            .max_by_key(|b| b.bound(&b.distances()))
        else {
            continue;
        };
        if !unique.insert((floor, goals, board.boxes)) {
            continue;
        }
        let Some(solved) = solve(board, LIMIT) else {
            continue;
        };
        if solved.pushes < 5 || (solved.pushes >= 16 && solved.overhead < 2) {
            continue;
        }
        let rows = board.rows();
        verify(&rows, &solved.solution, solved.pushes);
        if solved.overhead >= 4 || (found.len() < 120 && solved.pushes < 16) {
            println!(
                "candidate {attempt}: {} crates / {} pushes / +{} overhead / {} states",
                count, solved.pushes, solved.overhead, solved.states
            );
            found.push(Candidate {
                seed: attempt,
                rows,
                solved,
            });
        }
        if found
            .iter()
            .filter(|c| c.solved.pushes >= 28 && c.solved.overhead >= 6)
            .count()
            >= 15
            && found.len() > 200
        {
            break;
        }
        if attempt % 500 == 0 {
            std::fs::write(
                "analysis/campaign-candidates.json",
                serde_json::to_string_pretty(&found).unwrap(),
            )
            .unwrap();
        }
    }
    std::fs::write(
        "analysis/campaign-candidates.json",
        serde_json::to_string_pretty(&found).unwrap(),
    )
    .unwrap();
    println!("saved {} candidates", found.len());
}
fn variations() {
    let mut rng = Rng(1310202603);
    // The shipped campaign contains our original prototypes and curated designs.
    // It is sufficient to explore new variants from a clean source checkout.
    let sources = parse_collection(include_str!("../assets/levels.xsb")).unwrap();
    let mut found = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for attempt in 0..3500 {
        let mut b = Board::from_level(&sources[rng.next(sources.len())]);
        // Add a new delivery or rearrange one of our own existing crates/goals.
        if b.boxes.count_ones() >= 4 {
            let boxes: Vec<_> = bits(b.boxes).collect();
            let goals: Vec<_> = bits(b.goals).collect();
            b.boxes ^= boxes[rng.next(boxes.len())];
            b.goals ^= goals[rng.next(goals.len())];
        }
        let free: Vec<_> = bits(b.floor & !b.boxes & !b.player).collect();
        b.boxes |= free[rng.next(free.len())];
        let targets: Vec<_> = bits(b.floor & !b.goals).collect();
        b.goals |= targets[rng.next(targets.len())];
        if !seen.insert((b.floor, b.goals, b.boxes, b.player)) {
            continue;
        }
        let Some(solved) = solve(b, LIMIT) else {
            continue;
        };
        if solved.pushes < 18 || solved.overhead < 4 {
            continue;
        }
        let rows = b.rows();
        verify(&rows, &solved.solution, solved.pushes);
        println!(
            "variation {attempt}: {} pushes / +{} overhead / {} states",
            solved.pushes, solved.overhead, solved.states
        );
        found.push(Candidate {
            seed: attempt,
            rows,
            solved,
        });
        if found.len() % 10 == 0 {
            std::fs::write(
                "analysis/campaign-variations.json",
                serde_json::to_string_pretty(&found).unwrap(),
            )
            .unwrap();
        }
    }
    std::fs::write(
        "analysis/campaign-variations.json",
        serde_json::to_string_pretty(&found).unwrap(),
    )
    .unwrap();
    println!("saved {} variations", found.len());
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    std::fs::create_dir_all("analysis").expect("create analysis output directory");
    match args.get(1).map(String::as_str) {
        Some("generate") => generate(),
        Some("variations") => variations(),
        Some("verify") => {
            let levels = parse_collection(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
            let mut records = Vec::new();
            for l in levels {
                let solved =
                    solve(Board::from_level(&l), 1_000_000).expect("verification exceeded budget");
                let mut game = Game::new(l.clone());
                for c in solved.solution.chars() {
                    assert!(game.step(Direction::from_char(c).unwrap()));
                }
                assert!(game.solved());
                println!(
                    "{}: {} pushes, lower bound {}, overhead {}, {} states",
                    l.name, solved.pushes, solved.lower_bound, solved.overhead, solved.states
                );
                records.push(serde_json::json!({"name":l.name,"id":l.id,"pushes":solved.pushes,"lower_bound":solved.lower_bound,"overhead":solved.overhead,"states":solved.states,"solution":solved.solution}));
            }
            std::fs::write(
                "analysis/campaign-verification.json",
                serde_json::to_string_pretty(&records).unwrap(),
            )
            .unwrap();
        }
        _ => eprintln!(
            "Usage: campaign_lab generate | variations | verify FILE.xsb (boards up to 16×8)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitboards_preserve_all_campaign_boards_including_wide_levels() {
        for level in parse_collection(include_str!("../assets/levels.xsb")).unwrap() {
            let rows = Board::from_level(&level).rows();
            let refs: Vec<_> = rows.iter().map(String::as_str).collect();
            assert_eq!(Level::parse(&level.name, &refs).unwrap().id, level.id);
        }
    }

    #[test]
    fn shifts_do_not_wrap_across_bitboard_edges() {
        assert_eq!(shift(1, 2), 0);
        assert_eq!(shift(1 << 16, 2), 0);
        assert_eq!(shift(1 << 15, 3), 0);
        assert_eq!(shift(1 << 127, 3), 0);
        assert_eq!(shift(1 << 7, 3), 1 << 8);
        assert_eq!(shift(1 << 8, 1), 1 << 24);
    }
}
