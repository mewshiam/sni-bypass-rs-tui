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

pub struct EventHandler;

impl EventHandler {
    pub fn new(_tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self
    }
}