use crate::indexer::IndexedSession;
use crate::names::NameStore;
use std::collections::BTreeMap;

pub fn display_name<'a>(session: &'a IndexedSession, names: &'a NameStore) -> &'a str {
    if let Some(custom) = names.get(&session.session_id) {
        return custom;
    }
    &session.name
}

pub fn group_by_project(sessions: &[IndexedSession]) -> BTreeMap<String, Vec<&IndexedSession>> {
    let mut groups: BTreeMap<String, Vec<&IndexedSession>> = BTreeMap::new();
    for session in sessions {
        groups.entry(session.project_dir.clone()).or_default().push(session);
    }
    groups
}

pub fn group_by_date(sessions: &[IndexedSession]) -> BTreeMap<String, Vec<&IndexedSession>> {
    let mut groups: BTreeMap<String, Vec<&IndexedSession>> = BTreeMap::new();
    for session in sessions {
        let dt = chrono::DateTime::from_timestamp(session.last_activity as i64, 0).unwrap_or_default();
        let date_str = dt.format("%Y-%m-%d").to_string();
        groups.entry(date_str).or_default().push(session);
    }
    groups
}

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
