use crate::search::{display_name, format_date, group_by_project};
use crate::tui::{App, InputMode, ViewMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Outer vertical layout: search bar, tabs, main content, status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search bar
            Constraint::Length(1), // tabs
            Constraint::Min(0),    // main content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    draw_search_bar(f, app, chunks[0]);
    draw_tabs(f, app, chunks[1]);
    draw_main(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);
}

fn draw_search_bar(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let (title, content) = match app.input_mode {
        InputMode::Search => ("Search (Enter to confirm, Esc to cancel)", app.search_query.as_str()),
        InputMode::Rename => ("Rename (Enter to save, Esc to cancel)", app.rename_buffer.as_str()),
        InputMode::Normal => ("/ to search  n to rename", app.search_query.as_str()),
    };

    let style = match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::DarkGray),
        _ => Style::default().fg(Color::Yellow),
    };

    let input = Paragraph::new(content)
        .style(style)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(input, area);

    // Show cursor when typing
    if matches!(app.input_mode, InputMode::Search | InputMode::Rename) {
        let cursor_x = area.x + content.len() as u16 + 1;
        let cursor_y = area.y + 1;
        if cursor_x < area.x + area.width - 1 {
            f.set_cursor_position(Position::new(cursor_x, cursor_y));
        }
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let total = app.sessions.len();
    let search_count = if app.view_mode == ViewMode::SearchResults {
        app.filtered.len()
    } else {
        0
    };

    let project_count = {
        let groups = group_by_project(&app.sessions);
        groups.len()
    };

    let tab_titles = vec![
        Line::from(format!("All ({})", total)),
        Line::from("By Project"),
        Line::from("By Date"),
        Line::from(format!("Search ({})", search_count)),
    ];

    let selected_tab = match app.view_mode {
        ViewMode::All => 0,
        ViewMode::ByProject => 1,
        ViewMode::ByDate => 2,
        ViewMode::SearchResults => 3,
    };

    let tabs = Tabs::new(tab_titles)
        .select(selected_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|");

    // Suppress unused variable warning
    let _ = project_count;

    f.render_widget(tabs, area);
}

fn draw_main(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    // Horizontal split: 45% list, 55% preview
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    draw_session_list(f, app, h_chunks[0]);
    draw_preview(f, app, h_chunks[1]);
}

fn draw_session_list(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .filter_map(|&idx| app.sessions.get(idx))
        .map(|session| {
            let name = display_name(session, &app.name_store).to_string();
            let date = format_date(session.last_activity);
            // Truncate name to fit
            let max_name = area.width.saturating_sub(12) as usize;
            let name_display = if name.len() > max_name {
                format!("{}…", &name[..max_name.saturating_sub(1)])
            } else {
                name
            };

            let line1 = Line::from(vec![
                Span::styled(name_display, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(date, Style::default().fg(Color::DarkGray)),
            ]);

            let project = if session.project_dir.is_empty() {
                session.cwd.clone()
            } else {
                session.project_dir.clone()
            };
            let project_short = shorten_path(&project, area.width.saturating_sub(8) as usize);
            let msg_info = format!(" {}msg", session.message_count);

            let line2 = Line::from(vec![
                Span::styled(project_short, Style::default().fg(Color::Cyan)),
                Span::styled(msg_info, Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(vec![line1, line2])
        })
        .collect();

    let title = match app.view_mode {
        ViewMode::All => format!("Sessions ({})", app.filtered.len()),
        ViewMode::ByProject => format!("By Project ({})", app.filtered.len()),
        ViewMode::ByDate => format!("By Date ({})", app.filtered.len()),
        ViewMode::SearchResults => format!("Results ({})", app.filtered.len()),
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    if !app.filtered.is_empty() {
        list_state.select(Some(app.selected));
    }

    f.render_stateful_widget(list, area, &mut list_state);
}

fn draw_preview(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let session = app.selected_session();

    let content = match session {
        None => Paragraph::new("No session selected")
            .block(Block::default().borders(Borders::ALL).title("Preview"))
            .style(Style::default().fg(Color::DarkGray)),
        Some(s) => {
            let name = display_name(s, &app.name_store);
            let started = format_date(s.started_at);
            let last = format_date(s.last_activity);

            let mut lines: Vec<Line> = vec![
                Line::from(vec![
                    Span::styled("ID:      ", Style::default().fg(Color::DarkGray)),
                    Span::raw(&s.session_id[..s.session_id.len().min(16)]),
                ]),
                Line::from(vec![
                    Span::styled("Name:    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("Started: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(started),
                ]),
                Line::from(vec![
                    Span::styled("Active:  ", Style::default().fg(Color::DarkGray)),
                    Span::raw(last),
                ]),
                Line::from(vec![
                    Span::styled("CWD:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(shorten_path(&s.cwd, area.width.saturating_sub(10) as usize), Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Model:   ", Style::default().fg(Color::DarkGray)),
                    Span::raw(&s.model),
                ]),
                Line::from(vec![
                    Span::styled("Messages:", Style::default().fg(Color::DarkGray)),
                    Span::raw(format!(" {}", s.message_count)),
                ]),
                Line::from(""),
            ];

            if !s.first_user_message.is_empty() {
                lines.push(Line::from(Span::styled(
                    "First message:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                let msg = truncate_text(&s.first_user_message, 300);
                for line in wrap_text(&msg, area.width.saturating_sub(4) as usize) {
                    lines.push(Line::from(Span::raw(line)));
                }
                lines.push(Line::from(""));
            }

            if !s.first_assistant_message.is_empty() {
                lines.push(Line::from(Span::styled(
                    "First response:",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )));
                let msg = truncate_text(&s.first_assistant_message, 200);
                for line in wrap_text(&msg, area.width.saturating_sub(4) as usize) {
                    lines.push(Line::from(Span::raw(line)));
                }
            }

            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Preview"))
                .wrap(Wrap { trim: false })
        }
    };

    f.render_widget(content, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let left = if !app.status_message.is_empty() {
        app.status_message.clone()
    } else {
        let project_count = group_by_project(&app.sessions).len();
        format!("{} sessions | {} projects", app.sessions.len(), project_count)
    };

    let right = match app.input_mode {
        InputMode::Normal => "↑↓ navigate  Enter resume  f fork  / search  n rename  r reindex  Tab views  q quit",
        InputMode::Search => "Enter confirm  Esc cancel  Type to filter",
        InputMode::Rename => "Enter save  Esc cancel  Type new name",
    };

    let width = area.width as usize;
    let right_len = right.len();
    let left_max = width.saturating_sub(right_len + 2);
    let left_short = if left.len() > left_max {
        &left[..left_max]
    } else {
        &left
    };

    let padding = width.saturating_sub(left_short.len() + right_len);
    let status_text = format!("{}{}{}", left_short, " ".repeat(padding), right);

    let status = Paragraph::new(status_text).style(
        Style::default()
            .bg(Color::DarkGray)
            .fg(Color::White),
    );

    f.render_widget(status, area);
}

fn shorten_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    // Try to keep the end of the path
    let keep = max_len.saturating_sub(3);
    if keep == 0 {
        return "...".to_string();
    }
    let start = path.len() - keep;
    format!("...{}", &path[start..])
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        format!("{}…", &text[..max_chars.saturating_sub(1)])
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            if current.is_empty() {
                current.push_str(word);
            } else if current.len() + 1 + word.len() <= width {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(current.clone());
                current = word.to_string();
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    lines
}
