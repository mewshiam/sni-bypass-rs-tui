use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode,
        KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;

use super::events::{AppEvent, EventHandler};
use super::ui;
use crate::bypass::ProxyServer;
use crate::scanner::{ScanResult, SniScanner};
use crate::Config;

// ─────────────────────────────────────────────
// Tab
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AppTab {
    Dashboard,
    Scanner,
    Results,
    Logs,
    Help,
}

// ─────────────────────────────────────────────
// Input mode
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

// ─────────────────────────────────────────────
// Log
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

// ─────────────────────────────────────────────
// Proxy / scan status
// ─────────────────────────────────────────────

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

// ─────────────────────────────────────────────
// InputField — value + cursor
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct InputField {
    pub value: String,
    pub cursor: usize, // byte index
}

impl InputField {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.len();
        Self { value, cursor }
    }

    pub fn insert(&mut self, c: char) {
        let pos = self.clamp(self.cursor);
        self.value.insert(pos, c);
        self.cursor = pos + c.len_utf8();
    }

    pub fn delete_backward(&mut self) {
        let pos = self.clamp(self.cursor);
        if pos == 0 {
            return;
        }
        let prev = self.prev_boundary(pos);
        self.value.drain(prev..pos);
        self.cursor = prev;
    }

    pub fn delete_forward(&mut self) {
        let pos = self.clamp(self.cursor);
        if pos >= self.value.len() {
            return;
        }
        let next = self.next_boundary(pos);
        self.value.drain(pos..next);
    }

    pub fn move_left(&mut self) {
        let pos = self.clamp(self.cursor);
        self.cursor = self.prev_boundary(pos);
    }

    pub fn move_right(&mut self) {
        let pos = self.clamp(self.cursor);
        self.cursor = self.next_boundary(pos);
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    pub fn delete_to_start(&mut self) {
        let pos = self.clamp(self.cursor);
        self.value.drain(..pos);
        self.cursor = 0;
    }

    pub fn delete_to_end(&mut self) {
        let pos = self.clamp(self.cursor);
        self.value.truncate(pos);
    }

    pub fn delete_word_backward(&mut self) {
        let pos = self.clamp(self.cursor);
        if pos == 0 {
            return;
        }
        // skip trailing spaces
        let mut p = pos;
        while p > 0 {
            let prev = self.prev_boundary(p);
            if self.value[prev..p].chars().next() == Some(' ') {
                p = prev;
            } else {
                break;
            }
        }
        // delete until space or start
        while p > 0 {
            let prev = self.prev_boundary(p);
            if self.value[prev..p].chars().next() == Some(' ') {
                break;
            }
            p = prev;
        }
        self.value.drain(p..pos);
        self.cursor = p;
    }

    pub fn paste(&mut self, text: &str) {
        let clean: String =
            text.chars().filter(|c| !c.is_control()).collect();
        let pos = self.clamp(self.cursor);
        self.value.insert_str(pos, &clean);
        self.cursor = pos + clean.len();
    }

    pub fn set(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.len();
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    /// String with cursor marker injected for display
    pub fn display_with_cursor(&self) -> String {
        let pos = self.clamp(self.cursor);
        if pos >= self.value.len() {
            format!("{}█", self.value)
        } else {
            let (before, after) = self.value.split_at(pos);
            let mut chars = after.chars();
            let cur = chars.next().unwrap_or(' ');
            format!("{}[{}]{}", before, cur, chars.as_str())
        }
    }

    fn clamp(&self, pos: usize) -> usize {
        pos.min(self.value.len())
    }

    fn prev_boundary(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let mut p = pos - 1;
        while p > 0 && !self.value.is_char_boundary(p) {
            p -= 1;
        }
        p
    }

    fn next_boundary(&self, pos: usize) -> usize {
        if pos >= self.value.len() {
            return self.value.len();
        }
        let mut p = pos + 1;
        while p < self.value.len() && !self.value.is_char_boundary(p) {
            p += 1;
        }
        p
    }
}

// ─────────────────────────────────────────────
// ActiveField — which input is focused
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveField {
    // Dashboard
    Target,
    Sni,
    Port,
    // Scanner
    HostsFile,
    Concurrency,
}

impl ActiveField {
    pub fn fields_for_tab(tab: &AppTab) -> Vec<ActiveField> {
        match tab {
            AppTab::Dashboard => vec![
                ActiveField::Target,
                ActiveField::Sni,
                ActiveField::Port,
            ],
            AppTab::Scanner => vec![
                ActiveField::HostsFile,
                ActiveField::Concurrency,
            ],
            _ => vec![],
        }
    }

    pub fn next_in_tab(&self, tab: &AppTab) -> ActiveField {
        let fields = Self::fields_for_tab(tab);
        if fields.is_empty() {
            return self.clone();
        }
        let pos = fields.iter().position(|f| f == self).unwrap_or(0);
        fields[(pos + 1) % fields.len()].clone()
    }

    pub fn prev_in_tab(&self, tab: &AppTab) -> ActiveField {
        let fields = Self::fields_for_tab(tab);
        if fields.is_empty() {
            return self.clone();
        }
        let len = fields.len();
        let pos = fields.iter().position(|f| f == self).unwrap_or(0);
        fields[(pos + len - 1) % len].clone()
    }

    pub fn index(&self, tab: &AppTab) -> usize {
        Self::fields_for_tab(tab)
            .iter()
            .position(|f| f == self)
            .unwrap_or(0)
    }
}

// ─────────────────────────────────────────────
// AppState
// ─────────────────────────────────────────────

pub struct AppState {
    // Navigation
    pub active_tab: AppTab,
    pub input_mode: InputMode,
    pub active_field: ActiveField,

    // Dashboard input fields
    pub field_target: InputField,
    pub field_sni: InputField,
    pub field_port: InputField,

    // Scanner input fields
    pub field_hosts_file: InputField,
    pub field_concurrency: InputField,

    // Proxy runtime
    pub target_host: String,
    pub sni_host: String,
    pub proxy_port: u16,
    pub proxy_status: ProxyStatus,

    // Scanner runtime
    pub scan_status: ScanStatus,
    pub scan_results: Vec<ScanResult>,
    pub scan_progress: f64,
    pub scan_total: usize,
    pub scan_done: usize,

    // Results view
    pub selected_result: usize,
    pub result_scroll: usize,

    // Logs
    pub logs: Vec<LogEntry>,
    pub log_scroll: usize,
    pub auto_scroll_logs: bool,
    pub max_log_entries: usize,

    // Stats
    pub bytes_transferred: u64,
    pub connections_total: u64,
    pub connections_active: u64,
    pub requests_per_sec: f64,

    // Misc
    pub is_termux: bool,
    pub show_help_popup: bool,
}

impl AppState {
    pub fn new(config: &Config, is_termux: bool) -> Self {
        Self {
            active_tab: AppTab::Dashboard,
            input_mode: InputMode::Normal,
            active_field: ActiveField::Target,

            field_target: InputField::new(&config.proxy.target_host),
            field_sni: InputField::new(&config.proxy.sni_host),
            field_port: InputField::new(config.proxy.port.to_string()),

            field_hosts_file: InputField::new(
                &config.scanner.hosts_file,
            ),
            field_concurrency: InputField::new(
                config.scanner.concurrency.to_string(),
            ),

            target_host: config.proxy.target_host.clone(),
            sni_host: config.proxy.sni_host.clone(),
            proxy_port: config.proxy.port,
            proxy_status: ProxyStatus::Stopped,

            scan_status: ScanStatus::Idle,
            scan_results: Vec::new(),
            scan_progress: 0.0,
            scan_total: 0,
            scan_done: 0,

            selected_result: 0,
            result_scroll: 0,

            logs: Vec::new(),
            log_scroll: 0,
            auto_scroll_logs: config.tui.auto_scroll_logs,
            max_log_entries: config.tui.max_log_entries,

            bytes_transferred: 0,
            connections_total: 0,
            connections_active: 0,
            requests_per_sec: 0.0,

            is_termux,
            show_help_popup: config.tui.show_help_on_start,
        }
    }

    pub fn active_input_mut(&mut self) -> Option<&mut InputField> {
        match self.active_field {
            ActiveField::Target => Some(&mut self.field_target),
            ActiveField::Sni => Some(&mut self.field_sni),
            ActiveField::Port => Some(&mut self.field_port),
            ActiveField::HostsFile => Some(&mut self.field_hosts_file),
            ActiveField::Concurrency => {
                Some(&mut self.field_concurrency)
            }
        }
    }

    pub fn add_log(
        &mut self,
        level: LogLevel,
        message: impl Into<String>,
    ) {
        let now = chrono::Local::now();
        self.logs.push(LogEntry {
            timestamp: now.format("%H:%M:%S").to_string(),
            level,
            message: message.into(),
        });
        if self.logs.len() > self.max_log_entries {
            let drain = self.max_log_entries / 5;
            self.logs.drain(0..drain);
        }
        if self.auto_scroll_logs {
            self.log_scroll = self.logs.len().saturating_sub(1);
        }
    }
}

// ─────────────────────────────────────────────
// Clipboard helpers
// ─────────────────────────────────────────────

fn get_clipboard(is_termux: bool) -> Option<String> {
    let out = if is_termux {
        std::process::Command::new("termux-clipboard-get")
            .output()
            .ok()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("pbpaste").output().ok()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("powershell")
            .args(["-command", "Get-Clipboard"])
            .output()
            .ok()
    } else {
        std::process::Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output()
            .ok()
            .or_else(|| {
                std::process::Command::new("xsel")
                    .args(["--clipboard", "--output"])
                    .output()
                    .ok()
            })
            .or_else(|| {
                std::process::Command::new("wl-paste")
                    .output()
                    .ok()
            })
    }?;

    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        None
    }
}

