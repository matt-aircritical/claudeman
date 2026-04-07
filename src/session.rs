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
