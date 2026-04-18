use tokio::sync::mpsc;
use crate::scanner::ScanResult;
use super::app::LogLevel;

#[derive(Debug)]
pub enum AppEvent {
    ScanResult(ScanResult),
    ScanStarted(usize),
    ScanCompleted,
    ProxyConnection { bytes: u64, active: u64 },
    Log(LogLevel, String),
}

// EventHandler does NOT read keyboard events.
// Keyboard input is handled exclusively in app.rs main_loop
// via crossterm::event::read() — having two readers causes
// every key to be processed twice.
pub struct EventHandler;

impl EventHandler {
    pub fn new(_tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self
    }
}
