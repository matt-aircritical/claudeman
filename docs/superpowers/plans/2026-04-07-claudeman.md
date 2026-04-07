# ClaudeMan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI tool that discovers, indexes, searches, and resumes Claude Code sessions from anywhere on the system.

**Architecture:** Tantivy indexes all session JSONL files for full-text search. ratatui provides the interactive TUI with split-pane layout. The tool auto-indexes on startup (incremental mtime check) and execs into `claude --resume` when the user selects a session.

**Tech Stack:** Rust, tantivy 0.26, ratatui 0.30, crossterm 0.29, clap 4.6, serde/serde_json, toml, dirs, chrono, anyhow

---

## File Structure

```
claudeman/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI arg parsing, dispatch
│   ├── config.rs            # Config loading (~/.config/claudeman/config.toml)
│   ├── scanner.rs           # Discovers session JSONL files from ~/.claude/
│   ├── parser.rs            # Parses JSONL files, extracts session metadata + messages
│   ├── indexer.rs           # Tantivy schema, indexing, incremental updates
│   ├── search.rs            # Search queries against Tantivy index
│   ├── names.rs             # Session renaming (sidecar names.toml)
│   ├── resume.rs            # cd + exec into claude --resume
│   ├── tui/
│   │   ├── mod.rs           # TUI app state, event loop, dispatch
│   │   ├── ui.rs            # ratatui widget rendering (layout, list, preview, tabs)
│   │   ├── input.rs         # Keyboard input handling, mode switching
│   │   └── search_input.rs  # Inline search bar + rename input widget
│   └── session.rs           # Session data struct shared across modules
├── tests/
│   ├── fixtures/            # Sample JSONL + JSON session files for tests
│   │   ├── sessions/
│   │   │   └── 12345.json
│   │   └── projects/
│   │       └── -home-test-project/
│   │           └── aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl
│   ├── scanner_test.rs
│   ├── parser_test.rs
│   ├── indexer_test.rs
│   ├── search_test.rs
│   ├── names_test.rs
│   └── config_test.rs
└── docs/
    └── superpowers/
        ├── specs/
        │   └── 2026-04-07-claudeman-session-manager-design.md
        └── plans/
            └── 2026-04-07-claudeman.md  (this file)
```

---

## Task 1: Project Scaffold + Session Data Struct

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/session.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "claudeman"
version = "0.1.0"
edition = "2021"
description = "Claude Code session manager"

[dependencies]
tantivy = "0.26"
ratatui = "0.30"
crossterm = "0.29"
clap = { version = "4.6", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "1.1"
dirs = "6.0"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
```

- [ ] **Step 2: Create the Session struct**

Create `src/session.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Core data struct representing a Claude Code session.
/// Shared across scanner, parser, indexer, and TUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub project_dir: String,
    pub cwd: String,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub name: String,
    pub model: String,
    pub version: String,
    pub message_count: u32,
    pub user_messages: String,
    pub assistant_messages: String,
    pub first_user_message: String,
    pub first_assistant_message: String,
    pub file_mtime: u64,
    pub jsonl_path: String,
}
```

- [ ] **Step 3: Create minimal main.rs**

Create `src/main.rs`:

```rust
mod session;

