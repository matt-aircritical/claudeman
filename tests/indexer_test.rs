use std::path::PathBuf;
use tempfile::tempdir;

#[path = "../src/session.rs"]
mod session;
#[path = "../src/scanner.rs"]
mod scanner;
#[path = "../src/parser.rs"]
mod parser;
#[path = "../src/indexer.rs"]
mod indexer;

fn make_test_session() -> session::Session {
    let jsonl_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/projects/-home-test-project/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl");
    let discovered = scanner::DiscoveredSession {
        session_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
        project_dir: "/home/test/project".to_string(),
        jsonl_path: jsonl_path.to_string_lossy().to_string(),
        file_mtime: 1000,
    };
    parser::parse_session(&discovered).unwrap()
}

#[test]
fn test_index_and_retrieve_all() {
    let dir = tempdir().unwrap();
    let idx = indexer::SessionIndex::create(dir.path()).unwrap();
    let session = make_test_session();
    idx.add_session(&session).unwrap();
    idx.commit().unwrap();
    let all = idx.all_sessions().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].session_id, "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
    assert_eq!(all[0].name, "help me fix the login bug");
}

#[test]
fn test_search_by_content() {
    let dir = tempdir().unwrap();
    let idx = indexer::SessionIndex::create(dir.path()).unwrap();
    let session = make_test_session();
    idx.add_session(&session).unwrap();
    idx.commit().unwrap();

    let results = idx.search("login bug").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].session_id, "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");

    let results = idx.search("middleware").unwrap();
    assert_eq!(results.len(), 1);

    let results = idx.search("nonexistent garbage query xyz").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_incremental_skip() {
    let dir = tempdir().unwrap();
    let idx = indexer::SessionIndex::create(dir.path()).unwrap();
    let session = make_test_session();
    idx.add_session(&session).unwrap();
    idx.commit().unwrap();

    assert!(!idx.needs_reindex("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", 1000).unwrap());
    assert!(idx.needs_reindex("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", 2000).unwrap());
    assert!(idx.needs_reindex("unknown-session-id", 1000).unwrap());
}
