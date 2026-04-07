pub mod input;
pub mod search_input;
pub mod ui;

use crate::config::Config;
use crate::indexer::{IndexedSession, SessionIndex};
use crate::names::NameStore;
use crate::resume::ResumeOptions;
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
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
        let filtered: Vec<usize> = (0..sessions.len()).collect();
        let mut app = Self {
            sessions,
            filtered,
            selected: 0,
            view_mode: ViewMode::SearchResults,
            input_mode: InputMode::Normal,
            search_query: query,
            rename_buffer: String::new(),
            name_store,
            config,
            index,
            should_quit: false,
            resume_action: None,
            reindex_requested: false,
            status_message: String::new(),
        };
        app.run_search();
        app
    }

    pub fn selected_session(&self) -> Option<&IndexedSession> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.sessions.get(idx))
    }

    pub fn run_search(&mut self) {
        if self.search_query.is_empty() {
            self.filtered = (0..self.sessions.len()).collect();
            return;
        }
        match self.index.search(&self.search_query) {
            Ok(results) => {
                let result_ids: Vec<&str> =
                    results.iter().map(|s| s.session_id.as_str()).collect();
                self.filtered = self
                    .sessions
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| result_ids.contains(&s.session_id.as_str()))
                    .map(|(i, _)| i)
                    .collect();
            }
            Err(e) => {
                self.status_message = format!("Search error: {e}");
                self.filtered = Vec::new();
            }
        }
        self.selected = 0;
    }
}

pub fn run(mut app: App) -> Result<Option<ResumeOptions>> {
    let mut terminal = ratatui::init();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    input::handle_key(&mut app, key);
                }
            }
        }

        if app.should_quit || app.resume_action.is_some() {
            break;
        }
    }

    ratatui::restore();

    Ok(app.resume_action)
}
