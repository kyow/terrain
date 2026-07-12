#![cfg(feature = "bundled-provider")]

use std::fs;
use std::path::PathBuf;

use terrain::build_engine;
use traverze::{SearchOptions, Traverze};

fn search_paths(engine: &Traverze, query: &str) -> Vec<String> {
    engine
        .search_with_options(
            query,
            SearchOptions {
                limit: 10,
                snippet: None,
            },
        )
        .unwrap()
        .into_iter()
        .map(|h| h.path)
        .collect()
}

/// Rebuilding into the same index dir must drop documents from previous
/// runs so search never returns files outside the current file set (#42).
#[test]
fn build_engine_resets_previous_index() {
    let tmp = tempfile::tempdir().unwrap();
    let index_dir = tmp.path().join("index");
    let docs = tmp.path().join("docs");
    fs::create_dir_all(&docs).unwrap();

    let old_file = docs.join("old.md");
    fs::write(&old_file, "zebra galaxy notes").unwrap();
    let old_file = fs::canonicalize(&old_file).unwrap();

    // First run: index only old.md, then drop the engine to release the
    // index files (Windows cannot delete files that are still open).
    {
        let (engine, indexed) = build_engine(&index_dir, &[old_file]).unwrap();
        assert_eq!(indexed, 1);
        assert_eq!(search_paths(&engine, "zebra").len(), 1);
    }

    let new_file = docs.join("new.md");
    fs::write(&new_file, "quartz harbor notes").unwrap();
    let new_file = fs::canonicalize(&new_file).unwrap();

    // Second run against a different file set: old.md must be gone.
    let (engine, indexed) = build_engine(&index_dir, &[new_file]).unwrap();
    assert_eq!(indexed, 1);
    assert_eq!(search_paths(&engine, "quartz").len(), 1);
    assert!(search_paths(&engine, "zebra").is_empty());
}

/// A missing index dir is not an error: build_engine creates it.
#[test]
fn build_engine_creates_missing_index_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let index_dir = tmp.path().join("does-not-exist-yet");

    let (_engine, indexed) = build_engine(&index_dir, &[] as &[PathBuf]).unwrap();
    assert_eq!(indexed, 0);
}
