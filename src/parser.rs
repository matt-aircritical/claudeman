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
        Value::Array(arr) => arr
            .iter()
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
            .join(" "),
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
        Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    item.get("text").and_then(|t| t.as_str()).map(String::from)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
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