fn main() {
    println!("claudeman v0.1.0");
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully (dependencies download on first run)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/
git commit -m "feat: project scaffold with Session struct and dependencies"
```

---

## Task 2: Config Loading

**Files:**
- Create: `src/config.rs`
- Create: `tests/config_test.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing test for config loading**

Create `tests/config_test.rs`:

```rust
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
```

- [ ] **Step 2: Add tempfile dev-dependency**

Add to `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test config_test`
Expected: FAIL — `config` module doesn't exist yet

- [ ] **Step 4: Implement config.rs**

Create `src/config.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub claude_args: Vec<String>,
    pub claude_bin: String,
    pub claude_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            claude_args: Vec::new(),
            claude_bin: "claude".to_string(),
            claude_dir: None,
        }
    }
}

impl Config {
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        Self::load_from(&path)
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("claudeman")
            .join("config.toml")
    }

    pub fn claude_dir(&self) -> PathBuf {
        self.claude_dir
            .clone()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("~"))
                    .join(".claude")
            })
    }

    pub fn index_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("claudeman")
            .join("index")
    }
}
```

- [ ] **Step 5: Add module to main.rs**

Update `src/main.rs`:

```rust
mod config;
mod session;

fn main() {
    println!("claudeman v0.1.0");
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test config_test`
Expected: All 3 tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/config.rs tests/config_test.rs Cargo.toml
git commit -m "feat: config loading from ~/.config/claudeman/config.toml"
```

---

## Task 3: Session Scanner

**Files:**
- Create: `src/scanner.rs`
- Create: `tests/fixtures/sessions/12345.json`
- Create: `tests/fixtures/projects/-home-test-project/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl`
- Create: `tests/scanner_test.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create test fixtures**

Create `tests/fixtures/sessions/12345.json`:

```json
{"pid":12345,"sessionId":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee","cwd":"/home/test/project","startedAt":1775231143367,"kind":"interactive","entrypoint":"cli"}
```

Create `tests/fixtures/projects/-home-test-project/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl`:

```
{"type":"file-history-snapshot","messageId":"msg1","snapshot":{"messageId":"msg1","trackedFileBackups":{},"timestamp":"2026-03-09T12:11:23.509Z"},"isSnapshotUpdate":false}
{"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/home/test/project","sessionId":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee","version":"2.1.71","type":"user","message":{"role":"user","content":"help me fix the login bug"},"uuid":"msg1","timestamp":"2026-03-09T12:11:23.508Z"}
{"type":"assistant","message":{"role":"assistant","content":"I'll take a look at the login code. Let me start by reading the auth module."},"uuid":"msg2","timestamp":"2026-03-09T12:11:30.000Z"}
{"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/home/test/project","sessionId":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee","version":"2.1.71","type":"user","message":{"role":"user","content":"check the middleware too"},"uuid":"msg3","timestamp":"2026-03-09T12:15:00.000Z"}
{"type":"assistant","message":{"role":"assistant","content":"Found the issue in the auth middleware - the token validation was skipping expired tokens."},"uuid":"msg4","timestamp":"2026-03-09T12:15:10.000Z"}
```

- [ ] **Step 2: Write failing test for scanner**

Create `tests/scanner_test.rs`:

```rust
use std::path::PathBuf;

#[path = "../src/scanner.rs"]
mod scanner;

#[test]
fn test_discover_sessions() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let discovered = scanner::discover_sessions(&fixtures);
    assert_eq!(discovered.len(), 1);
    assert_eq!(
        discovered[0].session_id,
        "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    );
    assert!(discovered[0].jsonl_path.ends_with(".jsonl"));
}

#[test]
fn test_decode_mangled_path() {
    assert_eq!(
        scanner::decode_mangled_path("-home-test-project"),
        "/home/test/project"
    );
    assert_eq!(
        scanner::decode_mangled_path("-home-matt-projects-nanoclaw"),
        "/home/matt/projects/nanoclaw"
    );
}

#[test]
fn test_discover_skips_subagents() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let discovered = scanner::discover_sessions(&fixtures);
    for d in &discovered {
        assert!(!d.jsonl_path.contains("subagents"));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test scanner_test`
Expected: FAIL — `scanner` module doesn't exist

- [ ] **Step 4: Implement scanner.rs**

Create `src/scanner.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// A discovered session file before full parsing.
#[derive(Debug, Clone)]
pub struct DiscoveredSession {
    pub session_id: String,
    pub project_dir: String,
    pub jsonl_path: String,
    pub file_mtime: u64,
}

/// Scan the claude data directory for all session JSONL files.
/// Looks in `<claude_dir>/projects/<mangled-path>/<uuid>.jsonl`.
/// Skips subagent session files.
pub fn discover_sessions(claude_dir: &Path) -> Vec<DiscoveredSession> {
    let projects_dir = claude_dir.join("projects");
    if !projects_dir.exists() {
        return Vec::new();
    }

    let mut sessions = Vec::new();

    let project_entries = match fs::read_dir(&projects_dir) {
        Ok(entries) => entries,
        Err(_) => return sessions,
    };

    for project_entry in project_entries.flatten() {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }

        let mangled_name = project_entry.file_name().to_string_lossy().to_string();
        let project_dir = decode_mangled_path(&mangled_name);

        let jsonl_entries = match fs::read_dir(&project_path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in jsonl_entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                continue; // skip subagents/, tool-results/, etc.
            }

            let filename = entry.file_name().to_string_lossy().to_string();
            if !filename.ends_with(".jsonl") {
                continue;
            }

            let session_id = filename.trim_end_matches(".jsonl").to_string();

            // Validate it looks like a UUID
            if session_id.len() != 36 || session_id.chars().filter(|c| *c == '-').count() != 4 {
                continue;
            }

            let mtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            sessions.push(DiscoveredSession {
                session_id,
                project_dir: project_dir.clone(),
                jsonl_path: path.to_string_lossy().to_string(),
                file_mtime: mtime,
            });
        }
    }

    sessions
}

/// Decode a mangled directory name back to a filesystem path.
/// `-home-matt-projects` -> `/home/matt/projects`
pub fn decode_mangled_path(mangled: &str) -> String {
    if mangled.starts_with('-') {
        format!("/{}", mangled[1..].replace('-', "/"))
    } else {
        mangled.replace('-', "/")
    }
}
```

- [ ] **Step 5: Add module to main.rs**

Update `src/main.rs`:

```rust
mod config;
mod scanner;
mod session;

fn main() {
    println!("claudeman v0.1.0");
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test scanner_test`
Expected: All 3 tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/scanner.rs tests/scanner_test.rs tests/fixtures/
git commit -m "feat: session scanner discovers JSONL files from ~/.claude/"
```

---

## Task 4: JSONL Parser

**Files:**
- Create: `src/parser.rs`
- Create: `tests/parser_test.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing tests for parser**

Create `tests/parser_test.rs`:

```rust
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
    // file-history-snapshot should be skipped, only user+assistant counted
    assert_eq!(session.message_count, 4);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test parser_test`
Expected: FAIL — `parser` module doesn't exist

- [ ] **Step 3: Implement parser.rs**

Create `src/parser.rs`:

```rust
use crate::scanner::DiscoveredSession;
use crate::session::Session;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn parse_session(discovered: &DiscoveredSession) -> Result<Session> {
    let file = File::open(&discovered.jsonl_path)?;
    let reader = BufReader::new(file);

    let mut cwd = discovered.project_dir.clone();
    let mut started_at: Option<DateTime<Utc>> = None;
    let mut last_activity: Option<DateTime<Utc>> = None;
    let mut model = String::new();
    let mut version = String::new();
    let mut message_count: u32 = 0;
    let mut user_messages = Vec::new();
    let mut assistant_messages = Vec::new();
    let mut first_user_message = String::new();
    let mut first_assistant_message = String::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let value: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match msg_type {
            "user" => {
                message_count += 1;

                if let Some(ts) = parse_timestamp(&value) {
                    if started_at.is_none() {
                        started_at = Some(ts);
                    }
                    last_activity = Some(ts);
                }

                if let Some(c) = value.get("cwd").and_then(|c| c.as_str()) {
                    if cwd == discovered.project_dir {
                        cwd = c.to_string();
                    }
                }

                if version.is_empty() {
                    if let Some(v) = value.get("version").and_then(|v| v.as_str()) {
                        version = v.to_string();
                    }
                }

                let text = extract_user_text(&value);
                if !text.is_empty() {
                    if first_user_message.is_empty() {
                        first_user_message = truncate(&text, 500);
                    }
                    user_messages.push(text);
                }
            }
            "assistant" => {
                message_count += 1;

                if let Some(ts) = parse_timestamp(&value) {
                    last_activity = Some(ts);
                }

                if model.is_empty() {
                    if let Some(m) = value.get("model").and_then(|m| m.as_str()) {
                        model = m.to_string();
                    }
                }

                let text = extract_assistant_text(&value);
                if !text.is_empty() {
                    if first_assistant_message.is_empty() {
                        first_assistant_message = truncate(&text, 500);
                    }
                    assistant_messages.push(text);
                }
            }
            _ => continue,
        }
    }

    let now = Utc::now();
    let name = if first_user_message.len() > 80 {
        first_user_message[..80].to_string()
    } else {
        first_user_message.clone()
    };

    Ok(Session {
        session_id: discovered.session_id.clone(),
        project_dir: discovered.project_dir.clone(),
        cwd,
        started_at: started_at.unwrap_or(now),
        last_activity: last_activity.unwrap_or(now),
        name,
        model,
        version,
        message_count,
        user_messages: user_messages.join("\n"),
        assistant_messages: assistant_messages.join("\n"),
        first_user_message,
        first_assistant_message,
        file_mtime: discovered.file_mtime,
        jsonl_path: discovered.jsonl_path.clone(),
    })
}

fn parse_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    value
        .get("timestamp")
        .and_then(|t| t.as_str())
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
}

fn extract_user_text(value: &Value) -> String {
    let content = match value.get("message").and_then(|m| m.get("content")) {
        Some(c) => c,
        None => return String::new(),
    };

    match content {
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            arr.iter()
                .filter_map(|item| {
                    if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                        item.get("text").and_then(|t| t.as_str()).map(String::from)
                    } else if item.is_string() {
                        item.as_str().map(String::from)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        _ => String::new(),
    }
}

fn extract_assistant_text(value: &Value) -> String {
    let content = match value.get("message").and_then(|m| m.get("content")) {
        Some(c) => c,
        None => return String::new(),
    };

    match content {
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            arr.iter()
                .filter_map(|item| {
                    if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                        item.get("text").and_then(|t| t.as_str()).map(String::from)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        _ => String::new(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        s[..max].to_string()
    }
}
```

- [ ] **Step 4: Add module to main.rs**

Update `src/main.rs`:

```rust
mod config;
mod parser;
mod scanner;
mod session;

fn main() {
    println!("claudeman v0.1.0");
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test parser_test`
Expected: All 2 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/parser.rs tests/parser_test.rs
git commit -m "feat: JSONL parser extracts session metadata and messages"
```

---

## Task 5: Tantivy Indexer

**Files:**
- Create: `src/indexer.rs`
- Create: `tests/indexer_test.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing tests for indexer**

Create `tests/indexer_test.rs`:

```rust
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

    // Same mtime should indicate no re-index needed
    assert!(!idx.needs_reindex("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", 1000).unwrap());
    // Different mtime should indicate re-index needed
    assert!(idx.needs_reindex("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", 2000).unwrap());
    // Unknown session always needs indexing
    assert!(idx.needs_reindex("unknown-session-id", 1000).unwrap());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test indexer_test`
Expected: FAIL — `indexer` module doesn't exist

- [ ] **Step 3: Implement indexer.rs**

Create `src/indexer.rs`:

```rust
use crate::session::Session;
use anyhow::Result;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};

pub struct SessionIndex {
    index: Index,
    reader: IndexReader,
    schema: Schema,
    writer: std::sync::Mutex<IndexWriter>,
}

impl SessionIndex {
    pub fn create(path: &Path) -> Result<Self> {
        std::fs::create_dir_all(path)?;
        let schema = Self::build_schema();
        let index = Index::create_in_dir(path, schema.clone())
            .or_else(|_| Index::open_in_dir(path))?;

        let writer = index.writer(50_000_000)?; // 50MB heap
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            schema,
            writer: std::sync::Mutex::new(writer),
        })
    }

    pub fn open(path: &Path) -> Result<Self> {
        let schema = Self::build_schema();
        let index = Index::open_in_dir(path)?;
        let writer = index.writer(50_000_000)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            schema,
            writer: std::sync::Mutex::new(writer),
        })
    }

    fn build_schema() -> Schema {
        let mut builder = Schema::builder();
        builder.add_text_field("session_id", STRING | STORED);
        builder.add_text_field("project_dir", TEXT | STORED);
        builder.add_text_field("cwd", STORED);
        builder.add_u64_field("started_at", INDEXED | STORED);
        builder.add_u64_field("last_activity", INDEXED | STORED);
        builder.add_text_field("name", TEXT | STORED);
        builder.add_text_field("model", STORED);
        builder.add_text_field("version", STORED);
        builder.add_u64_field("message_count", STORED);
        builder.add_text_field("user_messages", TEXT);
        builder.add_text_field("assistant_messages", TEXT);
        builder.add_text_field("first_user_message", STORED);
        builder.add_text_field("first_assistant_message", STORED);
        builder.add_u64_field("file_mtime", STORED);
        builder.add_text_field("jsonl_path", STORED);
        builder.build()
    }

    pub fn add_session(&self, session: &Session) -> Result<()> {
        let writer = self.writer.lock().unwrap();

        let session_id_field = self.schema.get_field("session_id").unwrap();

        // Delete existing document for this session (for re-indexing)
        let term = tantivy::Term::from_field_text(session_id_field, &session.session_id);
        writer.delete_term(term);

        writer.add_document(doc!(
            self.schema.get_field("session_id").unwrap() => session.session_id.clone(),
            self.schema.get_field("project_dir").unwrap() => session.project_dir.clone(),
            self.schema.get_field("cwd").unwrap() => session.cwd.clone(),
            self.schema.get_field("started_at").unwrap() => session.started_at.timestamp() as u64,
            self.schema.get_field("last_activity").unwrap() => session.last_activity.timestamp() as u64,
            self.schema.get_field("name").unwrap() => session.name.clone(),
            self.schema.get_field("model").unwrap() => session.model.clone(),
            self.schema.get_field("version").unwrap() => session.version.clone(),
            self.schema.get_field("message_count").unwrap() => session.message_count as u64,
            self.schema.get_field("user_messages").unwrap() => session.user_messages.clone(),
            self.schema.get_field("assistant_messages").unwrap() => session.assistant_messages.clone(),
            self.schema.get_field("first_user_message").unwrap() => session.first_user_message.clone(),
            self.schema.get_field("first_assistant_message").unwrap() => session.first_assistant_message.clone(),
            self.schema.get_field("file_mtime").unwrap() => session.file_mtime,
            self.schema.get_field("jsonl_path").unwrap() => session.jsonl_path.clone(),
        ))?;

        Ok(())
    }

    pub fn commit(&self) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    pub fn all_sessions(&self) -> Result<Vec<IndexedSession>> {
        let searcher = self.reader.searcher();
        let top_docs = searcher.search(&AllQuery, &TopDocs::with_limit(10000))?;

        let mut sessions = Vec::new();
        for (_score, doc_addr) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_addr)?;
            if let Some(session) = self.doc_to_session(&doc) {
                sessions.push(session);
            }
        }

        sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        Ok(sessions)
    }

    pub fn search(&self, query_str: &str) -> Result<Vec<IndexedSession>> {
        let searcher = self.reader.searcher();
        let name = self.schema.get_field("name").unwrap();
        let user_messages = self.schema.get_field("user_messages").unwrap();
        let assistant_messages = self.schema.get_field("assistant_messages").unwrap();
        let project_dir = self.schema.get_field("project_dir").unwrap();

        let query_parser =
            QueryParser::for_index(&self.index, vec![name, user_messages, assistant_messages, project_dir]);
        let query = query_parser.parse_query(query_str)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(100))?;

        let mut sessions = Vec::new();
        for (_score, doc_addr) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_addr)?;
            if let Some(session) = self.doc_to_session(&doc) {
                sessions.push(session);
            }
        }

        Ok(sessions)
    }

    pub fn needs_reindex(&self, session_id: &str, current_mtime: u64) -> Result<bool> {
        let searcher = self.reader.searcher();
        let sid_field = self.schema.get_field("session_id").unwrap();
        let mtime_field = self.schema.get_field("file_mtime").unwrap();

        let term = tantivy::Term::from_field_text(sid_field, session_id);
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;

        if top_docs.is_empty() {
            return Ok(true);
        }

        let doc: TantivyDocument = searcher.doc(top_docs[0].1)?;
        let stored_mtime = doc
            .get_first(mtime_field)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(stored_mtime != current_mtime)
    }

    fn doc_to_session(&self, doc: &TantivyDocument) -> Option<IndexedSession> {
        let get_text = |name: &str| -> String {
            self.schema
                .get_field(name)
                .ok()
                .and_then(|field| doc.get_first(field))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        let get_u64 = |name: &str| -> u64 {
            self.schema
                .get_field(name)
                .ok()
                .and_then(|field| doc.get_first(field))
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        };

        Some(IndexedSession {
            session_id: get_text("session_id"),
            project_dir: get_text("project_dir"),
            cwd: get_text("cwd"),
            started_at: get_u64("started_at"),
            last_activity: get_u64("last_activity"),
            name: get_text("name"),
            model: get_text("model"),
            version: get_text("version"),
            message_count: get_u64("message_count") as u32,
            first_user_message: get_text("first_user_message"),
            first_assistant_message: get_text("first_assistant_message"),
            jsonl_path: get_text("jsonl_path"),
        })
    }

    pub fn remove_session(&self, session_id: &str) -> Result<()> {
        let writer = self.writer.lock().unwrap();
        let sid_field = self.schema.get_field("session_id").unwrap();
        let term = tantivy::Term::from_field_text(sid_field, session_id);
        writer.delete_term(term);
        Ok(())
    }
}

/// Lightweight session data retrieved from the index (no full message text).
#[derive(Debug, Clone)]
pub struct IndexedSession {
    pub session_id: String,
    pub project_dir: String,
    pub cwd: String,
    pub started_at: u64,
    pub last_activity: u64,
    pub name: String,
    pub model: String,
    pub version: String,
    pub message_count: u32,
    pub first_user_message: String,
    pub first_assistant_message: String,
    pub jsonl_path: String,
}
```

- [ ] **Step 4: Add module to main.rs**

Update `src/main.rs`:

```rust
mod config;
mod indexer;
mod parser;
mod scanner;
mod session;

fn main() {
    println!("claudeman v0.1.0");
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test indexer_test`
Expected: All 3 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/indexer.rs tests/indexer_test.rs
git commit -m "feat: Tantivy indexer with full-text search and incremental updates"
```

---

## Task 6: Session Names (Rename Feature)

**Files:**
- Create: `src/names.rs`
- Create: `tests/names_test.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/names_test.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test names_test`
Expected: FAIL — `names` module doesn't exist

- [ ] **Step 3: Implement names.rs**

Create `src/names.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NameStore {
    #[serde(default)]
    names: HashMap<String, String>,
}

impl NameStore {
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let store: NameStore = toml::from_str(&content)?;
        Ok(store)
    }

    pub fn load() -> Result<Self> {
        let path = Self::default_path();
        Self::load_from(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::default_path())
    }

    pub fn get(&self, session_id: &str) -> Option<&str> {
        self.names.get(session_id).map(|s| s.as_str())
    }

    pub fn set(&mut self, session_id: &str, name: &str) {
        self.names.insert(session_id.to_string(), name.to_string());
    }

    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("claudeman")
            .join("names.toml")
    }
}
```

- [ ] **Step 4: Add module to main.rs**

Update `src/main.rs`:

```rust
mod config;
mod indexer;
mod names;
mod parser;
mod scanner;
mod session;