fn set_clipboard(text: &str, is_termux: bool) {
    use std::io::Write;
    if is_termux {
        let _ = std::process::Command::new("termux-clipboard-set")
            .arg(text)
            .status();
    } else if cfg!(target_os = "macos") {
        if let Ok(mut child) = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
        }
    } else if cfg!(target_os = "windows") {
        let _ = std::process::Command::new("powershell")
            .args(["-command", &format!("Set-Clipboard '{}'", text)])
            .status();
    } else {
        let try_write = |cmd: &str, args: &[&str]| {
            std::process::Command::new(cmd)
                .args(args)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .ok()
                .and_then(|mut child| {
                    if let Some(stdin) = child.stdin.as_mut() {
                        let _ = stdin.write_all(text.as_bytes());
                    }
                    child.wait().ok()
                })
        };
        if try_write("xclip", &["-selection", "clipboard"]).is_none() {
            let _ = try_write("wl-copy", &[]);
        }
    }
}

// ─────────────────────────────────────────────
// App
// ─────────────────────────────────────────────

pub struct App {
    pub state: Arc<Mutex<AppState>>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    proxy_handle: Option<tokio::task::JoinHandle<()>>,
    scanner_handle: Option<tokio::task::JoinHandle<()>>,
}

impl App {
    pub fn new(config: &Config, is_termux: bool) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let state =
            Arc::new(Mutex::new(AppState::new(config, is_termux)));
        Ok(Self {
            state,
            event_tx,
            event_rx,
            proxy_handle: None,
            scanner_handle: None,
        })
    }

    pub fn set_target(&mut self, target: String) {
        let mut s = self.state.lock().unwrap();
        s.field_target.set(&target);
        s.target_host = target;
    }

    pub fn set_sni(&mut self, sni: String) {
        let mut s = self.state.lock().unwrap();
        s.field_sni.set(&sni);
        s.sni_host = sni;
    }

    // ── Run ───────────────────────────────────

    pub async fn run(mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        {
            let mut s = self.state.lock().unwrap();
            let termux = s.is_termux;
            s.add_log(
                LogLevel::Info,
                format!(
                    "SNI Bypass RS-TUI v{} started",
                    env!("CARGO_PKG_VERSION")
                ),
            );
            if termux {
                s.add_log(
                    LogLevel::Info,
                    "Termux detected — clipboard via termux-clipboard-get/set",
                );
            }
            s.add_log(LogLevel::Info, "Press [?] for help");
        }

        let _eh = EventHandler::new(self.event_tx.clone());
        let result = self.main_loop(&mut terminal).await;

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Some(h) = self.proxy_handle {
            h.abort();
        }
        if let Some(h) = self.scanner_handle {
            h.abort();
        }

        result
    }

    // ── Main loop ─────────────────────────────

    async fn main_loop<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<()> {
        let tick = Duration::from_millis(16);

        loop {
            // Draw frame
            {
                let s = self.state.lock().unwrap();
                terminal.draw(|f| ui::render(f, &s))?;
            }

            // Poll input
            if event::poll(tick)? {
                match event::read()? {
                    Event::Key(key) => {
                        if !self.handle_key(key).await? {
                            return Ok(());
                        }
                    }
                    Event::Paste(text) => {
                        self.handle_paste(text);
                    }
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }

            // Drain internal events
            while let Ok(ev) = self.event_rx.try_recv() {
                self.handle_app_event(ev).await?;
            }
        }
    }

    // ── Paste ─────────────────────────────────

    fn handle_paste(&mut self, text: String) {
        let mut s = self.state.lock().unwrap();
        if s.input_mode == InputMode::Editing {
            if let Some(f) = s.active_input_mut() {
                f.paste(&text);
            }
        }
    }

    // ── Key dispatch ──────────────────────────

    async fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        let mode = self.state.lock().unwrap().input_mode.clone();
        match mode {
            InputMode::Normal => self.key_normal(key).await,
            InputMode::Editing => self.key_edit(key).await,
        }
    }

    // ── Normal mode ───────────────────────────

    async fn key_normal(&mut self, key: KeyEvent) -> Result<bool> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(false),
            KeyCode::Char('c') if ctrl => return Ok(false),

            // Tabs
            KeyCode::Char('1') => self.set_tab(AppTab::Dashboard),
            KeyCode::Char('2') => self.set_tab(AppTab::Scanner),
            KeyCode::Char('3') => self.set_tab(AppTab::Results),
            KeyCode::Char('4') => self.set_tab(AppTab::Logs),
            KeyCode::Char('5') => self.set_tab(AppTab::Help),
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),

            // Help popup
            KeyCode::Char('?') => {
                let mut s = self.state.lock().unwrap();
                s.show_help_popup = !s.show_help_popup;
            }
            KeyCode::Esc => {
                let mut s = self.state.lock().unwrap();
                s.show_help_popup = false;
            }

            // Enter edit mode
            KeyCode::Char('e') | KeyCode::Char('i') => {
                let mut s = self.state.lock().unwrap();
                let tab = s.active_tab.clone();
                if matches!(tab, AppTab::Dashboard | AppTab::Scanner) {
                    s.input_mode = InputMode::Editing;
                }
            }

            // Proxy / scan
            KeyCode::Char('s') => self.toggle_proxy().await?,
            KeyCode::Char('S') => self.start_scan().await?,
            KeyCode::Char('x') => self.stop_scan(),
            KeyCode::Enter => self.ctx_enter().await?,

            // Use SNI
            KeyCode::Char('u') => self.use_selected_sni(),

            // Field navigation
            KeyCode::Char('n') => self.next_field(),
            KeyCode::Char('p') => self.prev_field(),

            // Scrolling
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Home | KeyCode::Char('g') => self.scroll_top(),
            KeyCode::End | KeyCode::Char('G') => self.scroll_bottom(),

            // Log auto scroll
            KeyCode::Char('a') => {
                let mut s = self.state.lock().unwrap();
                s.auto_scroll_logs = !s.auto_scroll_logs;
            }

            _ => {}
        }
        Ok(true)
    }

    // ── Edit mode ─────────────────────────────

    async fn key_edit(&mut self, key: KeyEvent) -> Result<bool> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            // Exit edit mode
            KeyCode::Esc => {
                self.state.lock().unwrap().input_mode =
                    InputMode::Normal;
            }

            // Hard quit
            KeyCode::Char('c') if ctrl => return Ok(false),

            // Field navigation
            KeyCode::Enter | KeyCode::Tab => self.next_field(),
            KeyCode::BackTab => self.prev_field(),

            // Cursor movement
            KeyCode::Left => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.move_left();
                }
            }
            KeyCode::Right => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.move_right();
                }
            }
            KeyCode::Home => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.move_home();
                }
            }
            KeyCode::End => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.move_end();
                }
            }

            // Ctrl+A / E  — line start / end
            KeyCode::Char('a') if ctrl => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.move_home();
                }
            }
            KeyCode::Char('e') if ctrl => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.move_end();
                }
            }

            // Deletion
            KeyCode::Backspace => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.delete_backward();
                }
            }
            KeyCode::Delete => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.delete_forward();
                }
            }
            KeyCode::Char('u') if ctrl => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.delete_to_start();
                }
            }
            KeyCode::Char('k') if ctrl => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.delete_to_end();
                }
            }
            KeyCode::Char('w') if ctrl => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.delete_word_backward();
                }
            }

            // Paste  Ctrl+V / Ctrl+Y
            KeyCode::Char('v') if ctrl => {
                let is_termux =
                    self.state.lock().unwrap().is_termux;
                if let Some(text) = get_clipboard(is_termux) {
                    let mut s = self.state.lock().unwrap();
                    if let Some(f) = s.active_input_mut() {
                        f.paste(&text);
                    }
                }
            }
            KeyCode::Char('y') if ctrl => {
                let is_termux =
                    self.state.lock().unwrap().is_termux;
                if let Some(text) = get_clipboard(is_termux) {
                    let mut s = self.state.lock().unwrap();
                    if let Some(f) = s.active_input_mut() {
                        f.paste(&text);
                    }
                }
            }

            // Copy field  Ctrl+C
            KeyCode::Char('c') if ctrl => {
                let (value, is_termux) = {
                    let s = self.state.lock().unwrap();
                    let v = match s.active_field {
                        ActiveField::Target => {
                            s.field_target.value.clone()
                        }
                        ActiveField::Sni => s.field_sni.value.clone(),
                        ActiveField::Port => {
                            s.field_port.value.clone()
                        }
                        ActiveField::HostsFile => {
                            s.field_hosts_file.value.clone()
                        }
                        ActiveField::Concurrency => {
                            s.field_concurrency.value.clone()
                        }
                    };
                    (v, s.is_termux)
                };
                set_clipboard(&value, is_termux);
                self.state
                    .lock()
                    .unwrap()
                    .add_log(LogLevel::Info, "Copied to clipboard");
            }

            // Regular character
            KeyCode::Char(c) => {
                let mut s = self.state.lock().unwrap();
                if let Some(f) = s.active_input_mut() {
                    f.insert(c);
                }
            }

            _ => {}
        }
        Ok(true)
    }

    // ── Tab helpers ───────────────────────────

    fn set_tab(&mut self, tab: AppTab) {
        let mut s = self.state.lock().unwrap();
        let first = ActiveField::fields_for_tab(&tab)
            .into_iter()
            .next()
            .unwrap_or(ActiveField::Target);
        s.active_tab = tab;
        s.active_field = first;
        s.input_mode = InputMode::Normal;
        s.show_help_popup = false;
    }

    fn next_tab(&mut self) {
        let mut s = self.state.lock().unwrap();
        let next = match s.active_tab {
            AppTab::Dashboard => AppTab::Scanner,
            AppTab::Scanner => AppTab::Results,
            AppTab::Results => AppTab::Logs,
            AppTab::Logs => AppTab::Help,
            AppTab::Help => AppTab::Dashboard,
        };
        let first = ActiveField::fields_for_tab(&next)
            .into_iter()
            .next()
            .unwrap_or(ActiveField::Target);
        s.active_tab = next;
        s.active_field = first;
        s.input_mode = InputMode::Normal;
    }

    fn prev_tab(&mut self) {
        let mut s = self.state.lock().unwrap();
        let prev = match s.active_tab {
            AppTab::Dashboard => AppTab::Help,
            AppTab::Scanner => AppTab::Dashboard,
            AppTab::Results => AppTab::Scanner,
            AppTab::Logs => AppTab::Results,
            AppTab::Help => AppTab::Logs,
        };
        let first = ActiveField::fields_for_tab(&prev)
            .into_iter()
            .next()
            .unwrap_or(ActiveField::Target);
        s.active_tab = prev;
        s.active_field = first;
        s.input_mode = InputMode::Normal;
    }

    // ── Field navigation ──────────────────────

    fn next_field(&mut self) {
        let mut s = self.state.lock().unwrap();
        let tab = s.active_tab.clone();
        if ActiveField::fields_for_tab(&tab).is_empty() {
            s.input_mode = InputMode::Normal;
            return;
        }
        s.active_field = s.active_field.next_in_tab(&tab);
    }

    fn prev_field(&mut self) {
        let mut s = self.state.lock().unwrap();
        let tab = s.active_tab.clone();
        if ActiveField::fields_for_tab(&tab).is_empty() {
            s.input_mode = InputMode::Normal;
            return;
        }
        s.active_field = s.active_field.prev_in_tab(&tab);
    }

    // ── Context Enter ─────────────────────────

    async fn ctx_enter(&mut self) -> Result<()> {
        let (tab, mode) = {
            let s = self.state.lock().unwrap();
            (s.active_tab.clone(), s.input_mode.clone())
        };
        if mode == InputMode::Editing {
            self.next_field();
            return Ok(());
        }
        match tab {
            AppTab::Dashboard => self.toggle_proxy().await?,
            AppTab::Scanner => self.start_scan().await?,
            AppTab::Results => self.use_selected_sni(),
            _ => {}
        }
        Ok(())
    }

    // ── Scroll ────────────────────────────────

    fn scroll_up(&mut self) {
        let mut s = self.state.lock().unwrap();
        match s.active_tab {
            AppTab::Results => {
                if s.selected_result > 0 {
                    s.selected_result -= 1;
                    if s.selected_result < s.result_scroll {
                        s.result_scroll = s.selected_result;
                    }
                }
            }
            AppTab::Logs => {
                s.auto_scroll_logs = false;
                s.log_scroll = s.log_scroll.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn scroll_down(&mut self) {
        let mut s = self.state.lock().unwrap();
        match s.active_tab {
            AppTab::Results => {
                let max = s.scan_results.len().saturating_sub(1);
                if s.selected_result < max {
                    s.selected_result += 1;
                }
            }
            AppTab::Logs => {
                let max = s.logs.len().saturating_sub(1);
                if s.log_scroll < max {
                    s.log_scroll += 1;
                }
            }
            _ => {}
        }
    }

    fn page_up(&mut self) {
        let mut s = self.state.lock().unwrap();
        match s.active_tab {
            AppTab::Results => {
                s.selected_result =
                    s.selected_result.saturating_sub(10);
                s.result_scroll = s.result_scroll.saturating_sub(10);
            }
            AppTab::Logs => {
                s.auto_scroll_logs = false;
                s.log_scroll = s.log_scroll.saturating_sub(10);
            }
            _ => {}
        }
    }

    fn page_down(&mut self) {
        let mut s = self.state.lock().unwrap();
        match s.active_tab {
            AppTab::Results => {
                let max = s.scan_results.len().saturating_sub(1);
                s.selected_result = (s.selected_result + 10).min(max);
            }
            AppTab::Logs => {
                let max = s.logs.len().saturating_sub(1);
                s.log_scroll = (s.log_scroll + 10).min(max);
            }
            _ => {}
        }
    }

    fn scroll_top(&mut self) {
        let mut s = self.state.lock().unwrap();
        match s.active_tab {
            AppTab::Results => {
                s.selected_result = 0;
                s.result_scroll = 0;
            }
            AppTab::Logs => {
                s.auto_scroll_logs = false;
                s.log_scroll = 0;
            }
            _ => {}
        }
    }

    fn scroll_bottom(&mut self) {
        let mut s = self.state.lock().unwrap();
        match s.active_tab {
            AppTab::Results => {
                let max = s.scan_results.len().saturating_sub(1);
                s.selected_result = max;
                s.result_scroll = max;
            }
            AppTab::Logs => {
                s.log_scroll = s.logs.len().saturating_sub(1);
                s.auto_scroll_logs = true;
            }
            _ => {}
        }
    }

    // ── Proxy control ─────────────────────────

    async fn toggle_proxy(&mut self) -> Result<()> {
        let status =
            self.state.lock().unwrap().proxy_status.clone();
        match status {
            ProxyStatus::Stopped | ProxyStatus::Error(_) => {
                self.start_proxy().await
            }
            ProxyStatus::Running => self.stop_proxy(),
            ProxyStatus::Starting => Ok(()),
        }
    }

    async fn start_proxy(&mut self) -> Result<()> {
        let (target, sni, port) = {
            let mut s = self.state.lock().unwrap();
            let target = s.field_target.value.trim().to_string();
            if target.is_empty() {
                s.add_log(LogLevel::Error, "Target host is required");
                return Ok(());
            }
            let sni = {
                let v = s.field_sni.value.trim().to_string();
                if v.is_empty() {
                    target.clone()
                } else {
                    v
                }
            };
            let port =
                s.field_port.value.trim().parse::<u16>().unwrap_or(8080);

            s.proxy_status = ProxyStatus::Starting;
            s.target_host = target.clone();
            s.sni_host = sni.clone();
            s.proxy_port = port;
            s.add_log(
                LogLevel::Info,
                format!("Starting proxy on port {}...", port),
            );
            s.add_log(
                LogLevel::Info,
                format!("Target: {}  SNI: {}", target, sni),
            );
            (target, sni, port)
        };

        let state_clone = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();

        let handle = tokio::spawn(async move {
            let server = ProxyServer::new(port, target, sni);
            match server
                .run_with_stats(state_clone.clone(), event_tx)
                .await
            {
                Ok(_) => {
                    let mut s = state_clone.lock().unwrap();
                    s.proxy_status = ProxyStatus::Stopped;
                    s.add_log(LogLevel::Info, "Proxy stopped");
                }
                Err(e) => {
                    let mut s = state_clone.lock().unwrap();
                    s.proxy_status =
                        ProxyStatus::Error(e.to_string());
                    s.add_log(
                        LogLevel::Error,
                        format!("Proxy error: {}", e),
                    );
                }
            }
        });

        self.proxy_handle = Some(handle);

        // AFTER — read proxy_port first, then borrow mutably
tokio::time::sleep(Duration::from_millis(150)).await;
let mut s = self.state.lock().unwrap();
if s.proxy_status == ProxyStatus::Starting {
    s.proxy_status = ProxyStatus::Running;
    let port = s.proxy_port; // read BEFORE mutable borrow via add_log
    s.add_log(
        LogLevel::Success,
        format!("Proxy running on 127.0.0.1:{}", port),
    );
}
        Ok(())
    }

    fn stop_proxy(&mut self) -> Result<()> {
        if let Some(h) = self.proxy_handle.take() {
            h.abort();
        }
        let mut s = self.state.lock().unwrap();
        s.proxy_status = ProxyStatus::Stopped;
        s.connections_active = 0;
        s.add_log(LogLevel::Info, "Proxy stopped");
        Ok(())
    }

    // ── Scanner control ───────────────────────

    async fn start_scan(&mut self) -> Result<()> {
        if self.state.lock().unwrap().scan_status == ScanStatus::Running
        {
            self.state
                .lock()
                .unwrap()
                .add_log(LogLevel::Warning, "Scanner already running");
            return Ok(());
        }

        let (hosts_file, concurrency) = {
            let mut s = self.state.lock().unwrap();
            let hosts_file =
                s.field_hosts_file.value.trim().to_string();
            let concurrency = s
                .field_concurrency
                .value
                .trim()
                .parse::<usize>()
                .unwrap_or(50);
            s.scan_status = ScanStatus::Running;
            s.scan_results.clear();
            s.scan_progress = 0.0;
            s.scan_done = 0;
            s.scan_total = 0;
            s.add_log(
                LogLevel::Info,
                format!(
                    "Scanning '{}' concurrency={}",
                    hosts_file, concurrency
                ),
            );
            (hosts_file, concurrency)
        };

        let state_clone = Arc::clone(&self.state);
        let event_tx_main = self.event_tx.clone();

        let handle = tokio::spawn(async move {
            let scanner = SniScanner::new(concurrency);

            let hosts =
                match tokio::fs::read_to_string(&hosts_file).await {
                    Ok(c) => c
                        .lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| {
                            !l.is_empty() && !l.starts_with('#')
                        })
                        .collect::<Vec<_>>(),
                    Err(e) => {
                        let mut s = state_clone.lock().unwrap();
                        s.scan_status =
                            ScanStatus::Error(e.to_string());
                        s.add_log(
                            LogLevel::Error,
                            format!(
                                "Cannot read '{}': {}",
                                hosts_file, e
                            ),
                        );
                        return;
                    }
                };

            if hosts.is_empty() {
                let mut s = state_clone.lock().unwrap();
                s.scan_status = ScanStatus::Error(
                    "Hosts file is empty".to_string(),
                );
                s.add_log(
                    LogLevel::Error,
                    "Hosts file is empty or has no valid entries",
                );
                return;
            }

            {
                let mut s = state_clone.lock().unwrap();
                s.scan_total = hosts.len();
                s.add_log(
                    LogLevel::Info,
                    format!("{} hosts to scan", hosts.len()),
                );
            }

            let _ =
                event_tx_main.send(AppEvent::ScanStarted(hosts.len()));

            let event_tx_cb = event_tx_main.clone();
            let results = scanner
                .scan_hosts(hosts, move |result| {
                    let _ = event_tx_cb
                        .send(AppEvent::ScanResult(result));
                })
                .await;

            let working =
                results.iter().filter(|r| r.is_working).count();
            {
                let mut s = state_clone.lock().unwrap();
                s.scan_status = ScanStatus::Completed;
                s.scan_progress = 1.0;
                s.add_log(
                    LogLevel::Success,
                    format!(
                        "Done — {}/{} working",
                        working,
                        results.len()
                    ),
                );
            }
            let _ = event_tx_main.send(AppEvent::ScanCompleted);
        });

        self.scanner_handle = Some(handle);
        self.state.lock().unwrap().active_tab = AppTab::Results;
        Ok(())
    }

    fn stop_scan(&mut self) {
        if let Some(h) = self.scanner_handle.take() {
            h.abort();
        }
        let mut s = self.state.lock().unwrap();
        if s.scan_status == ScanStatus::Running {
            s.scan_status = ScanStatus::Idle;
            s.add_log(LogLevel::Warning, "Scan stopped by user");
        }
    }

    // ── Use selected SNI ──────────────────────

    fn use_selected_sni(&mut self) {
        let selected = {
            let s = self.state.lock().unwrap();
            s.scan_results.get(s.selected_result).cloned()
        };
        match selected {
            Some(r) if r.is_working => {
                let mut s = self.state.lock().unwrap();
                s.field_sni.set(&r.host);
                s.sni_host = r.host.clone();
                s.active_tab = AppTab::Dashboard;
                s.active_field = ActiveField::Target;
                s.add_log(
                    LogLevel::Success,
                    format!("SNI set to: {}", r.host),
                );
            }
            Some(_) => {
                self.state.lock().unwrap().add_log(
                    LogLevel::Warning,
                    "Selected host is not working — choose a ✓ host",
                );
            }
            None => {}
        }
    }

    // ── Internal event handler ────────────────

    async fn handle_app_event(
        &mut self,
        event: AppEvent,
    ) -> Result<()> {
        match event {
            AppEvent::ScanResult(result) => {
                let mut s = self.state.lock().unwrap();
                s.scan_done += 1;
                if s.scan_total > 0 {
                    s.scan_progress =
                        s.scan_done as f64 / s.scan_total as f64;
                }
                if result.is_working {
                    s.add_log(
                        LogLevel::Success,
                        format!(
                            "✓ {} — {}ms",
                            result.host, result.latency_ms
                        ),
                    );
                }
                s.scan_results.push(result);
                s.scan_results.sort_by(|a, b| {
                    b.is_working
                        .cmp(&a.is_working)
                        .then(a.latency_ms.cmp(&b.latency_ms))
                });
            }
            AppEvent::ScanStarted(total) => {
                self.state.lock().unwrap().scan_total = total;
            }
            AppEvent::ScanCompleted => {
                let mut s = self.state.lock().unwrap();
                s.scan_status = ScanStatus::Completed;
                s.scan_progress = 1.0;
            }
            AppEvent::ProxyConnection { bytes, active } => {
                let mut s = self.state.lock().unwrap();
                s.bytes_transferred += bytes;
                s.connections_active = active;
                if bytes > 0 {
                    s.connections_total += 1;
                }
            }
            AppEvent::Log(level, msg) => {
                self.state.lock().unwrap().add_log(level, msg);
            }
        }
        Ok(())
    }
}
