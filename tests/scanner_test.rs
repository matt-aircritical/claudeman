use std::path::PathBuf;

#[path = "../src/scanner.rs"]
mod scanner;

#[test]
fn test_discover_sessions() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let discovered = scanner::discover_sessions(&fixtures);
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].session_id, "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
    assert!(discovered[0].jsonl_path.ends_with(".jsonl"));
}

#[test]
fn test_decode_mangled_path() {
    assert_eq!(scanner::decode_mangled_path("-home-test-project"), "/home/test/project");
    assert_eq!(scanner::decode_mangled_path("-home-matt-projects-nanoclaw"), "/home/matt/projects/nanoclaw");
}

#[test]
fn test_discover_skips_subagents() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let discovered = scanner::discover_sessions(&fixtures);
    for d in &discovered {
        assert!(!d.jsonl_path.contains("subagents"));
    }
}
