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

use super::app::{
    ActiveField, AppState, AppTab, InputField, InputMode, LogLevel,
    ProxyStatus, ScanStatus,
};

// ─────────────────────────────────────────────
// Color palette
// ─────────────────────────────────────────────

const COLOR_PRIMARY: Color = Color::Cyan;
const COLOR_SUCCESS: Color = Color::Green;
const COLOR_WARNING: Color = Color::Yellow;
const COLOR_ERROR: Color = Color::Red;
const COLOR_DIM: Color = Color::DarkGray;
const COLOR_HIGHLIGHT: Color = Color::LightCyan;
const COLOR_BG: Color = Color::Black;
const COLOR_BORDER: Color = Color::Blue;

// ─────────────────────────────────────────────
// Root render — called every frame
// ─────────────────────────────────────────────

pub fn render(f: &mut Frame, state: &AppState) {
    let size = f.size();

    // Background fill
    f.render_widget(
        Block::default().style(Style::default().bg(COLOR_BG)),
        size,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header + tabs
            Constraint::Min(0),    // content
            Constraint::Length(2), // status bar
        ])
        .split(size);

    render_header(f, state, chunks[0]);
    render_content(f, state, chunks[1]);
    render_statusbar(f, state, chunks[2]);

    // Overlays drawn last
    if state.show_help_popup {
        render_help_popup(f, size);
    }
}

// ─────────────────────────────────────────────
// Header row: logo + tab bar
// ─────────────────────────────────────────────

fn render_header(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(72),
        ])
        .split(area);

    // Logo
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

    // Tabs
    let active_idx = match state.active_tab {
        AppTab::Dashboard => 0,
        AppTab::Scanner => 1,
        AppTab::Results => 2,
        AppTab::Logs => 3,
        AppTab::Help => 4,
    };

    let tabs = Tabs::new(vec![
        Line::from(" 1:Dashboard "),
        Line::from(" 2:Scanner "),
        Line::from(" 3:Results "),
        Line::from(" 4:Logs "),
        Line::from(" 5:Help "),
    ])
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

// ─────────────────────────────────────────────
// Content router
// ─────────────────────────────────────────────

fn render_content(f: &mut Frame, state: &AppState, area: Rect) {
    match state.active_tab {
        AppTab::Dashboard => render_dashboard(f, state, area),
        AppTab::Scanner => render_scanner(f, state, area),
        AppTab::Results => render_results(f, state, area),
        AppTab::Logs => render_logs(f, state, area),
        AppTab::Help => render_help_tab(f, area),
    }
}

// ─────────────────────────────────────────────
// Dashboard tab
// ─────────────────────────────────────────────

