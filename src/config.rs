use std::net::SocketAddr;

use serde::Deserialize;

/// Bypass mode for each listener.
///
/// - `"fake_sni"` — original behaviour: inject a fake ClientHello with a
///   wrong TCP sequence number so DPI whitelists the flow, then relay normally.
/// - `"fragment"` — split the real TLS ClientHello across two raw TCP segments
///   so DPI cannot parse the SNI field and falls back to allowing the flow.
/// - `"dual"` — both techniques applied together (recommended): fake SNI
///   desync first, then the real ClientHello forwarded in two fragments.
///   Gives the highest bypass rate against stateful DPI.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BypassMode {
    FakeSni,
    Fragment,
    Dual,
}

impl Default for BypassMode {
    fn default() -> Self {
        BypassMode::Dual
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listeners: Vec<ListenerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ListenerConfig {
    /// Local address to accept connections on.
    pub listen: SocketAddr,
    /// Upstream address to forward to (e.g. your Cloudflare-fronted server).
    pub connect: SocketAddr,
    /// Domain name placed in the fake TLS ClientHello sent to DPI.
    /// Ignored when mode is `"fragment"`.
    pub fake_sni: String,
    /// Bypass mode. Defaults to `"dual"` when omitted.
    #[serde(default)]
    pub mode: BypassMode,
    /// Number of bytes to put in fragment-1 of the real ClientHello.
    /// Defaults to 1 (just the first byte of the TLS record header).
    /// Valid range: 1..ClientHello_length-1.
    #[serde(default = "default_frag_split")]
    pub frag_split: usize,
    /// Milliseconds to sleep between fragment-1 and fragment-2.
    /// Defaults to 1. Increase to 5–10 on unreliable links.
    #[serde(default = "default_frag_delay_ms")]
    pub frag_delay_ms: u64,
}

fn default_frag_split() -> usize { 1 }
fn default_frag_delay_ms() -> u64 { 1 }

pub fn load(path: &str) -> Result<Config, crate::error::ConfigError> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| crate::error::ConfigError::Io(path.to_string(), e))?;
    let cfg: Config = serde_json::from_str(&data)
        .map_err(|e| crate::error::ConfigError::Parse(path.to_string(), e))?;
    if cfg.listeners.is_empty() {
        return Err(crate::error::ConfigError::Empty);
    }
    for lc in &cfg.listeners {
        if lc.fake_sni.len() > 219 {
            return Err(crate::error::ConfigError::SniTooLong(lc.fake_sni.clone()));
        }
        if lc.frag_split == 0 {
            return Err(crate::error::ConfigError::BadFragSplit);
        }
    }
    Ok(cfg)
}