fn main() {
    println!("claudeman v0.1.0");
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test names_test`
Expected: All 3 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/names.rs tests/names_test.rs
git commit -m "feat: session rename store (sidecar names.toml)"
```

---

## Task 7: Resume/Fork Logic

**Files:**
- Create: `src/resume.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Implement resume.rs with inline tests**

Create `src/resume.rs`:

```rust
use crate::config::Config;
use anyhow::Result;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

pub struct ResumeOptions {
    pub session_id: String,
    pub cwd: String,
    pub fork: bool,
}

/// Build the command that will be exec'd. Separated for testing.
pub fn build_resume_command(config: &Config, options: &ResumeOptions) -> Command {
    let mut cmd = Command::new(&config.claude_bin);
    cmd.arg("--resume");
    cmd.arg(&options.session_id);

    if options.fork {
        cmd.arg("--fork-session");
    }

    for arg in &config.claude_args {
        cmd.arg(arg);
    }

    // Set working directory to the session's original cwd if it exists
    let cwd_path = Path::new(&options.cwd);
    if cwd_path.exists() && cwd_path.is_dir() {
        cmd.current_dir(cwd_path);
    }

    cmd
}

/// Replace this process with claude --resume.
pub fn resume(config: &Config, options: &ResumeOptions) -> Result<()> {
    let mut cmd = build_resume_command(config, options);
    let err = cmd.exec();
    // exec() only returns on error
    Err(anyhow::anyhow!("Failed to launch claude: {}", err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_resume_command_basic() {
        let config = Config::default();
        let options = ResumeOptions {
            session_id: "abc-123".to_string(),
            cwd: "/tmp".to_string(),
            fork: false,
        };
        let cmd = build_resume_command(&config, &options);
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, &["--resume", "abc-123"]);
        assert_eq!(cmd.get_program(), "claude");
    }

    #[test]
    fn test_build_resume_command_with_fork_and_args() {
        let config = Config {
            claude_args: vec!["--dangerously-skip-permissions".to_string()],
            claude_bin: "claude".to_string(),
            claude_dir: None,
        };
        let options = ResumeOptions {
            session_id: "abc-123".to_string(),
            cwd: "/tmp".to_string(),
            fork: true,
        };
        let cmd = build_resume_command(&config, &options);
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(
            args,
            &["--resume", "abc-123", "--fork-session", "--dangerously-skip-permissions"]
        );
    }

    #[test]
    fn test_build_resume_command_nonexistent_cwd() {
        let config = Config::default();
        let options = ResumeOptions {
            session_id: "abc-123".to_string(),
            cwd: "/nonexistent/path/that/does/not/exist".to_string(),
            fork: false,
        };
        let cmd = build_resume_command(&config, &options);
        // Should not set current_dir when path doesn't exist
        assert!(cmd.get_current_dir().is_none());
    }
}
```

- [ ] **Step 2: Add module to main.rs**

Update `src/main.rs`:

```rust
mod config;
mod indexer;
mod names;
mod parser;
mod resume;
mod scanner;
mod session;

fn main() {
    println!("claudeman v0.1.0");
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test resume::tests`
Expected: All 3 tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/resume.rs
git commit -m "feat: resume/fork logic with configurable claude args"
```

---

## Task 8: Search Helpers

**Files:**
- Create: `src/search.rs`
- Create: `tests/search_test.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/search_test.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test search_test`
Expected: FAIL — `search` module doesn't exist

- [ ] **Step 3: Implement search.rs**

Create `src/search.rs`:

```rust
use crate::indexer::IndexedSession;
use crate::names::NameStore;
use std::collections::BTreeMap;

/// Get the display name for a session, respecting custom names.
pub fn display_name<'a>(session: &'a IndexedSession, names: &'a NameStore) -> &'a str {
    if let Some(custom) = names.get(&session.session_id) {
        return custom;
    }
    &session.name
}

/// Group sessions by their project directory.
pub fn group_by_project(sessions: &[IndexedSession]) -> BTreeMap<String, Vec<&IndexedSession>> {
    let mut groups: BTreeMap<String, Vec<&IndexedSession>> = BTreeMap::new();
    for session in sessions {
        groups
            .entry(session.project_dir.clone())
            .or_default()
            .push(session);
    }
    groups
}

/// Group sessions by date (day string).
pub fn group_by_date(sessions: &[IndexedSession]) -> BTreeMap<String, Vec<&IndexedSession>> {
    let mut groups: BTreeMap<String, Vec<&IndexedSession>> = BTreeMap::new();
    for session in sessions {
        let dt = chrono::DateTime::from_timestamp(session.last_activity as i64, 0)
            .unwrap_or_default();
        let date_str = dt.format("%Y-%m-%d").to_string();
        groups.entry(date_str).or_default().push(session);
    }
    groups
}

/// Format a timestamp as a relative or absolute date string for display.
pub fn format_date(timestamp: u64) -> String {
    let dt = chrono::DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_default();
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(dt);

    if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours().max(0))
    } else if diff.num_days() < 7 {
        format!("{}d ago", diff.num_days())
    } else {
        dt.format("%b %d").to_string()
    }
}
```

- [ ] **Step 4: Add module to main.rs**

Update `src/main.rs`:

```rust
mod config;
mod indexer;
mod names;
mod parser;
mod resume;
mod scanner;
mod search;
mod session;

fn main() {
    println!("claudeman v0.1.0");
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test search_test`
Expected: All 3 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/search.rs tests/search_test.rs
git commit -m "feat: search helpers for display names, grouping, date formatting"
```

---

## Task 9: CLI Argument Parsing + Indexing Orchestration

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement full main.rs with clap CLI**

Replace `src/main.rs` with:

```rust
mod config;
mod indexer;
mod names;
mod parser;
mod resume;
mod scanner;
mod search;
mod session;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claudeman", version, about = "Claude Code session manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Search sessions (opens TUI with search pre-filled)
    Search {
        /// Search query
        query: String,
    },
    /// List all sessions to stdout (non-interactive)
    List,
    /// Force a full re-index of all sessions
    Index,
    /// Show config path and current values
    Config,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::Config::load()?;

    match cli.command {
        None => {
            run_index(&config)?;
            let idx = indexer::SessionIndex::open(&config::Config::index_dir())?;
            let name_store = names::NameStore::load()?;
            let sessions = idx.all_sessions()?;

            if sessions.is_empty() {
                println!("No sessions found. Start a Claude session first, then run claudeman.");
                return Ok(());
            }

            let app = tui::App::new(sessions, name_store, config.clone(), idx);
            if let Some(resume_opts) = tui::run(app)? {
                resume::resume(&config, &resume_opts)?;
            }
        }
        Some(Commands::Search { query }) => {
            run_index(&config)?;
            let idx = indexer::SessionIndex::open(&config::Config::index_dir())?;
            let name_store = names::NameStore::load()?;
            let sessions = idx.all_sessions()?;

            let app = tui::App::new_with_search(
                sessions, name_store, config.clone(), idx, query,
            );
            if let Some(resume_opts) = tui::run(app)? {
                resume::resume(&config, &resume_opts)?;
            }
        }
        Some(Commands::List) => {
            run_index(&config)?;
            let idx = indexer::SessionIndex::open(&config::Config::index_dir())?;
            let name_store = names::NameStore::load()?;
            let sessions = idx.all_sessions()?;
            for session in &sessions {
                let name = search::display_name(session, &name_store);
                let date = search::format_date(session.last_activity);
                println!(
                    "{} | {:>8} | {:>4} msgs | {}",
                    &session.session_id[..8],
                    date,
                    session.message_count,
                    name
                );
            }
            println!("\n{} sessions total", sessions.len());
        }
        Some(Commands::Index) => {
            run_full_index(&config)?;
        }
        Some(Commands::Config) => {
            println!("Config path: {}", config::Config::config_path().display());
            println!("Index path:  {}", config::Config::index_dir().display());
            println!("Claude bin:  {}", config.claude_bin);
            println!("Claude args: {:?}", config.claude_args);
            println!("Claude dir:  {}", config.claude_dir().display());
        }
    }

    Ok(())
}

fn run_index(config: &config::Config) -> Result<()> {
    let index_dir = config::Config::index_dir();
    let idx = indexer::SessionIndex::create(&index_dir)?;
    let discovered = scanner::discover_sessions(&config.claude_dir());
    let mut indexed = 0;

    for d in &discovered {
        if idx.needs_reindex(&d.session_id, d.file_mtime)? {
            match parser::parse_session(d) {
                Ok(session) => {
                    idx.add_session(&session)?;
                    indexed += 1;
                }
                Err(e) => {
                    eprintln!("Warning: failed to parse {}: {}", d.jsonl_path, e);
                }
            }
        }
    }

    if indexed > 0 {
        idx.commit()?;
        eprintln!("Indexed {} new/updated sessions", indexed);
    }

    Ok(())
}

fn run_full_index(config: &config::Config) -> Result<()> {
    let index_dir = config::Config::index_dir();
    if index_dir.exists() {
        std::fs::remove_dir_all(&index_dir)?;
    }
    let idx = indexer::SessionIndex::create(&index_dir)?;
    let discovered = scanner::discover_sessions(&config.claude_dir());
    let mut indexed = 0;

    for d in &discovered {
        match parser::parse_session(d) {
            Ok(session) => {
                idx.add_session(&session)?;
                indexed += 1;
            }
            Err(e) => {
                eprintln!("Warning: failed to parse {}: {}", d.jsonl_path, e);
            }
        }
    }

    idx.commit()?;
    eprintln!("Re-indexed {} sessions", indexed);
    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compile error because `mod tui` doesn't exist yet. That's expected — we'll create it in Task 10. For now, comment out `mod tui;` and the TUI-related arms to verify the rest compiles.

Actually, create the placeholder first. Create `src/tui/mod.rs`:

```rust
pub mod input;
pub mod search_input;
pub mod ui;

// Placeholder — full implementation in Task 10
pub struct App;
pub fn run(_app: App) -> anyhow::Result<Option<crate::resume::ResumeOptions>> {
    Ok(None)
}

impl App {
    pub fn new(
        _sessions: Vec<crate::indexer::IndexedSession>,
        _name_store: crate::names::NameStore,
        _config: crate::config::Config,
        _index: crate::indexer::SessionIndex,
    ) -> Self {
        Self
    }
    pub fn new_with_search(
        _sessions: Vec<crate::indexer::IndexedSession>,
        _name_store: crate::names::NameStore,
        _config: crate::config::Config,
        _index: crate::indexer::SessionIndex,
        _query: String,
    ) -> Self {
        Self
    }
}
```

Create `src/tui/input.rs`:

```rust
// Placeholder — implemented in Task 10
```

Create `src/tui/search_input.rs`:

```rust
// Placeholder — implemented in Task 11
```

Create `src/tui/ui.rs`:

```rust
// Placeholder — implemented in Task 11
```

- [ ] **Step 3: Verify it compiles and basic commands work**

Run: `cargo build && cargo run -- --help`
Expected: Shows help with subcommands

Run: `cargo run -- config`
Expected: Prints config paths

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/tui/
git commit -m "feat: CLI arg parsing, indexing orchestration, TUI placeholders"
```

---

## Task 10: TUI — App State, Event Loop, Input Handling

**Files:**
- Modify: `src/tui/mod.rs`
- Modify: `src/tui/input.rs`

- [ ] **Step 1: Replace tui/mod.rs with full implementation**

Replace `src/tui/mod.rs` with:

```rust
pub mod input;
pub mod search_input;
pub mod ui;

use crate::config::Config;
use crate::indexer::{IndexedSession, SessionIndex};
use crate::names::NameStore;
use crate::resume::ResumeOptions;
use crate::search;
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    All,
    ByProject,
    ByDate,
    SearchResults,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    Rename,
}

pub struct App {
    pub sessions: Vec<IndexedSession>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub view_mode: ViewMode,
    pub input_mode: InputMode,
    pub search_query: String,
    pub rename_buffer: String,
    pub name_store: NameStore,
    pub config: Config,
    pub index: SessionIndex,
    pub should_quit: bool,
    pub resume_action: Option<ResumeOptions>,
    pub reindex_requested: bool,
    pub status_message: String,
}

impl App {
    pub fn new(
        sessions: Vec<IndexedSession>,
        name_store: NameStore,
        config: Config,
        index: SessionIndex,
    ) -> Self {
        let filtered: Vec<usize> = (0..sessions.len()).collect();
        Self {
            sessions,
            filtered,
            selected: 0,
            view_mode: ViewMode::All,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            rename_buffer: String::new(),
            name_store,
            config,
            index,
            should_quit: false,
            resume_action: None,
            reindex_requested: false,
            status_message: String::new(),
        }
    }

    pub fn new_with_search(
        sessions: Vec<IndexedSession>,
        name_store: NameStore,
        config: Config,
        index: SessionIndex,
        query: String,
    ) -> Self {
        let mut app = Self::new(sessions, name_store, config, index);
        app.search_query = query;
        app.view_mode = ViewMode::SearchResults;
        app.run_search();
        app
    }

    pub fn selected_session(&self) -> Option<&IndexedSession> {
        self.filtered
            .get(self.selected)
            .and_then(|&i| self.sessions.get(i))
    }

    pub fn run_search(&mut self) {
        if self.search_query.is_empty() {
            self.filtered = (0..self.sessions.len()).collect();
            return;
        }

        match self.index.search(&self.search_query) {
            Ok(results) => {
                let result_ids: Vec<String> =
                    results.iter().map(|r| r.session_id.clone()).collect();
                self.filtered = self
                    .sessions
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| result_ids.contains(&s.session_id))
                    .map(|(i, _)| i)
                    .collect();
            }
            Err(_) => {
                self.status_message = "Search error".to_string();
            }
        }
        self.selected = 0;
    }
}

pub fn run(mut app: App) -> Result<Option<ResumeOptions>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                input::handle_key(&mut app, key);
            }
        }

        if app.should_quit || app.resume_action.is_some() {
            break;
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(app.resume_action)
}
```

- [ ] **Step 2: Replace tui/input.rs with full implementation**

Replace `src/tui/input.rs` with:

```rust
use crate::resume::ResumeOptions;
use crate::tui::{App, InputMode, ViewMode};
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        InputMode::Normal => handle_normal(app, key),
        InputMode::Search => handle_search(app, key),
        InputMode::Rename => handle_rename(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Up => {
            if app.selected > 0 {
                app.selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.selected + 1 < app.filtered.len() {
                app.selected += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(session) = app.selected_session() {
                app.resume_action = Some(ResumeOptions {
                    session_id: session.session_id.clone(),
                    cwd: session.cwd.clone(),
                    fork: false,
                });
            }
        }
        KeyCode::Char('f') => {
            if let Some(session) = app.selected_session() {
                app.resume_action = Some(ResumeOptions {
                    session_id: session.session_id.clone(),
                    cwd: session.cwd.clone(),
                    fork: true,
                });
            }
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.status_message.clear();
        }
        KeyCode::Char('n') => {
            if let Some(session) = app.selected_session() {
                let current_name =
                    crate::search::display_name(session, &app.name_store).to_string();
                app.rename_buffer = current_name;
                app.input_mode = InputMode::Rename;
            }
        }
        KeyCode::Char('r') => {
            app.reindex_requested = true;
            app.should_quit = true;
        }
        KeyCode::Tab => {
            app.view_mode = match app.view_mode {
                ViewMode::All => ViewMode::ByProject,
                ViewMode::ByProject => ViewMode::ByDate,
                ViewMode::ByDate => ViewMode::SearchResults,
                ViewMode::SearchResults => ViewMode::All,
            };
            app.selected = 0;
        }
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                app.search_query.clear();
                app.filtered = (0..app.sessions.len()).collect();
                app.view_mode = ViewMode::All;
                app.selected = 0;
            }
        }
        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            app.view_mode = ViewMode::SearchResults;
            app.run_search();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
        }
        _ => {}
    }
}

