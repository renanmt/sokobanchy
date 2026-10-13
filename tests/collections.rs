use sokobanchy::{collections::COLLECTIONS, game::*};

#[test]
fn every_embedded_collection_and_solution_is_playable() {
    let witnesses: std::collections::BTreeMap<String, Vec<String>> =
        serde_json::from_str(include_str!("collection-solutions.json")).unwrap();
    for collection in COLLECTIONS {
        let levels = parse_collection(collection.boards)
            .unwrap_or_else(|e| panic!("{}: {e}", collection.name));
        assert_eq!(levels.len(), collection.count);
        assert_eq!(levels.len(), witnesses[collection.id].len());
        assert!(!collection.author.is_empty());
        assert!(collection.notes.contains("distributed"));
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/collections/sources")
            .join(format!("{}.sok", collection.id));
        let imported = parse_collection_bytes(&std::fs::read(source).unwrap()).unwrap();
        assert_eq!(imported.len(), levels.len());
        for (original, embedded) in imported.iter().zip(&levels) {
            assert_eq!(original.id, embedded.id, "{}", collection.name);
        }
        for (level, solution) in levels.into_iter().zip(&witnesses[collection.id]) {
            let mut game = Game::new(level);
            for c in solution.chars() {
                assert!(
                    game.step(Direction::from_char(c).unwrap()),
                    "{} / {}: invalid move {c}",
                    collection.name,
                    game.level.name
                );
            }
            assert!(game.solved(), "{} / {}", collection.name, game.level.name);
        }
    }
}

#[test]
fn external_text_encodings_preserve_titles_and_reject_binary_files() {
    let bytes = b"; Caf\xe9 \x96 first push\r\n#####\r\n#@$.#\r\n#####\r\n";
    let levels = parse_collection_bytes(bytes).unwrap();
    assert_eq!(levels[0].name, "Café – first push");
    let utf8 = "\u{feff}; Café – first push\n#####\n#@$.#\n#####\n";
    assert_eq!(
        parse_collection_bytes(utf8.as_bytes()).unwrap()[0].id,
        levels[0].id
    );
    assert!(parse_collection_bytes(b"\0binary data").is_err());
    assert!(parse_collection_bytes(&vec![b' '; 8 * 1024 * 1024 + 1]).is_err());
}
