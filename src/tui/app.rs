use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;

use super::ui;
use super::events::{AppEvent, EventHandler};
use crate::scanner::{SniScanner, ScanResult};
use crate::bypass::ProxyServer;

#[derive(Debug, Clone, PartialEq)]
pub enum AppTab {
    Dashboard,
    Scanner,
    Results,
    Logs,
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProxyStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScanStatus {
    Idle,
    Running,
    Completed,
    Error(String),
}

pub struct AppState {
    // Navigation
    pub active_tab: AppTab,
    pub input_mode: InputMode,

    // Proxy config
    pub target_host: String,
    pub sni_host: String,
    pub proxy_port: u16,
    pub proxy_status: ProxyStatus,

    // Input fields
    pub input_target: String,
    pub input_sni: String,
    pub input_port: String,
    pub input_hosts_file: String,
    pub input_concurrency: String,
    pub active_field: usize,

    // Scanner
    pub scan_status: ScanStatus,
    pub scan_results: Vec<ScanResult>,
    pub scan_progress: f64,
    pub scan_total: usize,
    pub scan_done: usize,
    pub selected_result: usize,
    pub result_scroll: usize,

    // Logs
    pub logs: Vec<LogEntry>,
    pub log_scroll: usize,
    pub auto_scroll_logs: bool,

    // Stats
    pub bytes_transferred: u64,
    pub connections_total: u64,
    pub connections_active: u64,
    pub requests_per_sec: f64,

    // Termux specific
    pub is_termux: bool,
    pub show_help_popup: bool,
}

impl AppState {
    pub fn new(port: u16, is_termux: bool) -> Self {
        Self {
            active_tab: AppTab::Dashboard,
            input_mode: InputMode::Normal,
            target_host: String::new(),
            sni_host: String::new(),
            proxy_port: port,
            proxy_status: ProxyStatus::Stopped,
            input_target: String::new(),
            input_sni: String::new(),
            input_port: port.to_string(),
            input_hosts_file: "hosts.txt".to_string(),
            input_concurrency: "50".to_string(),
            active_field: 0,
            scan_status: ScanStatus::Idle,
            scan_results: Vec::new(),
            scan_progress: 0.0,
            scan_total: 0,
            scan_done: 0,
            selected_result: 0,
            result_scroll: 0,
            logs: Vec::new(),
            log_scroll: 0,
            auto_scroll_logs: true,
            bytes_transferred: 0,
            connections_total: 0,
            connections_active: 0,
            requests_per_sec: 0.0,
            is_termux,
            show_help_popup: false,
        }
    }

    pub fn add_log(&mut self, level: LogLevel, message: impl Into<String>) {
        let now = chrono::Local::now();
        self.logs.push(LogEntry {
            timestamp: now.format("%H:%M:%S").to_string(),
            level,
            message: message.into(),
        });
        // Keep last 500 logs
        if self.logs.len() > 500 {
            self.logs.drain(0..100);
        }
        if self.auto_scroll_logs {
            self.log_scroll = self.logs.len().saturating_sub(1);
        }
    }
}

pub struct App {
    pub state: Arc<Mutex<AppState>>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    proxy_handle: Option<tokio::task::JoinHandle<()>>,
    scanner_handle: Option<tokio::task::JoinHandle<()>>,
}

impl App {
    pub fn new(port: u16, is_termux: bool) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let state = Arc::new(Mutex::new(AppState::new(port, is_termux)));

        Ok(Self {
            state,
            event_tx,
            event_rx,
            proxy_handle: None,
            scanner_handle: None,
        })
    }

    pub fn set_target(&mut self, target: String) {
        let mut state = self.state.lock().unwrap();
        state.input_target = target.clone();
        state.target_host = target;
    }

    pub fn set_sni(&mut self, sni: String) {
        let mut state = self.state.lock().unwrap();
        state.input_sni = sni.clone();
        state.sni_host = sni;
    }

