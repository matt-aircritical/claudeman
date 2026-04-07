use crate::resume::ResumeOptions;
use crate::search::display_name;
use crate::tui::{App, DisplayItem, InputMode, ViewMode};
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
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Up => {
            move_up(app);
        }
        KeyCode::Down => {
            move_down(app);
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
        }
        KeyCode::Char('n') => {
            let current_name = app
                .selected_session()
                .map(|s| display_name(s, &app.name_store).to_string())
                .unwrap_or_default();
            app.rename_buffer = current_name;
            app.input_mode = InputMode::Rename;
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
            app.rebuild_display_items();
        }
        KeyCode::Esc => {
            app.search_query.clear();
            app.filtered = (0..app.sessions.len()).collect();
            app.view_mode = ViewMode::All;
            app.rebuild_display_items();
        }
        _ => {}
    }
}

/// Move selection up, skipping header items.
fn move_up(app: &mut App) {
    if app.selected == 0 {
        return;
    }
    let mut next = app.selected - 1;
    // Skip headers
    while next > 0 && matches!(app.display_items.get(next), Some(DisplayItem::Header(_))) {
        next -= 1;
    }
    if matches!(app.display_items.get(next), Some(DisplayItem::Session(_))) {
        app.selected = next;
    }
}

/// Move selection down, skipping header items.
fn move_down(app: &mut App) {
    if app.display_items.is_empty() {
        return;
    }
    let max = app.display_items.len() - 1;
    if app.selected >= max {
        return;
    }
    let mut next = app.selected + 1;
    // Skip headers
    while next < max && matches!(app.display_items.get(next), Some(DisplayItem::Header(_))) {
        next += 1;
    }
    if matches!(app.display_items.get(next), Some(DisplayItem::Session(_))) {
        app.selected = next;
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            app.run_search();
            app.view_mode = ViewMode::SearchResults;
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
                let session_id = session.session_id.clone();
                let name = app.rename_buffer.trim().to_string();
                if !name.is_empty() {
                    app.name_store.set(&session_id, &name);
                    if let Err(e) = app.name_store.save() {
                        app.status_message = format!("Failed to save name: {e}");
                    } else {
                        app.status_message = format!("Renamed to: {name}");
                    }
                }
            }
            app.input_mode = InputMode::Normal;
            app.rename_buffer.clear();
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
