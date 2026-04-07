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
use std::collections::BTreeMap;
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

/// An item in the display list — either a group header or a session entry.
#[derive(Debug, Clone)]
pub enum DisplayItem {
    Header(String),
    Session(usize), // index into app.sessions
}

pub struct App {
    pub sessions: Vec<IndexedSession>,
    pub filtered: Vec<usize>,
    pub display_items: Vec<DisplayItem>,
    pub selected: usize, // index into display_items
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
        let mut app = Self {
            sessions,
            filtered,
            display_items: Vec::new(),
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
        };
        app.rebuild_display_items();
        app
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
            display_items: Vec::new(),
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
        match self.display_items.get(self.selected) {
            Some(DisplayItem::Session(idx)) => self.sessions.get(*idx),
            _ => None,
        }
    }

    pub fn session_count(&self) -> usize {
        self.display_items
            .iter()
            .filter(|item| matches!(item, DisplayItem::Session(_)))
            .count()
    }

    pub fn run_search(&mut self) {
        if self.search_query.is_empty() {
            self.filtered = (0..self.sessions.len()).collect();
        } else {
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
        }
        self.rebuild_display_items();
    }

    /// Rebuild the display_items list based on current view mode and filtered sessions.
    pub fn rebuild_display_items(&mut self) {
        self.display_items.clear();

        let filtered_sessions: Vec<&IndexedSession> = self
            .filtered
            .iter()
            .filter_map(|&idx| self.sessions.get(idx))
            .collect();

        match self.view_mode {
            ViewMode::All | ViewMode::SearchResults => {
                // Flat list, sorted by last_activity (already sorted)
                for &idx in &self.filtered {
                    self.display_items.push(DisplayItem::Session(idx));
                }
            }
            ViewMode::ByProject => {
                // Group by project directory
                let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
                for &idx in &self.filtered {
                    let project = &self.sessions[idx].project_dir;
                    groups.entry(project.clone()).or_default().push(idx);
                }
                for (project, indices) in &groups {
                    let short = shorten_project_path(project);
                    self.display_items
                        .push(DisplayItem::Header(format!(" {} ({}) ", short, indices.len())));
                    for &idx in indices {
                        self.display_items.push(DisplayItem::Session(idx));
                    }
                }
            }
            ViewMode::ByDate => {
                // Group by date
                let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
                for &idx in &self.filtered {
                    let ts = self.sessions[idx].last_activity;
                    let dt = chrono::DateTime::from_timestamp(ts as i64, 0)
                        .unwrap_or_default();
                    let date_str = dt.format("%A, %B %d %Y").to_string();
                    let sort_key = dt.format("%Y-%m-%d").to_string();
                    groups
                        .entry(format!("{}|{}", sort_key, date_str))
                        .or_default()
                        .push(idx);
                }
                // Reverse order (newest first)
                for (key, indices) in groups.iter().rev() {
                    let date_label = key.split('|').nth(1).unwrap_or(key);
                    self.display_items.push(DisplayItem::Header(format!(
                        " {} ({}) ",
                        date_label,
                        indices.len()
                    )));
                    for &idx in indices {
                        self.display_items.push(DisplayItem::Session(idx));
                    }
                }
            }
        }

        // Reset selection to first session item (skip headers)
        self.selected = 0;
        self.skip_to_next_session();
    }

    /// Move selection to the next session item if currently on a header.
    pub fn skip_to_next_session(&mut self) {
        while self.selected < self.display_items.len() {
            if matches!(self.display_items[self.selected], DisplayItem::Session(_)) {
                return;
            }
            self.selected += 1;
        }
        // If we went past the end, try to find any session
        if self.selected >= self.display_items.len() {
            for (i, item) in self.display_items.iter().enumerate() {
                if matches!(item, DisplayItem::Session(_)) {
                    self.selected = i;
                    return;
                }
            }
            self.selected = 0;
        }
    }
}

fn shorten_project_path(path: &str) -> &str {
    // Show last 2 path components for readability
    let parts: Vec<&str> = path.rsplitn(3, '/').collect();
    if parts.len() >= 2 {
        let start = path.len() - parts[0].len() - parts[1].len() - 1;
        &path[start..]
    } else {
        path
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
