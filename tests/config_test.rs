use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[path = "../src/config.rs"]
mod config;

#[test]
fn test_default_config() {
    let cfg = config::Config::default();
    assert!(cfg.claude_args.is_empty());
    assert_eq!(cfg.claude_bin, "claude");
    assert!(cfg.claude_dir.is_none());
}

#[test]
fn test_load_config_from_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
claude_args = ["--dangerously-skip-permissions"]
claude_bin = "/usr/local/bin/claude"
"#,
    )
    .unwrap();

    let cfg = config::Config::load_from(&config_path).unwrap();
    assert_eq!(cfg.claude_args, vec!["--dangerously-skip-permissions"]);
    assert_eq!(cfg.claude_bin, "/usr/local/bin/claude");
}

#[test]
fn test_load_missing_config_returns_default() {
    let cfg = config::Config::load_from(&PathBuf::from("/nonexistent/config.toml")).unwrap();
    assert!(cfg.claude_args.is_empty());
}
