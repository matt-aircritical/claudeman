use std::path::PathBuf;

#[path = "../src/session.rs"]
mod session;
#[path = "../src/scanner.rs"]
mod scanner;
#[path = "../src/parser.rs"]
mod parser;

#[test]
fn test_parse_session() {
    let jsonl_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/projects/-home-test-project/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl");

    let discovered = scanner::DiscoveredSession {
        session_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
        project_dir: "/home/test/project".to_string(),
        jsonl_path: jsonl_path.to_string_lossy().to_string(),
        file_mtime: 1000,
    };

    let session = parser::parse_session(&discovered).unwrap();
    assert_eq!(session.session_id, "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
    assert_eq!(session.cwd, "/home/test/project");
    assert_eq!(session.message_count, 4); // 2 user + 2 assistant
    assert!(session.user_messages.contains("login bug"));
    assert!(session.user_messages.contains("middleware"));
    assert!(session.assistant_messages.contains("auth module"));
    assert_eq!(session.first_user_message, "help me fix the login bug");
    assert!(session.first_assistant_message.contains("take a look"));
    assert_eq!(session.name, "help me fix the login bug");
    assert_eq!(session.version, "2.1.71");
}

#[test]
fn test_parse_skips_non_message_types() {
    let jsonl_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/projects/-home-test-project/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl");

    let discovered = scanner::DiscoveredSession {
        session_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
        project_dir: "/home/test/project".to_string(),
        jsonl_path: jsonl_path.to_string_lossy().to_string(),
        file_mtime: 1000,
    };

    let session = parser::parse_session(&discovered).unwrap();
    assert_eq!(session.message_count, 4);
}
