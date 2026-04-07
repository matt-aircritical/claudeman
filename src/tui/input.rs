use crate::resume::ResumeOptions;
use crate::search::display_name;
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
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Up => {
            if app.selected > 0 {
                app.selected -= 1;
            }
        }
        KeyCode::Down => {
            if !app.filtered.is_empty() && app.selected < app.filtered.len() - 1 {
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
        }
        KeyCode::Esc => {
            app.search_query.clear();
            app.filtered = (0..app.sessions.len()).collect();
            app.selected = 0;
            app.view_mode = ViewMode::All;
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
