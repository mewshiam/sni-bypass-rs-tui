use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use crate::tui::app::AppState;
use crate::tui::events::AppEvent;

pub struct ProxyServer {
    port: u16,
    target_host: String,
    sni_host: String,
    fragment_enabled: bool,
    frag_split: usize,
    frag_delay_ms: u64,
}

impl ProxyServer {
    pub fn new(
        port: u16,
        target_host: String,
        sni_host: String,
        fragment_enabled: bool,
        frag_split: usize,
        frag_delay_ms: u64,
    ) -> Self {
        Self {
            port,
            target_host,
            sni_host,
            fragment_enabled,
            frag_split,
            frag_delay_ms,
        }
    }

    /// Headless mode — no stats tracking
    pub async fn run(self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        let target = Arc::new(self.target_host);
        let sni = Arc::new(self.sni_host);
        let fragment_enabled = self.fragment_enabled;
        let frag_split = self.frag_split;
        let frag_delay_ms = self.frag_delay_ms;

        tracing::info!("Proxy listening on {}", addr);

        loop {
            let (client, peer) = listener.accept().await?;
            let target = Arc::clone(&target);
            let sni = Arc::clone(&sni);

            tokio::spawn(async move {
                if let Err(e) =
                    handle_connection(
                        client,
                        &target,
                        &sni,
                        fragment_enabled,
                        frag_split,
                        frag_delay_ms,
                    )
                    .await
                {
                    tracing::error!(
                        "Connection error from {}: {}",
                        peer, e
                    );
                }
            });
        }
    }

    /// TUI mode — tracks stats and sends events
    pub async fn run_with_stats(
        self,
        _state: Arc<Mutex<AppState>>,
        event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        let target = Arc::new(self.target_host);
        let sni = Arc::new(self.sni_host);
        let fragment_enabled = self.fragment_enabled;
        let frag_split = self.frag_split;
        let frag_delay_ms = self.frag_delay_ms;
        let active_conns = Arc::new(AtomicU64::new(0));

        tracing::info!("Proxy (with stats) listening on {}", addr);

        loop {
            let (client, peer) = listener.accept().await?;
            let target = Arc::clone(&target);
            let sni = Arc::clone(&sni);
            let event_tx = event_tx.clone();
            let active = Arc::clone(&active_conns);

            tokio::spawn(async move {
                let current = active.fetch_add(1, Ordering::Relaxed) + 1;
                let _ = event_tx.send(AppEvent::ProxyConnection {
                    bytes: 0,
                    active: current,
                });

                let bytes =
                    match handle_connection(
                        client,
                        &target,
                        &sni,
                        fragment_enabled,
                        frag_split,
                        frag_delay_ms,
                    )
                    .await {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::error!(
                                "Error from {}: {}",
                                peer, e
                            );
                            0
                        }
                    };

                let current = active.fetch_sub(1, Ordering::Relaxed) - 1;
                let _ = event_tx.send(AppEvent::ProxyConnection {
                    bytes,
                    active: current,
                });
            });
        }
    }
}

// ─────────────────────────────────────────────
// Connection handler
// ─────────────────────────────────────────────

async fn handle_connection(
    mut client: TcpStream,
    target: &str,
    sni: &str,
    fragment_enabled: bool,
    frag_split: usize,
    frag_delay_ms: u64,
) -> Result<u64> {
    let mut buf = vec![0u8; 4096];
    let n = timeout(
        Duration::from_secs(10),
        client.read(&mut buf),
    )
    .await??;

    if n == 0 {
        return Ok(0);
    }

    let request = std::str::from_utf8(&buf[..n]).unwrap_or("");

    if request.starts_with("CONNECT") {
        // HTTPS tunnel
        client
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .await?;
        handle_tls_tunnel(
            client,
            target,
            sni,
            fragment_enabled,
            frag_split,
            frag_delay_ms,
        )
        .await
    } else {
        // Plain HTTP
        handle_http_proxy(client, target, &buf[..n]).await
    }
}

// ─────────────────────────────────────────────
// TLS tunnel (SNI bypass)
// ─────────────────────────────────────────────

async fn handle_tls_tunnel(
    mut client: TcpStream,
    target: &str,
    _sni: &str,
    fragment_enabled: bool,
    frag_split: usize,
    frag_delay_ms: u64,
) -> Result<u64> {
    let addr = format!("{}:443", target);
    let mut upstream = TcpStream::connect(&addr).await?;
    upstream.set_nodelay(true)?;
    client.set_nodelay(true)?;

    if fragment_enabled {
        let mut first = vec![0u8; 16 * 1024];
        let n = client.read(&mut first).await?;
        if n == 0 {
            return Ok(0);
        }

        let split_at =
            frag_split.clamp(1, n.saturating_sub(1).max(1));
        upstream.write_all(&first[..split_at]).await?;
        tokio::time::sleep(Duration::from_millis(frag_delay_ms)).await;
        upstream.write_all(&first[split_at..n]).await?;

        return relay_bidirectional(client, upstream, n as u64).await;
    }

    relay_bidirectional(client, upstream, 0).await
}

async fn relay_bidirectional(
    mut client: TcpStream,
    mut upstream: TcpStream,
    initial_bytes: u64,
) -> Result<u64> {
    let (from_client, from_server) =
        tokio::io::copy_bidirectional(&mut client, &mut upstream)
            .await?;
    Ok(initial_bytes + from_client + from_server)
}

// ─────────────────────────────────────────────
// Plain HTTP proxy
// ─────────────────────────────────────────────

async fn handle_http_proxy(
    mut client: TcpStream,
    target: &str,
    request: &[u8],
) -> Result<u64> {
    let addr = format!("{}:80", target);
    let mut upstream = TcpStream::connect(&addr).await?;
    upstream.write_all(request).await?;

    // Two separate buffers — one per direction
    let mut client_buf = vec![0u8; 8192];
    let mut upstream_buf = vec![0u8; 8192];
    let mut bytes: u64 = 0;

    loop {
        tokio::select! {
            n = client.read(&mut client_buf) => {
                match n {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        upstream.write_all(&client_buf[..n]).await?;
                        bytes += n as u64;
                    }
                }
            }
            n = upstream.read(&mut upstream_buf) => {
                match n {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        client.write_all(&upstream_buf[..n]).await?;
                        bytes += n as u64;
                    }
                }
            }
        }
    }

    Ok(bytes)
}
