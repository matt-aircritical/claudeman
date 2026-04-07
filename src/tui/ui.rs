use crate::indexer::IndexedSession;
use crate::search::{display_name, format_date, group_by_project};
use crate::tui::{App, DisplayItem, InputMode, ViewMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search bar
            Constraint::Length(1), // tabs
            Constraint::Min(0),   // main content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    draw_search_bar(f, app, chunks[0]);
    draw_tabs(f, app, chunks[1]);
    draw_main(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);
}

fn draw_search_bar(f: &mut Frame, app: &mut App, area: Rect) {
    let (title, content, border_color) = match app.input_mode {
        InputMode::Search => (
            " Search (Enter to confirm, Esc to cancel) ",
            app.search_query.as_str(),
            Color::Yellow,
        ),
        InputMode::Rename => (
            " Rename (Enter to save, Esc to cancel) ",
            app.rename_buffer.as_str(),
            Color::Green,
        ),
        InputMode::Normal => {
            let hint = if app.search_query.is_empty() {
                " / search  n rename "
            } else {
                " Esc to clear search "
            };
            (hint, app.search_query.as_str(), Color::DarkGray)
        }
    };

    let input = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(title),
        );
    f.render_widget(input, area);

    if matches!(app.input_mode, InputMode::Search | InputMode::Rename) {
        let cursor_x = area.x + content.len() as u16 + 1;
        let cursor_y = area.y + 1;
        if cursor_x < area.x + area.width - 1 {
            f.set_cursor_position(Position::new(cursor_x, cursor_y));
        }
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let session_count = app.session_count();
    let search_count = if app.view_mode == ViewMode::SearchResults {
        session_count
    } else {
        app.filtered.len()
    };

    let tab_titles = vec![
        Line::from(format!(" All ({}) ", app.sessions.len())),
        Line::from(" By Project "),
        Line::from(" By Date "),
        Line::from(format!(" Search ({}) ", search_count)),
    ];

    let selected_tab = match app.view_mode {
        ViewMode::All => 0,
        ViewMode::ByProject => 1,
        ViewMode::ByDate => 2,
        ViewMode::SearchResults => 3,
    };

    let tabs = Tabs::new(tab_titles)
        .select(selected_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("│");

    f.render_widget(tabs, area);
}

fn draw_main(f: &mut Frame, app: &mut App, area: Rect) {
    // 2/3 session list, 1/3 preview
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(67), Constraint::Percentage(33)])
        .split(area);

    draw_session_list(f, app, h_chunks[0]);
    draw_preview(f, app, h_chunks[1]);
}

fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect) {
    let inner_width = area.width.saturating_sub(4) as usize; // account for borders + highlight symbol

    let items: Vec<ListItem> = app
        .display_items
        .iter()
        .map(|item| match item {
            DisplayItem::Header(title) => {
                // Group header bar — full width, distinct style
                let bar = "─".repeat(inner_width.saturating_sub(title.len()));
                ListItem::new(Line::from(vec![
                    Span::styled(
                        title.clone(),
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(bar, Style::default().fg(Color::DarkGray)),
                ]))
            }
            DisplayItem::Session(idx) => {
                let session = &app.sessions[*idx];
                let name = display_name(session, &app.name_store).to_string();
                let date = format_date(session.last_activity);
                let msgs = format!("{}msg", session.message_count);

                let max_name = inner_width.saturating_sub(date.len() + msgs.len() + 4);
                let name_display = if name.len() > max_name {
                    format!("{}…", &name[..max_name.saturating_sub(1).max(1)])
                } else {
                    name
                };

                let line1 = Line::from(vec![
                    Span::styled(
                        name_display,
                        Style::default().fg(Color::White),
                    ),
                    Span::raw("  "),
                    Span::styled(date, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(msgs, Style::default().fg(Color::DarkGray)),
                ]);

                let project = if session.project_dir.is_empty() {
                    &session.cwd
                } else {
                    &session.project_dir
                };
                let project_short = shorten_path(project, inner_width.saturating_sub(2));

                let line2 = Line::from(vec![
                    Span::styled(
                        format!("  {}", project_short),
                        Style::default().fg(Color::Cyan),
                    ),
                ]);

                ListItem::new(vec![line1, line2])
            }
        })
        .collect();

    let title = match app.view_mode {
        ViewMode::All => format!(" Sessions ({}) ", app.session_count()),
        ViewMode::ByProject => {
            let sessions_vec: Vec<IndexedSession> = app.filtered.iter().filter_map(|&i| app.sessions.get(i)).cloned().collect();
            let groups = group_by_project(&sessions_vec);
            format!(" By Project · {} groups ", groups.len())
        }
        ViewMode::ByDate => format!(" By Date ({}) ", app.session_count()),
        ViewMode::SearchResults => format!(" Search Results ({}) ", app.session_count()),
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Indexed(236)) // subtle dark gray background
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    let mut list_state = ListState::default();
    if !app.display_items.is_empty() {
        list_state.select(Some(app.selected));
    }

    f.render_stateful_widget(list, area, &mut list_state);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let session = app.selected_session();
    let inner_width = area.width.saturating_sub(3) as usize;

    let content = match session {
        None => Paragraph::new(Line::from(Span::styled(
            "No session selected",
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Preview "),
        ),
        Some(s) => {
            let name = display_name(s, &app.name_store);
            let started = format_date(s.started_at);
            let last = format_date(s.last_activity);

            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    name,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "─".repeat(inner_width),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(vec![
                    Span::styled("ID  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        &s.session_id[..s.session_id.len().min(8)],
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Age ", Style::default().fg(Color::DarkGray)),
                    Span::raw(started),
                    Span::styled(" → ", Style::default().fg(Color::DarkGray)),
                    Span::raw(last),
                ]),
                Line::from(vec![
                    Span::styled("Dir ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        shorten_path(&s.cwd, inner_width.saturating_sub(4)),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Msg ", Style::default().fg(Color::DarkGray)),
                    Span::raw(format!("{}", s.message_count)),
                    if !s.model.is_empty() {
                        Span::styled(
                            format!("  {}", s.model),
                            Style::default().fg(Color::DarkGray),
                        )
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(""),
            ];

            if !s.first_user_message.is_empty() {
                lines.push(Line::from(Span::styled(
                    "YOU",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )));
                let msg = truncate_text(&s.first_user_message, 200);
                for line in wrap_text(&msg, inner_width) {
                    lines.push(Line::from(Span::raw(line)));
                }
                lines.push(Line::from(""));
            }

            if !s.first_assistant_message.is_empty() {
                lines.push(Line::from(Span::styled(
                    "CLAUDE",
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )));
                let msg = truncate_text(&s.first_assistant_message, 150);
                for line in wrap_text(&msg, inner_width) {
                    lines.push(Line::from(Span::raw(line)));
                }
            }

            Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(Span::styled(
                            " Preview ",
                            Style::default().fg(Color::White),
                        )),
                )
                .wrap(Wrap { trim: false })
        }
    };

    f.render_widget(content, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let left = if !app.status_message.is_empty() {
        Span::styled(
            app.status_message.clone(),
            Style::default().fg(Color::Green),
        )
    } else {
        let project_count = group_by_project(&app.sessions).len();
        Span::styled(
            format!(" {} sessions · {} projects", app.sessions.len(), project_count),
            Style::default().fg(Color::Cyan),
        )
    };

    let right_text = match app.input_mode {
        InputMode::Normal => "↑↓ nav  ⏎ resume  f fork  / search  n rename  Tab views  q quit ",
        InputMode::Search => "⏎ confirm  Esc cancel ",
        InputMode::Rename => "⏎ save  Esc cancel ",
    };

    let right = Span::styled(right_text, Style::default().fg(Color::DarkGray));

    let width = area.width as usize;
    let used = left.content.len() + right.content.len();
    let padding = width.saturating_sub(used);

    let bar = Line::from(vec![left, Span::raw(" ".repeat(padding)), right]);

    let status = Paragraph::new(bar).style(
        Style::default()
            .bg(Color::Indexed(235))
            .fg(Color::White),
    );

    f.render_widget(status, area);
}

fn shorten_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    let keep = max_len.saturating_sub(3);
    if keep == 0 {
        return "…".to_string();
    }
    let start = path.len() - keep;
    format!("…{}", &path[start..])
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
