use crate::resume::ResumeOptions;
use crate::search::display_name;
use crate::tui::{App, DisplayItem, InputMode, ViewMode};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::time::Instant;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        InputMode::Normal => handle_normal(app, key),
        InputMode::Search => handle_search(app, key),
        InputMode::Rename => handle_rename(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    // Help overlay — any key closes it
    if app.show_help {
        app.show_help = false;
        return;
    }

    // Context menu — keyboard navigation
    if app.context_menu.visible {
        match key.code {
            KeyCode::Up => {
                if app.context_menu.selected > 0 {
                    app.context_menu.selected -= 1;
                }
            }
            KeyCode::Down => {
                if app.context_menu.selected < app.context_menu.items.len() - 1 {
                    app.context_menu.selected += 1;
                }
            }
            KeyCode::Enter => {
                let action = app.context_menu.items[app.context_menu.selected].1;
                app.context_menu.visible = false;
                execute_menu_action(app, action);
            }
            _ => {
                app.context_menu.visible = false;
            }
        }
        return;
    }

    // Delete confirmation mode
    if app.confirm_delete {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.delete_selected_session();
            }
            _ => {
                app.confirm_delete = false;
                app.status_message.clear();
            }
        }
        return;
    }

    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Up => {
            if app.expanded_preview {
                if app.preview_selected_exchange > 0 {
                    app.preview_selected_exchange -= 1;
                }
                scroll_to_selected_exchange(app);
            } else {
                move_up(app);
            }
        }
        KeyCode::Down => {
            if app.expanded_preview {
                if app.preview_selected_exchange < app.preview_exchanges.len().saturating_sub(1) {
                    app.preview_selected_exchange += 1;
                }
                scroll_to_selected_exchange(app);
            } else {
                move_down(app);
            }
        }
        KeyCode::Enter => {
            if let Some(session) = app.selected_session() {
                app.resume_action = Some(ResumeOptions {
                    session_id: session.session_id.clone(),
                    cwd: session.cwd.clone(),
                    fork: false,
                    launch_fresh: false,
                });
            }
        }
        KeyCode::Char('f') => {
            if app.expanded_preview && !app.preview_exchanges.is_empty() {
                // Fork from the selected exchange
                app.fork_from_exchange();
            } else if let Some(session) = app.selected_session() {
                app.resume_action = Some(ResumeOptions {
                    session_id: session.session_id.clone(),
                    cwd: session.cwd.clone(),
                    fork: true,
                    launch_fresh: false,
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
        KeyCode::Char('p') => {
            if app.expanded_preview {
                // Toggle off
                app.expanded_preview = false;
                app.preview_scroll = 0;
            } else {
                // Toggle on — load from disk
                app.load_expanded_preview();
                app.expanded_preview = true;
                app.preview_scroll = 0;
            }
        }
        KeyCode::Char('d') => {
            if app.selected_session().is_some() {
                app.confirm_delete = true;
                app.status_message = "Delete this session from index? (y/n)".to_string();
            }
        }
        KeyCode::Char('h') | KeyCode::Char('?') => {
            app.show_help = true;
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
            if app.expanded_preview {
                app.expanded_preview = false;
                app.preview_scroll = 0;
            } else {
                app.search_query.clear();
                app.filtered = (0..app.sessions.len()).collect();
                app.view_mode = ViewMode::All;
                app.rebuild_display_items();
            }
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
            // Cancel filter: clear query, restore full list, exit input mode
            app.search_query.clear();
            app.run_live_filter();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            // Exit input mode but keep the current live filter applied
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.run_live_filter();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.run_live_filter();
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

fn execute_menu_action(app: &mut App, action: &str) {
    match action {
        "resume" => {
            if let Some(session) = app.selected_session() {
                app.resume_action = Some(ResumeOptions {
                    session_id: session.session_id.clone(),
                    cwd: session.cwd.clone(),
                    fork: false,
                    launch_fresh: false,
                });
            }
        }
        "fork" => {
            if let Some(session) = app.selected_session() {
                app.resume_action = Some(ResumeOptions {
                    session_id: session.session_id.clone(),
                    cwd: session.cwd.clone(),
                    fork: true,
                    launch_fresh: false,
                });
            }
        }
        "rename" => {
            let current_name = app
                .selected_session()
                .map(|s| display_name(s, &app.name_store).to_string())
                .unwrap_or_default();
            app.rename_buffer = current_name;
            app.input_mode = InputMode::Rename;
        }
        "preview" => {
            if app.expanded_preview {
                app.expanded_preview = false;
            } else {
                app.load_expanded_preview();
                app.expanded_preview = true;
            }
        }
        "delete" => {
            app.confirm_delete = true;
            app.status_message = "Delete this session? (y/n)".to_string();
        }
        "fork_exchange" => {
            app.fork_from_exchange();
        }
        _ => {}
    }
}

pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    if app.show_help {
        return;
    }

    let col = mouse.column;
    let row = mouse.row;

    // If context menu is visible, handle clicks on it or dismiss
    if app.context_menu.visible {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let width = 19u16;
                let height = app.context_menu.items.len() as u16 + 2;
                let rx = app.context_menu.render_x;
                let ry = app.context_menu.render_y;
                let menu_rect = Rect::new(rx, ry, width, height);

                if in_rect(col, row, menu_rect) {
                    let inner_row = (row.saturating_sub(ry)).saturating_sub(1) as usize;
                    if inner_row < app.context_menu.items.len() {
                        let action = app.context_menu.items[inner_row].1;
                        app.context_menu.visible = false;
                        execute_menu_action(app, action);
                    }
                } else {
                    app.context_menu.visible = false;
                }
            }
            MouseEventKind::Down(MouseButton::Right) => {
                app.context_menu.visible = false;
            }
            _ => {} // ignore scroll/move while menu is open
        }
        return;
    }

    if app.confirm_delete {
        return;
    }
    if app.input_mode != InputMode::Normal {
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if in_rect(col, row, app.list_area) {
                // Clicking list while expanded collapses preview
                if app.expanded_preview {
                    app.expanded_preview = false;
                }
                let inner_row = (row.saturating_sub(app.list_area.y)).saturating_sub(2) as usize; // -1 border, -1 padding
                if let Some(idx) = row_to_item(app, inner_row) {
                    if matches!(app.display_items.get(idx), Some(DisplayItem::Session(_))) {
                        // Double-click detection: same item within 400ms
                        let now = Instant::now();
                        if app.last_click_idx == Some(idx)
                            && now.duration_since(app.last_click_time).as_millis() < 400
                        {
                            // Double-click — resume session
                            if let Some(session) = app.selected_session() {
                                app.resume_action = Some(ResumeOptions {
                                    session_id: session.session_id.clone(),
                                    cwd: session.cwd.clone(),
                                    fork: false,
                                    launch_fresh: false,
                                });
                            }
                        }
                        app.selected = idx;
                        app.last_click_time = now;
                        app.last_click_idx = Some(idx);
                    }
                }
            } else if in_rect(col, row, app.preview_area) {
                if app.expanded_preview && !app.preview_exchange_lines.is_empty() {
                    // Click to select an exchange in expanded preview
                    let inner_row = (row.saturating_sub(app.preview_area.y)).saturating_sub(1) as usize;
                    let visual_line = inner_row + app.preview_scroll as usize;
                    let mut found = false;
                    for (i, (start, end)) in app.preview_exchange_lines.iter().enumerate() {
                        if visual_line >= *start && visual_line < *end {
                            app.preview_selected_exchange = i;
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        // Clicked below all exchanges — don't toggle
                    }
                } else if app.expanded_preview {
                    app.expanded_preview = false;
                } else {
                    app.load_expanded_preview();
                    app.expanded_preview = true;
                }
            } else if in_rect(col, row, app.search_area) {
                // Clicking search bar activates search mode
                app.input_mode = InputMode::Search;
            } else if in_rect(col, row, app.claude_button_area) {
                // Launch fresh Claude with --dangerously-skip-permissions in cwd
                launch_fresh_claude(app);
            } else if in_rect(col, row, app.tabs_area) {
                // Click on a tab to switch view
                let rel_x = col.saturating_sub(app.tabs_area.x) as usize;
                let tab_titles = [
                    format!(" All ({}) ", app.sessions.len()),
                    " By Project ".to_string(),
                    " By Date ".to_string(),
                    format!(" Search ({}) ", app.filtered.len()),
                ];
                let mut x_pos = 0;
                for (i, title) in tab_titles.iter().enumerate() {
                    let tab_width = title.len() + 1; // +1 for separator
                    if rel_x < x_pos + tab_width {
                        let modes = [
                            ViewMode::All,
                            ViewMode::ByProject,
                            ViewMode::ByDate,
                            ViewMode::SearchResults,
                        ];
                        app.view_mode = modes[i];
                        app.rebuild_display_items();
                        break;
                    }
                    x_pos += tab_width;
                }
            }
        }
        MouseEventKind::ScrollUp => {
            if in_rect(col, row, app.list_area) {
                move_up(app);
            } else if in_rect(col, row, app.preview_area) {
                app.preview_scroll = app.preview_scroll.saturating_sub(3);
            }
        }
        MouseEventKind::ScrollDown => {
            if in_rect(col, row, app.list_area) {
                move_down(app);
            } else if in_rect(col, row, app.preview_area) {
                app.preview_scroll = app.preview_scroll.saturating_add(3);
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            if in_rect(col, row, app.preview_area) && app.expanded_preview {
                // Right-click in expanded preview — show fork menu
                app.context_menu.visible = true;
                app.context_menu.x = col;
                app.context_menu.y = row;
                app.context_menu.selected = 0;
                let ex_num = app.preview_selected_exchange + 1;
                app.context_menu.items = vec![
                    ("  Fork here     ", "fork_exchange"),
                    ("  Resume full   ", "resume"),
                ];
                app.status_message = format!("Fork point: exchange {ex_num}");
            } else if in_rect(col, row, app.list_area) {
                // Select the item under cursor
                let inner_row = (row.saturating_sub(app.list_area.y)).saturating_sub(2) as usize;
                if let Some(idx) = row_to_item(app, inner_row) {
                    if matches!(app.display_items.get(idx), Some(DisplayItem::Session(_))) {
                        app.selected = idx;
                    }
                }
                // Open context menu at click position
                app.context_menu.visible = true;
                app.context_menu.x = col;
                app.context_menu.y = row;
                app.context_menu.selected = 0;
                app.context_menu.items = vec![
                    ("  Resume       ", "resume"),
                    ("  Fork         ", "fork"),
                    ("  Rename       ", "rename"),
                    ("  Preview      ", "preview"),
                    ("  Delete       ", "delete"),
                ];
            }
        }
        _ => {}
    }
}

fn launch_fresh_claude(app: &mut App) {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    app.resume_action = Some(ResumeOptions {
        session_id: String::new(),
        cwd,
        fork: false,
        launch_fresh: true,
    });
}

fn scroll_to_selected_exchange(app: &mut App) {
    let idx = app.preview_selected_exchange;
    if let Some((start, _)) = app.preview_exchange_lines.get(idx) {
        let visible_height = app.preview_area.height.saturating_sub(2) as usize;
        let current_scroll = app.preview_scroll as usize;

        if *start < current_scroll {
            // Selected exchange is above viewport — scroll up to it
            app.preview_scroll = *start as u16;
        } else if *start >= current_scroll + visible_height {
            // Selected exchange is below viewport — scroll down
            app.preview_scroll = start.saturating_sub(2) as u16;
        }
    }
}

fn in_rect(col: u16, row: u16, rect: Rect) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

fn row_to_item(app: &App, visual_row: usize) -> Option<usize> {
    let offset = app.list_state.offset();
    let mut cumulative = 0;
    for i in offset..app.display_items.len() {
        let h = match &app.display_items[i] {
            DisplayItem::Header(_) => 1,
            DisplayItem::Session(_) => 4, // name + project + times + blank line
        };
        if visual_row < cumulative + h {
            return Some(i);
        }
        cumulative += h;
    }
    None
}