fn handle_rename(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.rename_buffer.clear();
        }
        KeyCode::Enter => {
            if let Some(session) = app.selected_session() {
                let sid = session.session_id.clone();
                if !app.rename_buffer.is_empty() {
                    app.name_store.set(&sid, &app.rename_buffer);
                    let _ = app.name_store.save();
                    app.status_message = format!("Renamed to: {}", app.rename_buffer);
                }
            }
            app.rename_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.rename_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.rename_buffer.push(c);
        }
        _ => {}
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles (ui.rs is still a placeholder, which is fine)

- [ ] **Step 4: Commit**

```bash
git add src/tui/mod.rs src/tui/input.rs
git commit -m "feat: TUI app state, event loop, and input handling"
```

---

## Task 11: TUI — Full UI Rendering

**Files:**
- Modify: `src/tui/ui.rs`
- Modify: `src/tui/search_input.rs`

- [ ] **Step 1: Implement the full UI layout**

Replace `src/tui/ui.rs` with:

```rust
use crate::search;
use crate::tui::{App, InputMode, ViewMode};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search bar
            Constraint::Length(1), // tabs
            Constraint::Min(5),   // main content
            Constraint::Length(1), // status bar
        ])
        .split(f.area());

    draw_search_bar(f, app, chunks[0]);
    draw_tabs(f, app, chunks[1]);
    draw_main(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let (title, style) = match app.input_mode {
        InputMode::Search => (
            " Search (Enter to confirm, Esc to cancel) ",
            Style::default().fg(Color::Yellow),
        ),
        InputMode::Rename => (
            " Rename (Enter to confirm, Esc to cancel) ",
            Style::default().fg(Color::Green),
        ),
        InputMode::Normal => (
            " Search (press /) ",
            Style::default().fg(Color::DarkGray),
        ),
    };

    let display_text = match app.input_mode {
        InputMode::Search => &app.search_query,
        InputMode::Rename => &app.rename_buffer,
        InputMode::Normal => &app.search_query,
    };

    let input = Paragraph::new(display_text.as_str())
        .style(style)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(input, area);

    if app.input_mode == InputMode::Search {
        f.set_cursor_position(Position::new(
            area.x + app.search_query.len() as u16 + 1,
            area.y + 1,
        ));
    } else if app.input_mode == InputMode::Rename {
        f.set_cursor_position(Position::new(
            area.x + app.rename_buffer.len() as u16 + 1,
            area.y + 1,
        ));
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tab_titles: Vec<Line> = vec![
        format!("All ({})", app.sessions.len()),
        "By Project".to_string(),
        "By Date".to_string(),
        format!("Search ({})", app.filtered.len()),
    ]
    .into_iter()
    .map(Line::from)
    .collect();

    let selected = match app.view_mode {
        ViewMode::All => 0,
        ViewMode::ByProject => 1,
        ViewMode::ByDate => 2,
        ViewMode::SearchResults => 3,
    };

    let tabs = Tabs::new(tab_titles)
        .select(selected)
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, area);
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    draw_session_list(f, app, chunks[0]);
    draw_preview(f, app, chunks[1]);
}

fn draw_session_list(f: &mut Frame, app: &App, area: Rect) {
    let max_name_width = area.width.saturating_sub(14) as usize;

    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|&i| {
            let session = &app.sessions[i];
            let name = search::display_name(session, &app.name_store);
            let date = search::format_date(session.last_activity);
            let line = Line::from(vec![
                Span::styled(
                    truncate_str(name, max_name_width),
                    Style::default().fg(Color::White),
                ),
                Span::raw(" "),
                Span::styled(date, Style::default().fg(Color::DarkGray)),
            ]);
            let detail = Line::from(Span::styled(
                format!(
                    "  {} | {} msgs",
                    truncate_str(
                        &session.project_dir,
                        area.width.saturating_sub(16) as usize,
                    ),
                    session.message_count,
                ),
                Style::default().fg(Color::DarkGray),
            ));
            ListItem::new(vec![line, detail])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Sessions "))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Preview ");

    let content = if let Some(session) = app.selected_session() {
        let name = search::display_name(session, &app.name_store);
        let started = search::format_date(session.started_at);
        vec![
            Line::from(Span::styled(
                "SESSION PREVIEW",
                Style::default().fg(Color::Cyan),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::DarkGray)),
                Span::raw(&session.session_id),
            ]),
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    name,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Started: ", Style::default().fg(Color::DarkGray)),
                Span::raw(started),
                Span::raw(" | "),
                Span::raw(&session.model),
            ]),
            Line::from(vec![
                Span::styled("CWD: ", Style::default().fg(Color::DarkGray)),
                Span::raw(&session.cwd),
            ]),
            Line::from(vec![
                Span::styled("Messages: ", Style::default().fg(Color::DarkGray)),
                Span::raw(session.message_count.to_string()),
            ]),
            Line::from(""),
            Line::from(Span::styled("YOU:", Style::default().fg(Color::Green))),
            Line::from(session.first_user_message.as_str()),
            Line::from(""),
            Line::from(Span::styled("CLAUDE:", Style::default().fg(Color::Blue))),
            Line::from(session.first_assistant_message.as_str()),
        ]
    } else {
        vec![Line::from("No session selected")]
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let left = if app.status_message.is_empty() {
        let project_count = app
            .sessions
            .iter()
            .map(|s| &s.project_dir)
            .collect::<std::collections::HashSet<_>>()
            .len();
        format!("{} sessions | {} projects", app.sessions.len(), project_count)
    } else {
        app.status_message.clone()
    };

    let right = "ENTER:resume  f:fork  n:rename  r:reindex  q:quit";

    let bar = Line::from(vec![
        Span::styled(left, Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled(right, Style::default().fg(Color::DarkGray)),
    ]);

    f.render_widget(Paragraph::new(bar), area);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        format!("{}...", &s[..max - 3])
    } else {
        s[..max].to_string()
    }
}
```

