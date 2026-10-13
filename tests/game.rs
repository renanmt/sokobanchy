use sokobanchy::{game::*, progress::Progress, theme::Theme};

#[test]
fn every_bundled_level_has_a_valid_solution() {
    let levels = parse_collection(include_str!("../assets/levels.xsb")).unwrap();
    let solutions: Vec<String> = serde_json::from_str(include_str!("solutions.json")).unwrap();
    assert_eq!(levels.len(), 35);
    assert_eq!(levels.len(), solutions.len());
    for (level, solution) in levels.into_iter().zip(solutions) {
        let mut game = Game::new(level);
        assert!(!game.solved());
        for c in solution.chars() {
            assert!(
                game.step(Direction::from_char(c).unwrap()),
                "{}: invalid {c}",
                game.level.name
            );
        }
        assert!(game.solved(), "{} is unsolved", game.level.name);
        let (moves, pushes) = (game.moves, game.pushes);
        while game.undo() {}
        assert_eq!(game.player, game.level.player);
        assert_eq!(game.crates, game.level.crates);
        assert_eq!(game.pushes, 0);
        while game.redo() {}
        assert!(game.solved());
        assert_eq!((game.moves, game.pushes), (moves, pushes));
    }
}
#[test]
fn campaign_difficulty_and_identity_match_verified_metadata() {
    let levels = parse_collection(include_str!("../assets/levels.xsb")).unwrap();
    let metadata: serde_json::Value =
        serde_json::from_str(include_str!("../assets/campaign.json")).unwrap();
    let mut previous = (0, 0);
    let mut ids = std::collections::HashSet::new();
    let rows = metadata["levels"].as_array().unwrap();
    assert_eq!(levels.len(), rows.len());
    let mut names = std::collections::HashSet::new();
    for (level, row) in levels.iter().zip(rows) {
        assert!(names.insert(&level.name), "duplicate title: {}", level.name);
        assert!(ids.insert(&level.id), "duplicate board: {}", level.name);
        assert_eq!(level.name, row["name"].as_str().unwrap());
        let mut game = Game::new(level.clone());
        for c in row["solution"].as_str().unwrap().chars() {
            assert!(game.step(Direction::from_char(c).unwrap()));
        }
        assert!(game.solved());
        let pushes = row["pushes"].as_u64().unwrap();
        assert_eq!(game.pushes as u64, pushes);
        let difficulty = (pushes, row["overhead"].as_u64().unwrap());
        assert!(difficulty >= previous);
        assert_eq!(
            pushes - row["lower_bound"].as_u64().unwrap(),
            row["overhead"].as_u64().unwrap()
        );
        previous = difficulty;
    }
}

#[test]
fn merged_campaign_preserves_every_original_board_and_its_progress() {
    let old = parse_collection(include_str!("fixtures/first-collection.xsb")).unwrap();
    let campaign = parse_collection(include_str!("../assets/levels.xsb")).unwrap();
    let solutions: Vec<String> =
        serde_json::from_str(include_str!("first-collection-solutions.json")).unwrap();
    assert_eq!(old.len(), 18);
    assert_eq!(old.len(), solutions.len());
    let original = parse_collection(include_str!("fixtures/original-campaign.xsb")).unwrap();
    let original_ids: std::collections::HashSet<_> =
        old.iter().chain(&original).map(|l| &l.id).collect();
    let campaign_ids: std::collections::HashSet<_> = campaign.iter().map(|l| &l.id).collect();
    assert_eq!(original_ids.len(), 35);
    assert_eq!(original_ids, campaign_ids);
    for (level, solution) in old.into_iter().zip(solutions) {
        let mut game = Game::new(level);
        for c in solution.chars() {
            assert!(game.step(Direction::from_char(c).unwrap()));
        }
        assert!(game.solved(), "{}", game.level.name);
        let mut progress = Progress::default();
        progress.capture(&game);
        let merged = campaign.iter().find(|l| l.id == game.level.id).unwrap();
        let mut restored = Game::new(merged.clone());
        progress.restore(&mut restored);
        assert!(restored.solved());
        assert_eq!((restored.moves, restored.pushes), (game.moves, game.pushes));
    }
}

#[test]
fn walls_double_crates_and_branching_history() {
    let l = Level::parse("test", &["########", "#@ $$..#", "########"]).unwrap();
    let mut g = Game::new(l);
    assert!(!g.step(Direction::Up));
    assert_eq!(g.moves, 0);
    assert!(g.step(Direction::Right));
    assert!(!g.step(Direction::Right));
    assert!(g.undo());
    assert!(g.can_redo());
    assert!(g.step(Direction::Right));
    assert!(!g.can_redo());
}
#[test]
fn malformed_and_open_boards_are_rejected() {
    for board in [
        "###\n#@@\n###",
        "###\n#@$\n###",
        "#####\n#@$. \n#####",
        "###\n#@#\n###",
    ] {
        assert!(parse_collection(board).is_err());
    }
    assert!(parse_collection("No levels here").is_err());
}
#[test]
fn progress_replays_and_keeps_best_after_restart() {
    let l = parse_collection(include_str!("../assets/levels.xsb"))
        .unwrap()
        .remove(0);
    let mut g = Game::new(l.clone());
    g.step(Direction::Right);
    let mut p = Progress::default();
    p.capture(&g);
    let mut resumed = Game::new(l.clone());
    p.restore(&mut resumed);
    assert!(resumed.solved());
    assert!(resumed.can_undo());
    p.capture(&Game::new(l));
    assert_eq!(p.records[&g.level.id].best, Some((1, 1)));
    let path = std::env::temp_dir().join(format!("sokobanchy-test-{}.json", std::process::id()));
    p.save(&path).unwrap();
    assert_eq!(Progress::load(&path).unwrap().records.len(), 1);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn palettes_support_current_legacy_and_invalid_colors() {
    let t = Theme::parse(
        "background = '#ffffff'\naccent = '#123456'\ngreen = 'bad'",
        "test".into(),
    )
    .unwrap();
    assert!(t.light);
    assert_eq!(t.accent.to_array(), [18, 52, 86, 255]);
    assert_eq!(t.green, Theme::default().green);
    let t = Theme::parse("color4 = '#112233'", "old".into()).unwrap();
    assert_eq!(t.accent.to_array(), [17, 34, 51, 255]);
    assert!(Theme::parse("not toml", "bad".into()).is_none());
}
