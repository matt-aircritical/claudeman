pub mod input;
pub mod search_input;
pub mod ui;

use crate::config::Config;
use crate::indexer::{IndexedSession, SessionIndex};
use crate::names::NameStore;
use crate::resume::ResumeOptions;

// Placeholder — full implementation in Task 10
pub struct App {
    pub sessions: Vec<IndexedSession>,
}

impl App {
    pub fn new(
        sessions: Vec<IndexedSession>,
        _name_store: NameStore,
        _config: Config,
        _index: SessionIndex,
    ) -> Self {
        Self { sessions }
    }
    pub fn new_with_search(
        sessions: Vec<IndexedSession>,
        _name_store: NameStore,
        _config: Config,
        _index: SessionIndex,
        _query: String,
    ) -> Self {
        Self { sessions }
    }
}

pub fn run(_app: App) -> anyhow::Result<Option<ResumeOptions>> {
    println!("TUI not yet implemented - use `claudeman list` for now");
    Ok(None)
}
