use std::path::PathBuf;
use tempfile::tempdir;

#[path = "../src/names.rs"]
mod names;

#[test]
fn test_set_and_get_name() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("names.toml");
    let mut store = names::NameStore::load_from(&path).unwrap();
    store.set("session-123", "My cool session");
    store.save_to(&path).unwrap();
    let store2 = names::NameStore::load_from(&path).unwrap();
    assert_eq!(store2.get("session-123"), Some("My cool session"));
    assert_eq!(store2.get("unknown"), None);
}

#[test]
fn test_load_missing_file_returns_empty() {
    let store = names::NameStore::load_from(&PathBuf::from("/nonexistent/names.toml")).unwrap();
    assert_eq!(store.get("anything"), None);
}

#[test]
fn test_overwrite_name() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("names.toml");
    let mut store = names::NameStore::load_from(&path).unwrap();
    store.set("session-123", "Original name");
    store.set("session-123", "Updated name");
    store.save_to(&path).unwrap();
    let store2 = names::NameStore::load_from(&path).unwrap();
    assert_eq!(store2.get("session-123"), Some("Updated name"));
}
