use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem,
        Padding, Paragraph, Row, Table, Tabs, Wrap,
    },
    Frame,
};

use super::app::{AppState, AppTab, InputMode, LogLevel, ProxyStatus, ScanStatus};

// Color Palette
const COLOR_PRIMARY: Color = Color::Cyan;
const COLOR_SUCCESS: Color = Color::Green;
const COLOR_WARNING: Color = Color::Yellow;
const COLOR_ERROR: Color = Color::Red;
const COLOR_DIM: Color = Color::DarkGray;
const COLOR_HIGHLIGHT: Color = Color::LightCyan;
const COLOR_BG: Color = Color::Black;
const COLOR_BORDER: Color = Color::Blue;

pub fn render(f: &mut Frame, state: &AppState) {
    let size = f.area();

    f.render_widget(
        Block::default().style(Style::default().bg(COLOR_BG)),
        size,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(size);

    render_header(f, state, chunks[0]);
    render_content(f, state, chunks[1]);
    render_statusbar(f, state, chunks[2]);

    if state.show_help_popup {
        render_help_popup(f, size);
    }
}

fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(" ◈ ", Style::default().fg(COLOR_PRIMARY)),
        Span::styled(
            "SNI BYPASS",
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" RS-TUI", Style::default().fg(COLOR_DIM)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER))
            .padding(Padding::horizontal(1)),
    );
    f.render_widget(title, chunks[0]);

    let tab_titles = vec![
        " 1:Dashboard ",
        " 2:Scanner ",
        " 3:Results ",
        " 4:Logs ",
        " 5:Help ",
    ];
    let active_idx = match state.active_tab {
        AppTab::Dashboard => 0,
        AppTab::Scanner => 1,
        AppTab::Results => 2,
        AppTab::Logs => 3,
        AppTab::Help => 4,
    };

    let tabs = Tabs::new(
        tab_titles
            .iter()
            .map(|t| Line::from(Span::raw(*t)))
            .collect::<Vec<_>>(),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    )
    .select(active_idx)
    .style(Style::default().fg(COLOR_DIM))
    .highlight_style(
        Style::default()
            .fg(COLOR_PRIMARY)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    );

    f.render_widget(tabs, chunks[1]);
}

fn render_content(f: &mut Frame, state: &AppState, area: Rect) {
    match state.active_tab {
        AppTab::Dashboard => render_dashboard(f, state, area),
        AppTab::Scanner => render_scanner(f, state, area),
        AppTab::Results => render_results(f, state, area),
        AppTab::Logs => render_logs(f, state, area),
        AppTab::Help => render_help(f, area),
    }
}

fn render_dashboard(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(0)])
        .split(chunks[0]);

    render_proxy_config(f, state, left_chunks[0]);
    render_stats(f, state, left_chunks[1]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(chunks[1]);

    render_proxy_status(f, state, right_chunks[0]);
    render_quick_actions(f, state, right_chunks[1]);
}

