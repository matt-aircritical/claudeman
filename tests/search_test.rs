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
#[path = "../src/names.rs"]
mod names;
#[path = "../src/search.rs"]
mod search;

fn setup_index() -> (tempfile::TempDir, indexer::SessionIndex) {
    let dir = tempdir().unwrap();
    let idx = indexer::SessionIndex::create(dir.path()).unwrap();
    let jsonl_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/projects/-home-test-project/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl");
    let discovered = scanner::DiscoveredSession {
        session_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
        project_dir: "/home/test/project".to_string(),
        jsonl_path: jsonl_path.to_string_lossy().to_string(),
        file_mtime: 1000,
    };
    let session = parser::parse_session(&discovered).unwrap();
    idx.add_session(&session).unwrap();
    idx.commit().unwrap();
    (dir, idx)
}

#[test]
fn test_display_name_uses_custom_name() {
    let mut name_store = names::NameStore::default();
    name_store.set("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "Login bug fix");
    let (_dir, idx) = setup_index();
    let sessions = idx.all_sessions().unwrap();
    let display = search::display_name(&sessions[0], &name_store);
    assert_eq!(display, "Login bug fix");
}

#[test]
fn test_display_name_falls_back_to_index_name() {
    let name_store = names::NameStore::default();
    let (_dir, idx) = setup_index();
    let sessions = idx.all_sessions().unwrap();
    let display = search::display_name(&sessions[0], &name_store);
    assert_eq!(display, "help me fix the login bug");
}

#[test]
fn test_group_by_project() {
    let (_dir, idx) = setup_index();
    let sessions = idx.all_sessions().unwrap();
    let groups = search::group_by_project(&sessions);
    assert_eq!(groups.len(), 1);
    assert!(groups.contains_key("/home/test/project"));
    assert_eq!(groups["/home/test/project"].len(), 1);
}