fn render_dashboard(f: &mut Frame, state: &AppState, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(0)])
        .split(cols[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(cols[1]);

    render_proxy_config(f, state, left[0]);
    render_stats(f, state, left[1]);
    render_proxy_status(f, state, right[0]);
    render_quick_actions(f, state, right[1]);
}

// ── Proxy config input panel ──────────────────

fn render_proxy_config(f: &mut Frame, state: &AppState, area: Rect) {
    let is_editing = state.input_mode == InputMode::Editing;
    let border_color = if is_editing { COLOR_PRIMARY } else { COLOR_BORDER };

    let fields: &[(&str, &InputField, &ActiveField)] = &[
        ("Target Host", &state.field_target, &ActiveField::Target),
        ("SNI Host   ", &state.field_sni, &ActiveField::Sni),
        ("Port       ", &state.field_port, &ActiveField::Port),
    ];

    let mut lines = vec![Line::from("")];

    for (label, field, field_id) in fields {
        let is_active = &state.active_field == *field_id;

        let (prefix, label_style, value_style) = if is_active && is_editing {
            (
                "▶ ",
                Style::default().fg(COLOR_PRIMARY),
                Style::default()
                    .fg(COLOR_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_active {
            (
                "▶ ",
                Style::default().fg(COLOR_PRIMARY),
                Style::default().fg(COLOR_PRIMARY),
            )
        } else {
            (
                "  ",
                Style::default().fg(COLOR_DIM),
                Style::default().fg(Color::White),
            )
        };

        let display = if is_active && is_editing {
            if field.value.is_empty() {
                format!("{}█", prefix)
            } else {
                format!("{}{}", prefix, field.display_with_cursor())
            }
        } else if field.value.is_empty() {
            format!("{}<empty>", prefix)
        } else {
            format!("{}{}", prefix, field.value)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {}: ", label), label_style),
            Span::styled(display, value_style),
        ]));
    }

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

// ── Proxy status panel ────────────────────────

fn render_proxy_status(f: &mut Frame, state: &AppState, area: Rect) {
    let (status_text, status_color, icon) = match &state.proxy_status {
        ProxyStatus::Stopped => ("STOPPED", COLOR_DIM, "⏹"),
        ProxyStatus::Starting => ("STARTING...", COLOR_WARNING, "⏳"),
        ProxyStatus::Running => ("RUNNING", COLOR_SUCCESS, "▶"),
        ProxyStatus::Error(_) => ("ERROR", COLOR_ERROR, "✗"),
    };

    let error_line = if let ProxyStatus::Error(e) = &state.proxy_status {
        Some(e.clone())
    } else {
        None
    };

    let action_hint = match state.proxy_status {
        ProxyStatus::Running => "[s] Stop proxy",
        _ => "[s] Start proxy",
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
                format!("127.0.0.1:{}", state.proxy_port),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Target: ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                if state.target_host.is_empty() {
                    "<not set>".into()
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
                    "<not set>".into()
                } else {
                    state.sni_host.clone()
                },
                Style::default().fg(COLOR_HIGHLIGHT),
            ),
        ]),
    ];

    if let Some(err) = error_line {
        lines.push(Line::from(vec![
            Span::styled("  Error:  ", Style::default().fg(COLOR_ERROR)),
            Span::styled(err, Style::default().fg(COLOR_ERROR)),
        ]));
    }

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
                    format!(" {} ", action_hint),
                    Style::default().fg(COLOR_DIM),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

// ── Stats panel ───────────────────────────────

fn render_stats(f: &mut Frame, state: &AppState, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  ↕ Transferred : ",
                Style::default().fg(COLOR_DIM),
            ),
            Span::styled(
                format_bytes(state.bytes_transferred),
                Style::default().fg(COLOR_SUCCESS),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  ⇌ Active Conns : ",
                Style::default().fg(COLOR_DIM),
            ),
            Span::styled(
                state.connections_active.to_string(),
                Style::default().fg(COLOR_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  ∑ Total Conns  : ",
                Style::default().fg(COLOR_DIM),
            ),
            Span::styled(
                state.connections_total.to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  ⚡ Req/sec      : ",
                Style::default().fg(COLOR_DIM),
            ),
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

// ── Quick actions panel ───────────────────────

fn render_quick_actions(f: &mut Frame, state: &AppState, area: Rect) {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  [1-5]   ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Switch tabs", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [e/i]   ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Edit fields", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [Esc]   ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Exit edit mode", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [s]     ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Start/Stop proxy", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [S]     ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Start SNI scan", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [u]     ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Use selected SNI", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [?]     ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled("Help popup", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  [q]     ", Style::default().fg(COLOR_ERROR)),
            Span::styled("Quit", Style::default().fg(Color::White)),
        ]),
    ];

    if state.is_termux {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  📱 ", Style::default()),
            Span::styled(
                "Termux mode",
                Style::default()
                    .fg(COLOR_WARNING)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "  Ctrl+V to paste from clipboard",
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

// ─────────────────────────────────────────────
// Scanner tab
// ─────────────────────────────────────────────

fn render_scanner(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // config inputs
            Constraint::Length(5), // progress bar
            Constraint::Min(0),    // info
        ])
        .split(area);

    render_scanner_config(f, state, chunks[0]);
    render_scanner_progress(f, state, chunks[1]);
    render_scanner_info(f, state, chunks[2]);
}

fn render_scanner_config(f: &mut Frame, state: &AppState, area: Rect) {
    let is_editing = state.input_mode == InputMode::Editing
        && state.active_tab == AppTab::Scanner;
    let border_color = if is_editing { COLOR_PRIMARY } else { COLOR_BORDER };

    let fields: &[(&str, &InputField, &ActiveField)] = &[
        (
            "Hosts File  ",
            &state.field_hosts_file,
            &ActiveField::HostsFile,
        ),
        (
            "Concurrency ",
            &state.field_concurrency,
            &ActiveField::Concurrency,
        ),
    ];

    let mut lines = vec![Line::from("")];

    for (label, field, field_id) in fields {
        let is_active = &state.active_field == *field_id
            && state.active_tab == AppTab::Scanner;

        let (prefix, label_style, value_style) = if is_active && is_editing {
            (
                "▶ ",
                Style::default().fg(COLOR_PRIMARY),
                Style::default()
                    .fg(COLOR_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_active {
            (
                "▶ ",
                Style::default().fg(COLOR_PRIMARY),
                Style::default().fg(COLOR_PRIMARY),
            )
        } else {
            (
                "  ",
                Style::default().fg(COLOR_DIM),
                Style::default().fg(Color::White),
            )
        };

        let display = if is_active && is_editing {
            if field.value.is_empty() {
                format!("{}█", prefix)
            } else {
                format!("{}{}", prefix, field.display_with_cursor())
            }
        } else if field.value.is_empty() {
            format!("{}<empty>", prefix)
        } else {
            format!("{}{}", prefix, field.value)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {}: ", label), label_style),
            Span::styled(display, value_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  [e] Edit fields  ", Style::default().fg(COLOR_DIM)),
        Span::styled("[S] Start  ", Style::default().fg(COLOR_SUCCESS)),
        Span::styled("[x] Stop", Style::default().fg(COLOR_ERROR)),
    ]));

    let para = Paragraph::new(Text::from(lines))
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

    f.render_widget(para, area);
}

fn render_scanner_progress(f: &mut Frame, state: &AppState, area: Rect) {
    let (label, gauge_color) = match &state.scan_status {
        ScanStatus::Idle => ("Idle — press [S] to start", COLOR_DIM),
        ScanStatus::Running => ("Scanning...", COLOR_PRIMARY),
        ScanStatus::Completed => ("Complete!", COLOR_SUCCESS),
        ScanStatus::Error(_) => ("Error", COLOR_ERROR),
    };

    let ratio = state.scan_progress.clamp(0.0, 1.0);

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
        .gauge_style(Style::default().fg(gauge_color).bg(Color::DarkGray))
        .ratio(ratio)
        .label(format!("{} ({:.0}%)", label, ratio * 100.0));

    f.render_widget(gauge, area);
}

fn render_scanner_info(f: &mut Frame, state: &AppState, area: Rect) {
    let working = state.scan_results.iter().filter(|r| r.is_working).count();
    let total = state.scan_results.len();

    let best_latency = state
        .scan_results
        .iter()
        .filter(|r| r.is_working)
        .map(|r| r.latency_ms)
        .min()
        .map(|ms| format!("{}ms", ms))
        .unwrap_or_else(|| "N/A".to_string());

    let error_text = if let ScanStatus::Error(e) = &state.scan_status {
        Some(e.clone())
    } else {
        None
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Results : ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                format!("{} scanned, {} working", total, working),
                Style::default().fg(COLOR_SUCCESS),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Best    : ",
                Style::default().fg(COLOR_DIM),
            ),
            Span::styled(
                best_latency,
                Style::default().fg(COLOR_WARNING),
            ),
        ]),
        Line::from(vec![Span::styled(
            "  [3] View results   [u] Use selected as SNI",
            Style::default().fg(COLOR_DIM),
        )]),
    ];

    if let Some(err) = error_text {
        lines.push(Line::from(Span::styled(
            format!("  Error: {}", err),
            Style::default().fg(COLOR_ERROR),
        )));
    }

    let para = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(Span::styled(
                    " ◈ Info ",
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

// ─────────────────────────────────────────────
// Results tab
// ─────────────────────────────────────────────

fn render_results(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let header = Row::new(
        ["#", "Host", "Latency", "Status", "TLS", "HTTP"]
            .iter()
            .map(|h| {
                Cell::from(*h).style(
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
            }),
    )
    .style(Style::default().bg(Color::DarkGray))
    .height(1);

    let visible_height = chunks[0].height.saturating_sub(3) as usize;
    let start = state.result_scroll;

    let rows: Vec<Row> = state
        .scan_results
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_height)
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

            let latency = if result.is_working {
                format!("{}ms", result.latency_ms)
            } else {
                "timeout".into()
            };

            Row::new(vec![
                Cell::from(format!("{:>3}", idx + 1)),
                Cell::from(result.host.clone()),
                Cell::from(latency),
                Cell::from(if result.is_working { "✓" } else { "✗" }),
                Cell::from(if result.tls_ok { "✓" } else { "✗" }),
                Cell::from(if result.http_ok { "✓" } else { "✗" }),
            ])
            .style(row_style)
        })
        .collect();

    let working = state.scan_results.iter().filter(|r| r.is_working).count();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Min(28),
            Constraint::Length(9),
            Constraint::Length(8),
            Constraint::Length(5),
            Constraint::Length(5),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(Span::styled(
                format!(
                    " ◈ Scan Results [{}/{} working] ",
                    working,
                    state.scan_results.len()
                ),
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

    // Hint bar
    let hint = Paragraph::new(Line::from(vec![
        Span::styled(" [↑↓/jk] ", Style::default().fg(COLOR_DIM)),
        Span::styled("Navigate  ", Style::default().fg(Color::White)),
        Span::styled("[u] ", Style::default().fg(COLOR_PRIMARY)),
        Span::styled("Use SNI  ", Style::default().fg(Color::White)),
        Span::styled("[PgUp/Dn] ", Style::default().fg(COLOR_DIM)),
        Span::styled("Page  ", Style::default().fg(Color::White)),
        Span::styled("[g/G] ", Style::default().fg(COLOR_DIM)),
        Span::styled("Top/Bottom", Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );

    f.render_widget(hint, chunks[1]);
}

// ─────────────────────────────────────────────
// Logs tab
// ─────────────────────────────────────────────

fn render_logs(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let visible_height = chunks[0].height.saturating_sub(2) as usize;

    let items: Vec<ListItem> = state
        .logs
        .iter()
        .skip(state.log_scroll)
        .take(visible_height)
        .map(|entry| {
            let (tag, color) = match entry.level {
                LogLevel::Info => ("[INFO] ", COLOR_PRIMARY),
                LogLevel::Success => ("[OK]   ", COLOR_SUCCESS),
                LogLevel::Warning => ("[WARN] ", COLOR_WARNING),
                LogLevel::Error => ("[ERR]  ", COLOR_ERROR),
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", entry.timestamp),
                    Style::default().fg(COLOR_DIM),
                ),
                Span::styled(tag, Style::default().fg(color)),
                Span::styled(
                    entry.message.clone(),
                    Style::default().fg(Color::White),
                ),
            ]))
        })
        .collect();

    let scroll_indicator = if state.auto_scroll_logs {
        "AUTO-SCROLL"
    } else {
        "MANUAL"
    };

    let log_list = List::new(items).block(
        Block::default()
            .title(Span::styled(
                format!(
                    " ◈ Logs [{}/{}] {} ",
                    state.log_scroll.saturating_add(1),
                    state.logs.len(),
                    scroll_indicator,
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
        Span::styled(" [↑↓/jk] ", Style::default().fg(COLOR_DIM)),
        Span::styled("Scroll  ", Style::default().fg(Color::White)),
        Span::styled("[a] ", Style::default().fg(COLOR_PRIMARY)),
        Span::styled("Toggle auto-scroll  ", Style::default().fg(Color::White)),
        Span::styled("[g/G] ", Style::default().fg(COLOR_DIM)),
        Span::styled("Top/Bottom", Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );

    f.render_widget(hint, chunks[1]);
}

// ─────────────────────────────────────────────
// Help tab (full keybinding reference)
// ─────────────────────────────────────────────

fn render_help_tab(f: &mut Frame, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Left column — navigation
    let nav: Vec<(&str, &str)> = vec![
        ("── Global", ""),
        ("1 – 5", "Switch tab"),
        ("Tab / BackTab", "Next / prev tab"),
        ("q / Ctrl+C", "Quit"),
        ("?", "Toggle help popup"),
        ("Esc", "Close popup / exit edit"),
        ("", ""),
        ("── Normal mode", ""),
        ("e / i", "Enter edit mode"),
        ("n / p", "Next / prev field"),
        ("s", "Start / Stop proxy"),
        ("S", "Start scan"),
        ("x", "Stop scan"),
        ("u", "Use selected SNI"),
        ("a", "Toggle log auto-scroll"),
        ("", ""),
        ("── Scrolling", ""),
        ("↑↓ / j k", "Up / Down"),
        ("PgUp / PgDn", "Page up / down"),
        ("g / G", "Top / Bottom"),
    ];

    // Right column — edit mode
    let edit: Vec<(&str, &str)> = vec![
        ("── Edit mode keys", ""),
        ("← →", "Move cursor"),
        ("Home / End", "Line start / end"),
        ("Ctrl+A / E", "Line start / end"),
        ("Backspace", "Delete backward"),
        ("Delete", "Delete forward"),
        ("Ctrl+W", "Delete word backward"),
        ("Ctrl+U", "Delete to line start"),
        ("Ctrl+K", "Delete to line end"),
        ("", ""),
        ("── Clipboard", ""),
        ("Ctrl+V / Y", "Paste from clipboard"),
        ("Ctrl+C", "Copy field to clipboard"),
        ("", ""),
        ("── Termux clipboard", ""),
        ("Ctrl+V", "termux-clipboard-get"),
        ("Ctrl+C", "termux-clipboard-set"),
        ("", ""),
        ("── Confirm / navigate", ""),
        ("Enter / Tab", "Next field"),
        ("BackTab", "Prev field"),
        ("Esc", "Exit edit mode"),
    ];

    f.render_widget(make_key_list(nav, " ◈ Navigation & Actions "), cols[0]);
    f.render_widget(make_key_list(edit, " ◈ Edit Mode & Clipboard "), cols[1]);
}

fn make_key_list(keys: Vec<(&str, &str)>, title: &str) -> List<'static> {
    let items: Vec<ListItem> = keys
        .into_iter()
        .map(|(key, desc)| {
            if key.is_empty() {
                ListItem::new(Line::from(""))
            } else if desc.is_empty() {
                // Section header
                ListItem::new(Line::from(vec![Span::styled(
                    format!("  {} ", key),
                    Style::default()
                        .fg(COLOR_DIM)
                        .add_modifier(Modifier::BOLD),
                )]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:16}", key),
                        Style::default().fg(COLOR_PRIMARY),
                    ),
                    Span::styled(
                        desc.to_string(),
                        Style::default().fg(Color::White),
                    ),
                ]))
            }
        })
        .collect();

    List::new(items).block(
        Block::default()
            .title(Span::styled(
                format!(" {} ", title.trim()),
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    )
}

// ─────────────────────────────────────────────
// Help popup overlay
// ─────────────────────────────────────────────

fn render_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(62, 72, area);
    f.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  SNI Bypass RS-TUI — Quick Start",
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ── Proxy Setup ──",
            Style::default().fg(COLOR_DIM),
        )]),
        Line::from(vec![
            Span::styled("  1. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Dashboard [1] → [e] to edit",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Enter Target Host",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Enter SNI Host (or leave blank = same as target)",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  4. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "[Esc] then [s] to start proxy",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  5. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Set device proxy → 127.0.0.1:<port>",
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
                "Scanner [2] → [e] → set hosts file",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "[Esc] then [S] to scan",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(COLOR_PRIMARY)),
            Span::styled(
                "Results [3] → [↑↓] → [u] use as SNI",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ── Clipboard ──",
            Style::default().fg(COLOR_DIM),
        )]),
        Line::from(vec![Span::styled(
            "  Ctrl+V paste   Ctrl+C copy field",
            Style::default().fg(COLOR_WARNING),
        )]),
        Line::from(vec![Span::styled(
            "  Termux: termux-clipboard-get/set used automatically",
            Style::default().fg(COLOR_DIM),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  [?] or [Esc] to close this popup",
            Style::default().fg(COLOR_DIM),
        )]),
    ];

    let popup = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(Span::styled(
                    " ◈ Help ",
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

// ─────────────────────────────────────────────
// Status bar
// ─────────────────────────────────────────────

fn render_statusbar(f: &mut Frame, state: &AppState, area: Rect) {
    let (mode_str, mode_color) = match state.input_mode {
        InputMode::Normal => ("NORMAL", COLOR_PRIMARY),
        InputMode::Editing => ("EDIT  ", COLOR_WARNING),
    };

    let proxy_span = match &state.proxy_status {
        ProxyStatus::Running => Span::styled(
            " ▶ PROXY ON  ",
            Style::default().fg(COLOR_BG).bg(COLOR_SUCCESS),
        ),
        ProxyStatus::Starting => Span::styled(
            " ⏳ STARTING  ",
            Style::default().fg(COLOR_BG).bg(COLOR_WARNING),
        ),
        ProxyStatus::Error(_) => Span::styled(
            " ✗ PROXY ERR ",
            Style::default().fg(COLOR_BG).bg(COLOR_ERROR),
        ),
        ProxyStatus::Stopped => Span::styled(
            " ⏹ PROXY OFF ",
            Style::default().fg(COLOR_BG).bg(COLOR_DIM),
        ),
    };

    let scan_span = match state.scan_status {
        ScanStatus::Running => Span::styled(
            " ⟳ SCANNING ",
            Style::default().fg(COLOR_BG).bg(COLOR_PRIMARY),
        ),
        ScanStatus::Completed => Span::styled(
            " ✓ SCAN DONE ",
            Style::default().fg(COLOR_BG).bg(COLOR_SUCCESS),
        ),
        _ => Span::raw(""),
    };

    let termux_span = if state.is_termux {
        Span::styled(" 📱 TERMUX ", Style::default().fg(COLOR_WARNING))
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", mode_str),
            Style::default()
                .fg(COLOR_BG)
                .bg(mode_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        proxy_span,
        Span::raw(" "),
        scan_span,
        Span::styled(
            "  [q]uit  [?]help  [Tab]navigate  [e]dit",
            Style::default().fg(COLOR_DIM),
        ),
        termux_span,
    ]);

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(COLOR_BG)),
        area,
    );
}

// ─────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vert = Layout::default()
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
        .split(vert[1])[1]
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