    pub async fn run(mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Log startup
        {
            let mut state = self.state.lock().unwrap();
            let is_termux = state.is_termux;
            state.add_log(LogLevel::Info, "SNI Bypass Tool started");
            if is_termux {
                state.add_log(LogLevel::Info, "Termux environment detected");
            }
            state.add_log(LogLevel::Info, "Press '?' for help");
        }

        // Start event handler
        let event_tx = self.event_tx.clone();
        let _event_handler = EventHandler::new(event_tx.clone());

        let result = self.main_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        // Cleanup background tasks
        if let Some(handle) = self.proxy_handle {
            handle.abort();
        }
        if let Some(handle) = self.scanner_handle {
            handle.abort();
        }

        result
    }

    async fn main_loop<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<()> {
        loop {
            // Draw UI
            {
                let state = self.state.lock().unwrap();
                terminal.draw(|f| ui::render(f, &state))?;
            }

            // Handle events with timeout
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if !self.handle_key(key.code, key.modifiers).await? {
                        return Ok(());
                    }
                }
            }

            // Process internal events
            while let Ok(event) = self.event_rx.try_recv() {
                self.handle_app_event(event).await?;
            }
        }
    }

    async fn handle_key(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<bool> {
        let input_mode = {
            let state = self.state.lock().unwrap();
            state.input_mode.clone()
        };

        match input_mode {
            InputMode::Normal => self.handle_normal_key(key, modifiers).await,
            InputMode::Editing => self.handle_edit_key(key, modifiers).await,
        }
    }

    async fn handle_normal_key(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<bool> {
        match key {
            // Quit
            KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(false),
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(false);
            }

            // Tab navigation
            KeyCode::Char('1') => self.set_tab(AppTab::Dashboard),
            KeyCode::Char('2') => self.set_tab(AppTab::Scanner),
            KeyCode::Char('3') => self.set_tab(AppTab::Results),
            KeyCode::Char('4') => self.set_tab(AppTab::Logs),
            KeyCode::Char('5') | KeyCode::Char('?') => {
                let mut state = self.state.lock().unwrap();
                state.show_help_popup = !state.show_help_popup;
            }
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),

            // Actions based on current tab
            KeyCode::Enter => self.handle_enter().await?,
            KeyCode::Char('e') | KeyCode::Char('i') => {
                let mut state = self.state.lock().unwrap();
                state.input_mode = InputMode::Editing;
            }
            KeyCode::Char('s') => self.toggle_proxy().await?,
            KeyCode::Char('S') => self.start_scan().await?,
            KeyCode::Char('x') => self.stop_scan(),

            // Scroll
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Char('g') => self.scroll_top(),
            KeyCode::Char('G') => self.scroll_bottom(),

            // Next field in editing
            KeyCode::Char('n') => self.next_field(),
            KeyCode::Char('p') => self.prev_field(),

            // Use selected SNI from results
            KeyCode::Char('u') => self.use_selected_sni(),

            // Toggle log auto-scroll
            KeyCode::Char('a') => {
                let mut state = self.state.lock().unwrap();
                state.auto_scroll_logs = !state.auto_scroll_logs;
            }

            _ => {}
        }
        Ok(true)
    }

    async fn handle_edit_key(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<bool> {
        match key {
            KeyCode::Esc => {
                let mut state = self.state.lock().unwrap();
                state.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                let mut state = self.state.lock().unwrap();
                state.input_mode = InputMode::Normal;
                // Move to next field
                state.active_field = (state.active_field + 1) % self.field_count(&state);
            }
            KeyCode::Tab => {
                let mut state = self.state.lock().unwrap();
                let count = self.field_count(&state);
                state.active_field = (state.active_field + 1) % count;
            }
            KeyCode::BackTab => {
                let mut state = self.state.lock().unwrap();
                let count = self.field_count(&state);
                state.active_field = state.active_field.saturating_sub(1);
                if state.active_field == 0 && count > 0 {
                    // wrap handled by saturating_sub
                }
            }
            KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => {
                if c == 'c' {
                    return Ok(false);
                }
            }
            KeyCode::Char(c) => {
                let mut state = self.state.lock().unwrap();
                self.push_char_to_field(&mut state, c);
            }
            KeyCode::Backspace => {
                let mut state = self.state.lock().unwrap();
                self.pop_char_from_field(&mut state);
            }
            _ => {}
        }
        Ok(true)
    }

    fn field_count(&self, state: &AppState) -> usize {
        match state.active_tab {
            AppTab::Dashboard => 3, // target, sni, port
            AppTab::Scanner => 2,   // hosts_file, concurrency
            _ => 1,
        }
    }

    fn push_char_to_field(&self, state: &mut AppState, c: char) {
        match state.active_tab {
            AppTab::Dashboard => match state.active_field {
                0 => state.input_target.push(c),
                1 => state.input_sni.push(c),
                2 => state.input_port.push(c),
                _ => {}
            },
            AppTab::Scanner => match state.active_field {
                0 => state.input_hosts_file.push(c),
                1 => state.input_concurrency.push(c),
                _ => {}
            },
            _ => {}
        }
    }

    fn pop_char_from_field(&self, state: &mut AppState) {
        match state.active_tab {
            AppTab::Dashboard => match state.active_field {
                0 => { state.input_target.pop(); }
                1 => { state.input_sni.pop(); }
                2 => { state.input_port.pop(); }
                _ => {}
            },
            AppTab::Scanner => match state.active_field {
                0 => { state.input_hosts_file.pop(); }
                1 => { state.input_concurrency.pop(); }
                _ => {}
            },
            _ => {}
        }
    }

    fn set_tab(&mut self, tab: AppTab) {
        let mut state = self.state.lock().unwrap();
        state.active_tab = tab;
        state.active_field = 0;
        state.input_mode = InputMode::Normal;
    }

    fn next_tab(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.active_tab = match state.active_tab {
            AppTab::Dashboard => AppTab::Scanner,
            AppTab::Scanner => AppTab::Results,
            AppTab::Results => AppTab::Logs,
            AppTab::Logs => AppTab::Help,
            AppTab::Help => AppTab::Dashboard,
        };
    }

    fn prev_tab(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.active_tab = match state.active_tab {
            AppTab::Dashboard => AppTab::Help,
            AppTab::Scanner => AppTab::Dashboard,
            AppTab::Results => AppTab::Scanner,
            AppTab::Logs => AppTab::Results,
            AppTab::Help => AppTab::Logs,
        };
    }

    async fn handle_enter(&mut self) -> Result<()> {
        let tab = {
            let state = self.state.lock().unwrap();
            state.active_tab.clone()
        };
        match tab {
            AppTab::Dashboard => self.toggle_proxy().await?,
            AppTab::Scanner => self.start_scan().await?,
            AppTab::Results => self.use_selected_sni(),
            _ => {}
        }
        Ok(())
    }

    async fn toggle_proxy(&mut self) -> Result<()> {
        let status = {
            let state = self.state.lock().unwrap();
            state.proxy_status.clone()
        };

        match status {
            ProxyStatus::Stopped | ProxyStatus::Error(_) => self.start_proxy().await,
            ProxyStatus::Running => self.stop_proxy(),
            _ => Ok(()),
        }
    }

    async fn start_proxy(&mut self) -> Result<()> {
        let (target, sni, port) = {
            let mut state = self.state.lock().unwrap();
            let target = if state.input_target.is_empty() {
                state.add_log(LogLevel::Error, "Target host is required");
                return Ok(());
            } else {
                state.input_target.clone()
            };
            let sni = if state.input_sni.is_empty() {
                target.clone()
            } else {
                state.input_sni.clone()
            };
            let port = state.input_port.parse::<u16>().unwrap_or(8080);
            state.proxy_status = ProxyStatus::Starting;
            state.target_host = target.clone();
            state.sni_host = sni.clone();
            state.proxy_port = port;
            state.add_log(LogLevel::Info, format!("Starting proxy on port {port}..."));
            state.add_log(LogLevel::Info, format!("Target: {target} | SNI: {sni}"));
            (target, sni, port)
        };

        let state_clone = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();

        let handle = tokio::spawn(async move {
            let server = ProxyServer::new(port, target, sni);
            match server.run_with_stats(state_clone.clone(), event_tx.clone()).await {
                Ok(_) => {
                    let mut state = state_clone.lock().unwrap();
                    state.proxy_status = ProxyStatus::Stopped;
                    state.add_log(LogLevel::Info, "Proxy stopped");
                }
                Err(e) => {
                    let mut state = state_clone.lock().unwrap();
                    state.proxy_status = ProxyStatus::Error(e.to_string());
                    state.add_log(LogLevel::Error, format!("Proxy error: {e}"));
                }
            }
        });

        self.proxy_handle = Some(handle);

        // Brief delay then check startup
        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut state = self.state.lock().unwrap();
        if state.proxy_status == ProxyStatus::Starting {
            state.proxy_status = ProxyStatus::Running;
            state.add_log(LogLevel::Success, "Proxy running!");
        }

        Ok(())
    }

    fn stop_proxy(&mut self) -> Result<()> {
        if let Some(handle) = self.proxy_handle.take() {
            handle.abort();
        }
        let mut state = self.state.lock().unwrap();
        state.proxy_status = ProxyStatus::Stopped;
        state.connections_active = 0;
        state.add_log(LogLevel::Info, "Proxy stopped by user");
        Ok(())
    }

    async fn start_scan(&mut self) -> Result<()> {
        let scan_status = {
            let state = self.state.lock().unwrap();
            state.scan_status.clone()
        };

        if scan_status == ScanStatus::Running {
            let mut state = self.state.lock().unwrap();
            state.add_log(LogLevel::Warning, "Scanner is already running");
            return Ok(());
        }

        let (hosts_file, concurrency) = {
            let mut state = self.state.lock().unwrap();
            let hosts_file = state.input_hosts_file.clone();
            let concurrency = state.input_concurrency.parse::<usize>().unwrap_or(50);
            state.scan_status = ScanStatus::Running;
            state.scan_results.clear();
            state.scan_progress = 0.0;
            state.scan_done = 0;
            state.add_log(LogLevel::Info, format!("Scanning {hosts_file} with concurrency {concurrency}"));
            (hosts_file, concurrency)
        };

        let state_clone = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();

        let handle = tokio::spawn(async move {
            let scanner = SniScanner::new(concurrency);

            // Count total hosts
            let hosts = match tokio::fs::read_to_string(&hosts_file).await {
                Ok(content) => content
                    .lines()
                    .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                    .map(|l| l.trim().to_string())
                    .collect::<Vec<_>>(),
                Err(e) => {
                    let mut state = state_clone.lock().unwrap();
                    state.scan_status = ScanStatus::Error(e.to_string());
                    state.add_log(LogLevel::Error, format!("Cannot read hosts file: {e}"));
                    return;
                }
            };

            {
                let mut state = state_clone.lock().unwrap();
                state.scan_total = hosts.len();
                state.add_log(LogLevel::Info, format!("Found {} hosts to scan", hosts.len()));
            }

            let _ = event_tx.send(AppEvent::ScanStarted(hosts.len()));

            let results = scanner.scan_hosts(
                hosts,
                move |result| {
                    let _ = event_tx.send(AppEvent::ScanResult(result));
                }
            ).await;

            let mut state = state_clone.lock().unwrap();
            let working_count = results.iter().filter(|r| r.is_working).count();
            state.scan_status = ScanStatus::Completed;
            state.scan_progress = 1.0;
            state.add_log(
                LogLevel::Success,
                format!("Scan complete! {}/{} hosts working", working_count, results.len())
            );
            let _ = event_tx.send(AppEvent::ScanCompleted);
        });

        self.scanner_handle = Some(handle);
        // Switch to results tab automatically
        {
            let mut state = self.state.lock().unwrap();
            state.active_tab = AppTab::Results;
        }

        Ok(())
    }

    fn stop_scan(&mut self) {
        if let Some(handle) = self.scanner_handle.take() {
            handle.abort();
        }
        let mut state = self.state.lock().unwrap();
        state.scan_status = ScanStatus::Idle;
        state.add_log(LogLevel::Warning, "Scan stopped by user");
    }

    async fn handle_app_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::ScanResult(result) => {
                let mut state = self.state.lock().unwrap();
                state.scan_done += 1;
                if state.scan_total > 0 {
                    state.scan_progress = state.scan_done as f64 / state.scan_total as f64;
                }
                if result.is_working {
                    state.add_log(
                        LogLevel::Success,
                        format!("✓ {} - {}ms", result.host, result.latency_ms)
                    );
                }
                state.scan_results.push(result);
                // Sort: working first, then by latency
                state.scan_results.sort_by(|a, b| {
                    b.is_working.cmp(&a.is_working)
                        .then(a.latency_ms.cmp(&b.latency_ms))
                });
            }
            AppEvent::ScanStarted(total) => {
                let mut state = self.state.lock().unwrap();
                state.scan_total = total;
            }
            AppEvent::ScanCompleted => {
                let mut state = self.state.lock().unwrap();
                state.scan_status = ScanStatus::Completed;
            }
            AppEvent::ProxyConnection { bytes, active } => {
                let mut state = self.state.lock().unwrap();
                state.bytes_transferred += bytes;
                state.connections_active = active;
                state.connections_total += 1;
            }
            AppEvent::Log(level, msg) => {
                let mut state = self.state.lock().unwrap();
                state.add_log(level, msg);
            }
        }
        Ok(())
    }

    fn use_selected_sni(&mut self) {
        let selected = {
            let state = self.state.lock().unwrap();
            state.scan_results.get(state.selected_result).cloned()
        };
        if let Some(result) = selected {
            if result.is_working {
                let mut state = self.state.lock().unwrap();
                state.input_sni = result.host.clone();
                state.sni_host = result.host.clone();
                state.active_tab = AppTab::Dashboard;
                state.add_log(
                    LogLevel::Info,
                    format!("SNI set to: {}", result.host)
                );
            }
        }
    }

    fn next_field(&mut self) {
        let mut state = self.state.lock().unwrap();
        let count = match state.active_tab {
            AppTab::Dashboard => 3,
            AppTab::Scanner => 2,
            _ => 1,
        };
        state.active_field = (state.active_field + 1) % count;
    }

    fn prev_field(&mut self) {
        let mut state = self.state.lock().unwrap();
        let count = match state.active_tab {
            AppTab::Dashboard => 3,
            AppTab::Scanner => 2,
            _ => 1usize,
        };
        if state.active_field == 0 {
            state.active_field = count - 1;
        } else {
            state.active_field -= 1;
        }
    }

    fn scroll_up(&mut self) {
        let mut state = self.state.lock().unwrap();
        match state.active_tab {
            AppTab::Results => {
                if state.selected_result > 0 {
                    state.selected_result -= 1;
                    if state.selected_result < state.result_scroll {
                        state.result_scroll = state.selected_result;
                    }
                }
            }
            AppTab::Logs => {
                state.auto_scroll_logs = false;
                state.log_scroll = state.log_scroll.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn scroll_down(&mut self) {
        let mut state = self.state.lock().unwrap();
        match state.active_tab {
            AppTab::Results => {
                let max = state.scan_results.len().saturating_sub(1);
                if state.selected_result < max {
                    state.selected_result += 1;
                }
            }
            AppTab::Logs => {
                let max = state.logs.len().saturating_sub(1);
                if state.log_scroll < max {
                    state.log_scroll += 1;
                }
            }
            _ => {}
        }
    }

    fn page_up(&mut self) {
        let mut state = self.state.lock().unwrap();
        match state.active_tab {
            AppTab::Results => {
                state.selected_result = state.selected_result.saturating_sub(10);
            }
            AppTab::Logs => {
                state.auto_scroll_logs = false;
                state.log_scroll = state.log_scroll.saturating_sub(10);
            }
            _ => {}
        }
    }

    fn page_down(&mut self) {
        let mut state = self.state.lock().unwrap();
        match state.active_tab {
            AppTab::Results => {
                let max = state.scan_results.len().saturating_sub(1);
                state.selected_result = (state.selected_result + 10).min(max);
            }
            AppTab::Logs => {
                let max = state.logs.len().saturating_sub(1);
                state.log_scroll = (state.log_scroll + 10).min(max);
            }
            _ => {}
        }
    }

    fn scroll_top(&mut self) {
        let mut state = self.state.lock().unwrap();
        match state.active_tab {
            AppTab::Results => state.selected_result = 0,
            AppTab::Logs => {
                state.auto_scroll_logs = false;
                state.log_scroll = 0;
            }
            _ => {}
        }
    }

    fn scroll_bottom(&mut self) {
        let mut state = self.state.lock().unwrap();
        match state.active_tab {
            AppTab::Results => {
                state.selected_result = state.scan_results.len().saturating_sub(1);
            }
            AppTab::Logs => {
                state.log_scroll = state.logs.len().saturating_sub(1);
                state.auto_scroll_logs = true;
            }
            _ => {}
        }
    }
}
