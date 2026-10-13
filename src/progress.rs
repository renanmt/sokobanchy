use crate::{
    game::{Direction, Game},
    theme::xdg,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io, path::Path};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub current: String,
    pub records: BTreeMap<String, Record>,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub best: Option<(usize, usize)>,
    pub path: Vec<Direction>,
}
impl Progress {
    pub fn load(path: &Path) -> Result<Self, String> {
        match fs::read(path) {
            Ok(data) => {
                serde_json::from_slice(&data).map_err(|e| format!("Could not read progress: {e}"))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("Could not read progress: {e}")),
        }
    }
    pub fn capture(&mut self, game: &Game) {
        self.current = game.level.id.clone();
        let record = self.records.entry(game.level.id.clone()).or_default();
        record.path = game.solution();
        let score = (game.moves, game.pushes);
        if game.solved() && record.best.is_none_or(|best| score < best) {
            record.best = Some(score);
        }
    }
    pub fn restore(&self, game: &mut Game) {
        if let Some(record) = self.records.get(&game.level.id) {
            if record.path.len() > 1_000_000 {
                return;
            }
            for &d in &record.path {
                if !game.step(d) {
                    *game = Game::new(game.level.clone());
                    break;
                }
            }
        }
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension(format!("{}.tmp", std::process::id()));
        let data = serde_json::to_vec_pretty(self)?;
        fs::write(&temp, data)?;
        fs::rename(temp, path)
    }
    pub fn path() -> std::path::PathBuf {
        xdg("XDG_STATE_HOME", ".local/state").join("sokobanchy/progress.json")
    }
}
