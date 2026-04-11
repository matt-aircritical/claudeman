use crate::indexer::IndexedSession;
use crate::search::{display_name, format_date, group_by_project};
use crate::tui::{App, DisplayItem, InputMode, ViewMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Tabs, Wrap},
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

    app.search_area = chunks[0];
    app.tabs_area = chunks[1];

    draw_search_bar(f, app, chunks[0]);
    draw_tabs(f, app, chunks[1]);
    draw_main(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);

    // Context menu overlay
    if app.context_menu.visible {
        draw_context_menu(f, app);
    }

    // Help overlay
    if app.show_help {
        draw_help(f, area);
    }
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

fn draw_tabs(f: &mut Frame, app: &mut App, area: Rect) {
    let session_count = app.session_count();
    let search_count = if app.view_mode == ViewMode::SearchResults {
        session_count
    } else {
        app.filtered.len()
    };

    // Reserve space on the right for the Claude button
    let button_label = " [ ⚡ Claude ] ";
    let button_width = button_label.chars().count() as u16;
    let tabs_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(button_width)])
        .split(area);

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

    f.render_widget(tabs, tabs_chunks[0]);

    // Store the button area for click handling
    app.claude_button_area = tabs_chunks[1];

    let button = Paragraph::new(Line::from(Span::styled(
        button_label,
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    f.render_widget(button, tabs_chunks[1]);
}

fn draw_main(f: &mut Frame, app: &mut App, area: Rect) {
    // 2/3 session list, 1/3 preview
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(67), Constraint::Percentage(33)])
        .split(area);

    app.list_area = h_chunks[0];
    app.preview_area = h_chunks[1];

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
                let updated = format_date(session.last_activity);
                let created = format_date(session.started_at);
                let msgs = format!("{}msg", session.message_count);

                let max_name = inner_width.saturating_sub(updated.len() + msgs.len() + 4);
                let name_display = if name.chars().count() > max_name {
                    let truncated: String = name.chars().take(max_name.saturating_sub(1).max(1)).collect();
                    format!("{truncated}…")
                } else {
                    name
                };

                let line1 = Line::from(vec![
                    Span::styled(
                        name_display,
                        Style::default().fg(Color::White),
                    ),
                    Span::raw("  "),
                    Span::styled(updated, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(msgs, Style::default().fg(Color::DarkGray)),
                ]);

                let project = if session.project_dir.is_empty() {
                    &session.cwd
                } else {
                    &session.project_dir
                };
                let project_short = shorten_path(project, inner_width.saturating_sub(2));

                let time_info = format!("created: {}  updated: {}", created, format_date(session.last_activity));
                let max_time = inner_width.saturating_sub(4);
                let time_display: String = if time_info.chars().count() > max_time {
                    time_info.chars().take(max_time).collect()
                } else {
                    time_info
                };

                let line2 = Line::from(vec![
                    Span::styled(
                        format!("  {}", project_short),
                        Style::default().fg(Color::Cyan),
                    ),
                ]);

                let line3 = Line::from(vec![
                    Span::styled(
                        format!("  {}", time_display),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);

                ListItem::new(vec![line1, line2, line3, Line::raw("")])
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
                .padding(Padding::new(0, 0, 1, 0))
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
        .highlight_symbol("▸ ")
        .scroll_padding(3);

    if !app.display_items.is_empty() {
        app.list_state.select(Some(app.selected));
    }

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_preview(f: &mut Frame, app: &mut App, area: Rect) {
    let session = app.selected_session().cloned();
    let inner_width = area.width.saturating_sub(3) as usize;

    let content = match session.as_ref() {
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

            // Expanded mode: show all exchanges from disk
            if app.expanded_preview
                && !app.preview_exchanges.is_empty()
                && app.preview_session_id == s.session_id
            {
                lines.push(Line::from(Span::styled(
                    format!("─── Full Conversation ({} exchanges) ───", app.preview_exchanges.len()),
                    Style::default().fg(Color::Yellow),
                )));
                lines.push(Line::from(""));

                app.preview_exchange_lines.clear();
                for (i, exchange) in app.preview_exchanges.iter().enumerate() {
                    let start_line = lines.len();
                    let is_selected = i == app.preview_selected_exchange;
                    let (label, color) = if exchange.role == "user" {
                        (format!("YOU [{}]", i + 1), Color::Green)
                    } else {
                        (format!("CLAUDE [{}]", i + 1), Color::Blue)
                    };

                    if is_selected {
                        // Bright highlight bar for the selected exchange
                        let marker = format!("▸ {label}  ◄ fork point");
                        lines.push(Line::from(Span::styled(
                            marker,
                            Style::default()
                                .fg(Color::Yellow)
                                .bg(Color::Indexed(236))
                                .add_modifier(Modifier::BOLD),
                        )));
                    } else {
                        lines.push(Line::from(Span::styled(
                            format!("  {label}"),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        )));
                    }

                    let text = truncate_text(&exchange.text, 500);
                    for line in wrap_text(&text, inner_width) {
                        if is_selected {
                            lines.push(Line::from(Span::styled(
                                line,
                                Style::default().bg(Color::Indexed(236)),
                            )));
                        } else {
                            lines.push(Line::from(Span::raw(line)));
                        }
                    }
                    lines.push(Line::from(""));
                    app.preview_exchange_lines.push((start_line, lines.len()));
                }
            } else {
                // Compact mode: just first exchange
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

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "press p for full conversation",
                    Style::default().fg(Color::DarkGray),
                )));
            }

            let title = if app.expanded_preview {
                " Preview (expanded · ↑↓ scroll · Esc close) "
            } else {
                " Preview "
            };

            Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(if app.expanded_preview {
                            Color::Yellow
                        } else {
                            Color::DarkGray
                        }))
                        .title(Span::styled(title, Style::default().fg(Color::White))),
                )
                .wrap(Wrap { trim: false })
                .scroll((app.preview_scroll, 0))
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

    let right_text = if app.confirm_delete {
        "y delete  n cancel "
    } else if app.expanded_preview {
        "↑↓ scroll  Esc close  p toggle "
    } else {
        match app.input_mode {
            InputMode::Normal => "↑↓ nav  ⏎ resume  f fork  p preview  d delete  / search  n name  q quit ",
            InputMode::Search => "⏎ confirm  Esc cancel ",
            InputMode::Rename => "⏎ save  Esc cancel ",
        }
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

fn draw_help(f: &mut Frame, area: Rect) {
    // Center the help popup
    let width = 68.min(area.width.saturating_sub(4));
    let height = 30.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    // Clear the area underneath the popup
    f.render_widget(Clear, popup_area);

    let help_lines = vec![
        Line::from(Span::styled(
            " ClaudeMan — Session Manager ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " NAVIGATION",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("   ↑ ↓          ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Navigate sessions / scroll preview", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("   Tab          ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Cycle views: All → Project → Date → Search", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("   Esc          ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Close preview / Clear search / Go back", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " ACTIONS",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("   Enter        ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("Resume selected session", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("   f            ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Fork session (resume as new branch)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("   p            ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Toggle expanded preview (load from disk)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("   d            ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Delete session from index (y to confirm)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " EDITING",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("   /            ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Search sessions (full-text across all content)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("   n            ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Rename selected session", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " OTHER",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("   r            ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Force re-index all sessions", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("   h  ?         ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Show this help", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("   q            ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("Quit", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Sessions are auto-indexed on startup.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            " Config: ~/.config/claudeman/config.toml",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "              Press any key to close",
            Style::default().fg(Color::Cyan),
        )),
    ];

    let help = Paragraph::new(help_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Indexed(233))),
    );

    f.render_widget(help, popup_area);
}

fn draw_context_menu(f: &mut Frame, app: &mut App) {
    let width = 19u16; // matches label width + borders
    let height = app.context_menu.items.len() as u16 + 2; // +2 for borders
    let frame_area = f.area();

    // Position near click, but clamp to stay in frame
    let x = app.context_menu.x.min(frame_area.width.saturating_sub(width));
    let y = app.context_menu.y.min(frame_area.height.saturating_sub(height));
    app.context_menu.render_x = x;
    app.context_menu.render_y = y;
    let area = Rect::new(x, y, width, height);

    let selected = app.context_menu.selected;
    let items: Vec<Line> = app
        .context_menu
        .items
        .iter()
        .enumerate()
        .map(|(i, (label, _))| {
            if i == selected {
                Line::from(Span::styled(
                    *label,
                    Style::default().bg(Color::Cyan).fg(Color::Black),
                ))
            } else {
                Line::from(Span::styled(
                    *label,
                    Style::default().fg(Color::White),
                ))
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Indexed(236)));

    let paragraph = Paragraph::new(items).block(block);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn shorten_path(path: &str, max_len: usize) -> String {
    let char_count = path.chars().count();
    if char_count <= max_len {
        return path.to_string();
    }
    let keep = max_len.saturating_sub(1);
    if keep == 0 {
        return "…".to_string();
    }
    let skip = char_count - keep;
    let shortened: String = path.chars().skip(skip).collect();
    format!("…{shortened}")
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{truncated}…")
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
