use std::net::IpAddr;

use tokio::sync::mpsc;

use crate::config::BypassMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnId {
    pub src_ip: IpAddr,
    pub src_port: u16,
    pub dst_ip: IpAddr,
    pub dst_port: u16,
}

#[derive(Debug)]
pub enum SnifferResult {
    FakeConfirmed,
    /// Fake SNI layer was skipped (fragment-only mode): relay can start immediately.
    ReadyImmediate,
    Failed(String),
}

/// Parameters the sniffer needs to know about each connection's bypass mode.
#[derive(Debug, Clone)]
pub struct FragmentConfig {
    /// How many bytes of the real ClientHello go into fragment 1.
    pub split_at: usize,
    /// Delay between the two fragment sends, in milliseconds.
    pub delay_ms: u64,
    /// Which mode is active for this connection.
    pub mode: BypassMode,
}

pub struct Registration {
    pub conn_id: ConnId,
    /// Pre-built fake ClientHello payload (used in FakeSni and Dual modes).
    pub fake_payload: Vec<u8>,
    pub frag_cfg: FragmentConfig,
    pub result_tx: mpsc::Sender<SnifferResult>,
    pub registered_tx: tokio::sync::oneshot::Sender<()>,
}

pub struct Deregistration {
    pub conn_id: ConnId,
}

pub enum SnifferCommand {
    Register(Registration),
    Deregister(Deregistration),
}
