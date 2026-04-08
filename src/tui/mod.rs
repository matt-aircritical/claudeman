pub mod input;
pub mod search_input;
pub mod ui;

use crate::config::Config;
use crate::indexer::{IndexedSession, SessionIndex};
use crate::names::NameStore;
use crate::parser;
use crate::resume::ResumeOptions;
use crate::scanner;
use crate::search;
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
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

/// A loaded exchange from the JSONL file for expanded preview.
#[derive(Debug, Clone)]
pub struct Exchange {
    pub role: String,   // "user" or "assistant"
    pub text: String,
    pub line_index: usize, // index of this message's raw line in the JSONL
}

/// Right-click context menu state.
#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub visible: bool,
    pub x: u16,
    pub y: u16,
    pub render_x: u16, // actual clamped position set during draw
    pub render_y: u16,
    pub selected: usize,
    pub items: Vec<(&'static str, &'static str)>, // (label, action_key)
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            render_x: 0,
            render_y: 0,
            selected: 0,
            items: vec![
                ("  Resume       ", "resume"),
                ("  Fork         ", "fork"),
                ("  Rename       ", "rename"),
                ("  Preview      ", "preview"),
                ("  Delete       ", "delete"),
            ],
        }
    }
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
    // Expanded preview
    pub expanded_preview: bool,
    pub preview_scroll: u16,
    pub preview_exchanges: Vec<Exchange>,
    pub preview_selected_exchange: usize,
    pub preview_exchange_lines: Vec<(usize, usize)>, // (start_line, end_line) for each exchange in preview
    pub preview_session_id: String, // which session is loaded
    // Delete confirmation
    pub confirm_delete: bool,
    // Help overlay
    pub show_help: bool,
    // Right-click context menu
    pub context_menu: ContextMenu,
    // Double-click tracking
    pub last_click_time: std::time::Instant,
    pub last_click_idx: Option<usize>,
    // Layout areas for mouse mapping
    pub search_area: Rect,
    pub list_area: Rect,
    pub preview_area: Rect,
    pub tabs_area: Rect,
    pub list_state: ListState,
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
            expanded_preview: false,
            preview_scroll: 0,
            preview_exchanges: Vec::new(),
            preview_selected_exchange: 0,
            preview_exchange_lines: Vec::new(),
            preview_session_id: String::new(),
            confirm_delete: false,
            show_help: false,
            context_menu: ContextMenu::default(),
            last_click_time: std::time::Instant::now(),
            last_click_idx: None,
            search_area: Rect::default(),
            list_area: Rect::default(),
            preview_area: Rect::default(),
            tabs_area: Rect::default(),
            list_state: ListState::default(),
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
            expanded_preview: false,
            preview_scroll: 0,
            preview_exchanges: Vec::new(),
            preview_selected_exchange: 0,
            preview_exchange_lines: Vec::new(),
            preview_session_id: String::new(),
            confirm_delete: false,
            show_help: false,
            context_menu: ContextMenu::default(),
            last_click_time: std::time::Instant::now(),
            last_click_idx: None,
            search_area: Rect::default(),
            list_area: Rect::default(),
            preview_area: Rect::default(),
            tabs_area: Rect::default(),
            list_state: ListState::default(),
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

    /// Load full conversation exchanges from the JSONL file on disk.
    pub fn load_expanded_preview(&mut self) {
        let session = match self.selected_session() {
            Some(s) => s,
            None => return,
        };

        // Don't reload if already loaded for this session
        if self.preview_session_id == session.session_id && !self.preview_exchanges.is_empty() {
            return;
        }

        let jsonl_path = session.jsonl_path.clone();
        let session_id = session.session_id.clone();

        let discovered = scanner::DiscoveredSession {
            session_id: session_id.clone(),
            project_dir: session.project_dir.clone(),
            jsonl_path: jsonl_path.clone(),
            file_mtime: 0,
        };

        match parser::parse_session(&discovered) {
            Ok(parsed) => {
                let mut exchanges = Vec::new();

                // Re-parse the JSONL to get individual messages
                if let Ok(file) = std::fs::File::open(&jsonl_path) {
                    use std::io::{BufRead, BufReader};
                    let reader = BufReader::new(file);
                    for (line_idx, line) in reader.lines().flatten().enumerate() {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                            let msg_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            let text = extract_message_text(&value);
                            if !text.is_empty() {
                                match msg_type {
                                    "user" => exchanges.push(Exchange {
                                        role: "user".to_string(),
                                        text,
                                        line_index: line_idx,
                                    }),
                                    "assistant" => exchanges.push(Exchange {
                                        role: "assistant".to_string(),
                                        text,
                                        line_index: line_idx,
                                    }),
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                self.preview_exchanges = exchanges;
                self.preview_selected_exchange = 0;
                self.preview_session_id = session_id;
                self.preview_scroll = 0;
            }
            Err(e) => {
                self.status_message = format!("Failed to load preview: {e}");
            }
        }
    }

    /// Fork from a specific exchange in the expanded preview.
    /// Writes a truncated JSONL containing lines up to and including the
    /// selected exchange's assistant response, then sets up resume.
    pub fn fork_from_exchange(&mut self) {
        let session = match self.selected_session() {
            Some(s) => s.clone(),
            None => return,
        };

        let exchange_idx = self.preview_selected_exchange;
        if exchange_idx >= self.preview_exchanges.len() {
            self.status_message = "No exchange selected".to_string();
            return;
        }

        // Find the last line to keep: if user selected a user message,
        // include its assistant response too (next exchange if it exists)
        let target = &self.preview_exchanges[exchange_idx];
        let last_line = if target.role == "user" {
            // Include the following assistant response if present
            if let Some(next) = self.preview_exchanges.get(exchange_idx + 1) {
                if next.role == "assistant" {
                    next.line_index
                } else {
                    target.line_index
                }
            } else {
                target.line_index
            }
        } else {
            target.line_index
        };

        // Read the original JSONL and write truncated copy
        let jsonl_path = &session.jsonl_path;
        let original = match std::fs::read_to_string(jsonl_path) {
            Ok(content) => content,
            Err(e) => {
                self.status_message = format!("Failed to read session: {e}");
                return;
            }
        };

        let lines: Vec<&str> = original.lines().collect();
        if last_line >= lines.len() {
            self.status_message = "Exchange index out of range".to_string();
            return;
        }

        // Create forked JSONL in the same directory with a new UUID
        let new_id = uuid::Uuid::new_v4().to_string();
        let parent = std::path::Path::new(jsonl_path).parent().unwrap();
        let fork_path = parent.join(format!("{new_id}.jsonl"));

        // Write lines 0..=last_line, replacing the session ID in each line
        let mut forked_content = String::new();
        for line in &lines[..=last_line] {
            if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("sessionId".to_string(), serde_json::Value::String(new_id.clone()));
                }
                forked_content.push_str(&serde_json::to_string(&value).unwrap_or_else(|_| line.to_string()));
            } else {
                forked_content.push_str(line);
            }
            forked_content.push('\n');
        }

        if let Err(e) = std::fs::write(&fork_path, &forked_content) {
            self.status_message = format!("Failed to write fork: {e}");
            return;
        }

        let exchange_num = exchange_idx + 1;
        let total = self.preview_exchanges.len();
        self.status_message = format!("Forked from exchange {exchange_num}/{total}");

        self.resume_action = Some(ResumeOptions {
            session_id: new_id,
            cwd: session.cwd.clone(),
            fork: false, // already forked manually, just resume the new file
        });
    }

    /// Delete the currently selected session from the index.
    pub fn delete_selected_session(&mut self) {
        let (session_id, jsonl_path) = match self.selected_session() {
            Some(s) => (s.session_id.clone(), s.jsonl_path.clone()),
            None => return,
        };

        let saved_position = self.selected;

        // Remove the JSONL file from disk
        if !jsonl_path.is_empty() {
            let path = std::path::Path::new(&jsonl_path);
            if path.exists() {
                if let Err(e) = std::fs::remove_file(path) {
                    self.status_message = format!("Failed to delete file: {e}");
                    return;
                }
            }
        }

        if let Err(e) = self.index.remove_session(&session_id) {
            self.status_message = format!("Delete failed: {e}");
            return;
        }
        if let Err(e) = self.index.commit() {
            self.status_message = format!("Commit failed: {e}");
            return;
        }

        // Remove from sessions list
        self.sessions.retain(|s| s.session_id != session_id);
        self.filtered.retain(|&i| i < self.sessions.len());
        // Rebuild filtered to be valid indices
        self.filtered = (0..self.sessions.len()).collect();
        if self.view_mode == ViewMode::SearchResults && !self.search_query.is_empty() {
            self.run_search();
        } else {
            self.rebuild_display_items();
        }

        // Restore position (clamp to valid range, skip headers)
        self.selected = saved_position.min(self.display_items.len().saturating_sub(1));
        if matches!(self.display_items.get(self.selected), Some(DisplayItem::Header(_))) {
            self.skip_to_next_session();
        }

        // Reset preview
        self.expanded_preview = false;
        self.preview_exchanges.clear();
        self.preview_session_id.clear();

        self.status_message = format!("Session {} deleted from index", &session_id[..8]);
        self.confirm_delete = false;
    }
}

fn extract_message_text(value: &serde_json::Value) -> String {
    let content = match value.get("message").and_then(|m| m.get("content")) {
        Some(c) => c,
        None => return String::new(),
    };
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
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
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    input::handle_key(&mut app, key);
                }
                Event::Mouse(mouse) => {
                    input::handle_mouse(&mut app, mouse);
                }
                _ => {}
            }
        }

        if app.should_quit || app.resume_action.is_some() {
            break;
        }
    }

    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();

    Ok(app.resume_action)
}
