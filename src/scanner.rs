use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone)]
pub struct DiscoveredSession {
    pub session_id: String,
    pub project_dir: String,
    pub jsonl_path: String,
    pub file_mtime: u64,
}

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
                continue;
            }

            let filename = entry.file_name().to_string_lossy().to_string();
            if !filename.ends_with(".jsonl") {
                continue;
            }

            let session_id = filename.trim_end_matches(".jsonl").to_string();

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

pub fn decode_mangled_path(mangled: &str) -> String {
    if mangled.starts_with('-') {
        format!("/{}", mangled[1..].replace('-', "/"))
    } else {
        mangled.replace('-', "/")
    }
}