fn render_proxy_config(f: &mut Frame, state: &AppState, area: Rect) {
    let is_editing = state.input_mode == InputMode::Editing;

    let fields: [(&str, &str, usize); 3] = [
        ("Target Host", &state.input_target, 0),
        ("SNI Host   ", &state.input_sni, 1),
        ("Port       ", &state.input_port, 2),
    ];

    let mut lines = vec![Line::from("")];
    for (label, value, idx) in &fields {
        let is_active = state.active_field == *idx;
        let label_style = Style::default().fg(COLOR_DIM);
        let (prefix, value_style) = if is_active && is_editing {
            (
                "▶ ",
                Style::default()
                    .fg(COLOR_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_active {
            (
                "▶ ",
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", Style::default().fg(Color::White))
        };

        let display_value = if value.is_empty() {
            format!("  {}<empty>", prefix)
        } else if is_active && is_editing {
            format!("  {}{}█", prefix, value)
        } else {
            format!("  {}{}", prefix, value)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {}: ", label), label_style),
            Span::styled(display_value, value_style),
        ]));
    }

    let border_color = if is_editing { COLOR_PRIMARY } else { COLOR_BORDER };

    let para = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(Span::styled(
                    " ⚙ Proxy Configuration ",
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn render_proxy_status(f: &mut Frame, state: &AppState, area: Rect) {
    let (status_text, status_color, icon) = match &state.proxy_status {
        ProxyStatus::Stopped => ("STOPPED", COLOR_DIM, "⏹"),
        ProxyStatus::Starting => ("STARTING...", COLOR_WARNING, "⏳"),
        ProxyStatus::Running => ("RUNNING", COLOR_SUCCESS, "▶"),
        ProxyStatus::Error(_) => ("ERROR", COLOR_ERROR, "✗"),
    };

    let error_msg = if let ProxyStatus::Error(e) = &state.proxy_status {
        e.clone()
    } else {
        String::new()
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Status: ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                format!("{} {} ", icon, status_text),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Listen: ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                format!("0.0.0.0:{}", state.proxy_port),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Target: ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                if state.target_host.is_empty() {
                    "<not set>".to_string()
                } else {
                    state.target_host.clone()
                },
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  SNI:    ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                if state.sni_host.is_empty() {
                    "<not set>".to_string()
                } else {
                    state.sni_host.clone()
                },
                Style::default().fg(COLOR_HIGHLIGHT),
            ),
        ]),
    ];

    if !error_msg.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Error:  ", Style::default().fg(COLOR_ERROR)),
            Span::styled(error_msg, Style::default().fg(COLOR_ERROR)),
        ]));
    }

    let action = match state.proxy_status {
        ProxyStatus::Running => "[s] Stop",
        _ => "[s] Start",
    };

    let para = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(Span::styled(
                    " ◈ Proxy Status ",
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .title_bottom(Span::styled(
                    format!(" {} ", action),
                    Style::default().fg(COLOR_DIM),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn render_stats(f: &mut Frame, state: &AppState, area: Rect) {
    let bytes = format_bytes(state.bytes_transferred);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↕ Transferred : ", Style::default().fg(COLOR_DIM)),
            Span::styled(bytes, Style::default().fg(COLOR_SUCCESS)),
        ]),
        Line::from(vec![
            Span::styled("  ⇌ Active Conns : ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                state.connections_active.to_string(),
                Style::default().fg(COLOR_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ∑ Total Conns  : ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                state.connections_total.to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ⚡ Req/sec      : ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                format!("{:.1}", state.requests_per_sec),
                Style::default().fg(COLOR_WARNING),
            ),
        ]),
    ];

    let para = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(Span::styled(
                    " ◈ Statistics ",
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn render_quick_actions(f: &mut Frame, state: &AppState, area: Rect) {
    let is_termux = state.is_termux;

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  [1-5]  ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Switch tabs", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [e/i]  ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Edit fields", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [s]    ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Start/Stop proxy", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [S]    ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Start SNI scan", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [Tab]  ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Next tab/field", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [?]    ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Help popup", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [q]    ", Style::default().fg(COLOR_ERROR)),
            Span::styled("Quit", Style::default().fg(Color::White)),
        ]),
    ];

    if is_termux {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  📱 ", Style::default()),
            Span::styled(
                "Termux mode active",
                Style::default()
                    .fg(COLOR_WARNING)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "  Use hardware keyboard for best experience",
            Style::default().fg(COLOR_DIM),
        )]));
    }

    let para = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(Span::styled(
                    " ◈ Quick Reference ",
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn render_scanner(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(5),
            Constraint::Min(0),
        ])
        .split(area);

    let is_editing = state.input_mode == InputMode::Editing
        && state.active_tab == AppTab::Scanner;

    let fields: [(&str, &str, usize); 2] = [
        ("Hosts File  ", &state.input_hosts_file, 0),
        ("Concurrency ", &state.input_concurrency, 1),
    ];

    let mut lines = vec![Line::from("")];
    for (label, value, idx) in &fields {
        let is_active = state.active_field == *idx;
        let (prefix, value_style) = if is_active && is_editing {
            (
                "▶ ",
                Style::default()
                    .fg(COLOR_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_active {
            (
                "▶ ",
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", Style::default().fg(Color::White))
        };

        let display = if is_active && is_editing {
            format!("{}{}█", prefix, value)
        } else {
            format!("{}{}", prefix, value)
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}: ", label),
                Style::default().fg(COLOR_DIM),
            ),
            Span::styled(display, value_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  [e] Edit  ", Style::default().fg(COLOR_DIM)),
        Span::styled("[S] Start Scan  ", Style::default().fg(COLOR_SUCCESS)),
        Span::styled("[x] Stop Scan", Style::default().fg(COLOR_ERROR)),
    ]));

    let border_color = if is_editing { COLOR_PRIMARY } else { COLOR_BORDER };

    let config_block = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(Span::styled(
                    " ◈ Scanner Configuration ",
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)),
        );

    f.render_widget(config_block, chunks[0]);

    // Progress gauge
    let (scan_label, gauge_color) = match &state.scan_status {
        ScanStatus::Idle => ("Idle — Press [S] to start", COLOR_DIM),
        ScanStatus::Running => ("Scanning...", COLOR_PRIMARY),
        ScanStatus::Completed => ("Scan Complete!", COLOR_SUCCESS),
        ScanStatus::Error(_) => ("Error", COLOR_ERROR),
    };

    let progress_ratio = state.scan_progress.clamp(0.0, 1.0);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(Span::styled(
                    format!(
                        " ◈ Progress [{}/{}] ",
                        state.scan_done, state.scan_total
                    ),
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
        .gauge_style(
            Style::default()
                .fg(gauge_color)
                .bg(Color::DarkGray),
        )
        .ratio(progress_ratio)
        .label(format!(
            "{} ({:.0}%)",
            scan_label,
            progress_ratio * 100.0
        ));

    f.render_widget(gauge, chunks[1]);

    // Info panel
    let working = state.scan_results.iter().filter(|r| r.is_working).count();
    let total = state.scan_results.len();

    let error_text = if let ScanStatus::Error(e) = &state.scan_status {
        e.clone()
    } else {
        String::new()
    };

    let mut info_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Results so far: ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                format!("{} total, {} working", total, working),
                Style::default().fg(COLOR_SUCCESS),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Best latency:   ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                state
                    .scan_results
                    .iter()
                    .filter(|r| r.is_working)
                    .map(|r| r.latency_ms)
                    .min()
                    .map(|ms| format!("{}ms", ms))
                    .unwrap_or_else(|| "N/A".to_string()),
                Style::default().fg(COLOR_WARNING),
            ),
        ]),
        Line::from(vec![Span::styled(
            "  Press [3] to view results, [u] to use selected SNI",
            Style::default().fg(COLOR_DIM),
        )]),
    ];

    if !error_text.is_empty() {
        info_lines.push(Line::from(Span::styled(
            format!("  Error: {}", error_text),
            Style::default().fg(COLOR_ERROR),
        )));
    }

    let info = Paragraph::new(Text::from(info_lines))
        .block(
            Block::default()
                .title(Span::styled(
                    " ◈ Scanner Info ",
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(info, chunks[2]);
}

fn render_results(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let header_cells = ["#", "Host", "Latency", "Status", "TLS", "HTTP"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        });

    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

    let visible_start = state.result_scroll;

    let rows: Vec<Row> = state
        .scan_results
        .iter()
        .enumerate()
        .skip(visible_start)
        .map(|(idx, result)| {
            let is_selected = idx == state.selected_result;
            let row_style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(COLOR_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD)
            } else if result.is_working {
                Style::default().fg(COLOR_SUCCESS)
            } else {
                Style::default().fg(COLOR_DIM)
            };

            let status_icon = if result.is_working { "✓" } else { "✗" };
            let latency = if result.is_working {
                format!("{}ms", result.latency_ms)
            } else {
                "timeout".to_string()
            };
            let tls = if result.tls_ok { "✓" } else { "✗" };
            let http = if result.http_ok { "✓" } else { "✗" };

            Row::new(vec![
                Cell::from(format!("{:>3}", idx + 1)),
                Cell::from(result.host.clone()),
                Cell::from(latency),
                Cell::from(status_icon),
                Cell::from(tls),
                Cell::from(http),
            ])
            .style(row_style)
        })
        .collect();

    let working_count = state.scan_results.iter().filter(|r| r.is_working).count();
    let title = format!(
        " ◈ Scan Results [{}/{} working] ",
        working_count,
        state.scan_results.len()
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Min(25),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(6),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(Span::styled(
                title,
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    )
    .column_spacing(1);

    f.render_widget(table, chunks[0]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled(" [↑↓/jk] Navigate  ", Style::default().fg(COLOR_DIM)),
        Span::styled(
            "[u] Use selected SNI  ",
            Style::default().fg(COLOR_PRIMARY),
        ),
        Span::styled("[PgUp/PgDn] Page  ", Style::default().fg(COLOR_DIM)),
        Span::styled("[g/G] Top/Bottom", Style::default().fg(COLOR_DIM)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );

    f.render_widget(hint, chunks[1]);
}

fn render_logs(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let visible_height = chunks[0].height.saturating_sub(2) as usize;
    let start = state.log_scroll;

    let items: Vec<ListItem> = state
        .logs
        .iter()
        .skip(start)
        .take(visible_height)
        .map(|entry| {
            let (level_str, level_color) = match entry.level {
                LogLevel::Info => ("[INFO]  ", COLOR_PRIMARY),
                LogLevel::Success => ("[OK]    ", COLOR_SUCCESS),
                LogLevel::Warning => ("[WARN]  ", COLOR_WARNING),
                LogLevel::Error => ("[ERROR] ", COLOR_ERROR),
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", entry.timestamp),
                    Style::default().fg(COLOR_DIM),
                ),
                Span::styled(
                    level_str,
                    Style::default().fg(level_color),
                ),
                Span::styled(
                    &entry.message,
                    Style::default().fg(Color::White),
                ),
            ]))
        })
        .collect();

    let auto_scroll_indicator = if state.auto_scroll_logs {
        "AUTO"
    } else {
        "MANUAL"
    };

    let log_list = List::new(items).block(
        Block::default()
            .title(Span::styled(
                format!(
                    " ◈ Logs [{}/{}] [{}] ",
                    state.log_scroll + 1,
                    state.logs.len(),
                    auto_scroll_indicator
                ),
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );

    f.render_widget(log_list, chunks[0]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled(" [↑↓/jk] Scroll  ", Style::default().fg(COLOR_DIM)),
        Span::styled(
            "[a] Toggle auto-scroll  ",
            Style::default().fg(COLOR_PRIMARY),
        ),
        Span::styled("[g/G] Top/Bottom", Style::default().fg(COLOR_DIM)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );

    f.render_widget(hint, chunks[1]);
}

fn render_help(f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let nav_keys: Vec<(&str, &str)> = vec![
        ("1-5", "Switch to tab"),
        ("Tab / BackTab", "Next/prev tab"),
        ("q / Ctrl+C", "Quit"),
        ("?", "Toggle help popup"),
        ("", ""),
        ("── Navigation", ""),
        ("↑↓ / j k", "Scroll up/down"),
        ("PgUp / PgDn", "Page up/down"),
        ("g / G", "Top / Bottom"),
        ("", ""),
        ("── Editing", ""),
        ("e / i", "Enter edit mode"),
        ("Esc", "Exit edit mode"),
        ("Tab", "Next field"),
        ("BackTab", "Prev field"),
        ("Enter", "Confirm field"),
    ];

    let action_keys: Vec<(&str, &str)> = vec![
        ("── Proxy", ""),
        ("s", "Start/Stop proxy"),
        ("", ""),
        ("── Scanner", ""),
        ("S", "Start scan"),
        ("x", "Stop scan"),
        ("u", "Use selected SNI"),
        ("", ""),
        ("── Logs", ""),
        ("a", "Toggle auto-scroll"),
        ("", ""),
        ("── Dashboard", ""),
        ("n / p", "Next/prev field"),
    ];

    let make_items = |keys: Vec<(&str, &str)>| -> Vec<ListItem<'static>> {
        keys.into_iter()
            .map(|(key, desc)| {
                if key.is_empty() {
                    ListItem::new(Line::from(""))
                } else if desc.is_empty() {
                    ListItem::new(Line::from(vec![Span::styled(
                        format!("  {} ", key),
                        Style::default()
                            .fg(COLOR_DIM)
                            .add_modifier(Modifier::BOLD),
                    )]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("  {:15}", key),
                            Style::default().fg(COLOR_PRIMARY),
                        ),
                        Span::styled(
                            desc.to_string(),
                            Style::default().fg(Color::White),
                        ),
                    ]))
                }
            })
            .collect()
    };

    let nav_list = List::new(make_items(nav_keys)).block(
        Block::default()
            .title(Span::styled(
                " ◈ Navigation Keys ",
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );

    let action_list = List::new(make_items(action_keys)).block(
        Block::default()
            .title(Span::styled(
                " ◈ Action Keys ",
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );

    f.render_widget(nav_list, chunks[0]);
    f.render_widget(action_list, chunks[1]);
}

fn render_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 70, area);

    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  SNI Bypass RS-TUI — Quick Help",
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ── Setup ──",
            Style::default().fg(COLOR_DIM),
        )]),
        Line::from(vec![
            Span::styled("  1. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Go to Dashboard [1]",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Edit target host [e]",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Set SNI (or leave same as target)",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  4. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Press [s] to start proxy",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  5. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Set device proxy to 127.0.0.1:<port>",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ── SNI Scanner ──",
            Style::default().fg(COLOR_DIM),
        )]),
        Line::from(vec![
            Span::styled("  1. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Go to Scanner tab [2]",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Set hosts file path [e]",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Press [S] to start scan",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  4. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "View results in Results tab [3]",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  5. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Press [u] to use selected SNI",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ── Termux ──",
            Style::default().fg(COLOR_DIM),
        )]),
        Line::from(vec![Span::styled(
            "  pkg install rust",
            Style::default().fg(COLOR_WARNING),
        )]),
        Line::from(vec![Span::styled(
            "  cargo build --release",
            Style::default().fg(COLOR_WARNING),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Press [?] to close",
            Style::default().fg(COLOR_DIM),
        )]),
    ];

    let popup = Paragraph::new(Text::from(help_text))
        .block(
            Block::default()
                .title(Span::styled(
                    " ◈ Help [?] ",
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(COLOR_PRIMARY)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(popup, popup_area);
}

fn render_statusbar(f: &mut Frame, state: &AppState, area: Rect) {
    let mode_str = match state.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Editing => "EDIT",
    };
    let mode_color = match state.input_mode {
        InputMode::Normal => COLOR_PRIMARY,
        InputMode::Editing => COLOR_WARNING,
    };

    let proxy_indicator = match &state.proxy_status {
        ProxyStatus::Running => Span::styled(
            " ▶ PROXY ON  ",
            Style::default().fg(COLOR_BG).bg(COLOR_SUCCESS),
        ),
        ProxyStatus::Error(_) => Span::styled(
            " ✗ PROXY ERR ",
            Style::default().fg(COLOR_BG).bg(COLOR_ERROR),
        ),
        _ => Span::styled(
            " ⏹ PROXY OFF ",
            Style::default().fg(COLOR_BG).bg(COLOR_DIM),
        ),
    };

    let scan_indicator = match state.scan_status {
        ScanStatus::Running => Span::styled(
            " ⟳ SCANNING ",
            Style::default().fg(COLOR_BG).bg(COLOR_PRIMARY),
        ),
        ScanStatus::Completed => Span::styled(
            " ✓ SCAN DONE ",
            Style::default().fg(COLOR_BG).bg(COLOR_SUCCESS),
        ),
        _ => Span::styled("", Style::default()),
    };

    let termux_ind = if state.is_termux {
        Span::styled(" 📱 TERMUX ", Style::default().fg(COLOR_WARNING))
    } else {
        Span::styled("", Style::default())
    };

    let status_line = Line::from(vec![
        Span::styled(
            format!(" {} ", mode_str),
            Style::default()
                .fg(COLOR_BG)
                .bg(mode_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        proxy_indicator,
        Span::raw(" "),
        scan_indicator,
        Span::styled(
            "  [q]uit  [?]help  [Tab]navigate",
            Style::default().fg(COLOR_DIM),
        ),
        termux_ind,
    ]);

    let statusbar =
        Paragraph::new(status_line).style(Style::default().bg(COLOR_BG));

    f.render_widget(statusbar, area);
}

// ─────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