- [ ] **Step 2: Update search_input.rs**

Replace `src/tui/search_input.rs` with:

```rust
// Search and rename input are handled inline in ui.rs draw_search_bar.
// This module reserved for future extraction if the input widget grows complex.
```

- [ ] **Step 3: Verify it compiles and runs**

Run: `cargo build && cargo run`
Expected: TUI launches, shows session list and preview. Arrow keys navigate, `/` opens search, `n` opens rename, `q` quits.

- [ ] **Step 4: Commit**

```bash
git add src/tui/ui.rs src/tui/search_input.rs
git commit -m "feat: full TUI rendering with split-pane layout, tabs, preview"
```

---

## Task 12: End-to-End Smoke Test + Release Build

**Files:**
- No new files

- [ ] **Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 2: Build release binary**

Run: `cargo build --release`
Expected: Binary at `target/release/claudeman`

- [ ] **Step 3: Smoke test commands**

Run: `cargo run -- list`
Expected: Lists all Claude Code sessions on this system

Run: `cargo run -- config`
Expected: Shows config paths

Run: `cargo run -- index`
Expected: Full re-index, prints count

Run: `cargo run`
Expected: TUI launches with all sessions visible. Arrow keys, search, rename all work.

- [ ] **Step 4: Commit any fixes from smoke testing**

```bash
git add -A
git commit -m "fix: smoke test fixes"
```

(Skip this commit if no fixes were needed.)

---

## Task 13: Package Metadata + Local Install

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add package metadata**

Add under `[package]` in `Cargo.toml`:

```toml
authors = ["Matt"]
license = "MIT"
keywords = ["claude", "session", "manager", "tui"]
categories = ["command-line-utilities"]
```

- [ ] **Step 2: Install locally**

Run: `cargo install --path .`
Expected: `claudeman` available in PATH

- [ ] **Step 3: Verify installed binary**

Run: `claudeman list`
Expected: Lists sessions

Run: `claudeman`
Expected: TUI launches

- [ ] **Step 4: Commit and merge**

```bash
git add Cargo.toml
git commit -m "chore: add package metadata, ready for local install"
git checkout main
git merge development
git checkout development
```
