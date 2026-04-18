use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use crate::tui::app::AppState;
use crate::tui::events::AppEvent;

pub struct ProxyServer {
    port: u16,
    target_host: String,
    sni_host: String,
}

impl ProxyServer {
    pub fn new(port: u16, target_host: String, sni_host: String) -> Self {
        Self { port, target_host, sni_host }
    }

    pub async fn run(self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        let target = Arc::new(self.target_host);
        let sni = Arc::new(self.sni_host);

        tracing::info!("Proxy listening on {}", addr);

        loop {
            let (client, peer) = listener.accept().await?;
            let target = Arc::clone(&target);
            let sni = Arc::clone(&sni);

            tokio::spawn(async move {
                if let Err(e) = handle_connection(client, &target, &sni, peer.to_string()).await {
                    tracing::error!("Connection error from {}: {}", peer, e);
                }
            });
        }
    }

    pub async fn run_with_stats(
        self,
        state: Arc<Mutex<AppState>>,
        event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        let target = Arc::new(self.target_host);
        let sni = Arc::new(self.sni_host);
        let active_conns = Arc::new(AtomicU64::new(0));

        tracing::info!("Proxy with stats listening on {}", addr);

        loop {
            let (client, peer) = listener.accept().await?;
            let target = Arc::clone(&target);
            let sni = Arc::clone(&sni);
            let event_tx = event_tx.clone();
            let active = Arc::clone(&active_conns);

            tokio::spawn(async move {
                active.fetch_add(1, Ordering::Relaxed);
                let active_count = active.load(Ordering::Relaxed);

                let _ = event_tx.send(AppEvent::ProxyConnection {
                    bytes: 0,
                    active: active_count,
                });

                match handle_connection(client, &target, &sni, peer.to_string()).await {
                    Ok(bytes) => {
                        let active_count = active.fetch_sub(1, Ordering::Relaxed) - 1;
                        let _ = event_tx.send(AppEvent::ProxyConnection {
                            bytes,
                            active: active_count,
                        });
                    }
                    Err(e) => {
                        let active_count = active.fetch_sub(1, Ordering::Relaxed) - 1;
                        tracing::error!("Error from {}: {}", peer, e);
                        let _ = event_tx.send(AppEvent::ProxyConnection {
                            bytes: 0,
                            active: active_count,
                        });
                    }
                }
            });
        }
    }
}

async fn handle_connection(
    mut client: TcpStream,
    target: &str,
    sni: &str,
    peer: String,
) -> Result<u64> {
    use rustls::{ClientConfig, RootCertStore};
    use tokio_rustls::TlsConnector;

    // Read CONNECT request or first bytes
    let mut buf = [0u8; 4096];
    let n = timeout(Duration::from_secs(10), client.read(&mut buf)).await??;

    if n == 0 {
        return Ok(0);
    }

    let request = std::str::from_utf8(&buf[..n]).unwrap_or("");

    // Handle HTTP CONNECT (for HTTPS proxy)
    if request.starts_with("CONNECT") {
        let _ = client.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await;
        return handle_tls_tunnel(client, target, sni).await;
    }

    // Handle direct HTTP
    handle_http_proxy(client, target, &buf[..n]).await
}

async fn handle_tls_tunnel(
    mut client: TcpStream,
    target: &str,
    sni: &str,
) -> Result<u64> {
    use rustls::{ClientConfig, RootCertStore};
    use tokio_rustls::TlsConnector;

    // Connect to upstream with SNI bypass
    let addr = format!("{}:443", target);
    let upstream = TcpStream::connect(&addr).await?;

    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let server_name = rustls::pki_types::ServerName::try_from(sni.to_string())?;

    let mut tls_upstream = connector.connect(server_name, upstream).await?;

    // Bidirectional copy
    let (client_read, client_write) = client.split();
    // For simplicity, do raw bidirectional copy
    let mut bytes = 0u64;

    // Use tokio copy_bidirectional workaround for TLS
    // We'll do a simple manual approach
    let mut client_buf = vec![0u8; 8192];
    let mut upstream_buf = vec![0u8; 8192];

    // This is a simplified version - in production you'd want
    // proper bidirectional async copying
    loop {
        tokio::select! {
            result = client.read(&mut client_buf) => {
                match result {
                    Ok(0) => break,
                    Ok(n) => {
                        tls_upstream.write_all(&client_buf[..n]).await?;
                        bytes += n as u64;
                    }
                    Err(_) => break,
                }
            }
            result = tls_upstream.read(&mut upstream_buf) => {
                match result {
                    Ok(0) => break,
                    Ok(n) => {
                        client.write_all(&upstream_buf[..n]).await?;
                        bytes += n as u64;
                    }
                    Err(_) => break,
                }
            }
        }
    }

    Ok(bytes)
}

async fn handle_http_proxy(
    mut client: TcpStream,
    target: &str,
    request: &[u8],
) -> Result<u64> {
    let addr = format!("{}:80", target);
    let mut upstream = TcpStream::connect(&addr).await?;
    upstream.write_all(request).await?;

    let mut bytes = 0u64;
    let mut buf = vec![0u8; 8192];

    loop {
        tokio::select! {
            n = client.read(&mut buf) => {
                match n {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        upstream.write_all(&buf[..n]).await?;
                        bytes += n as u64;
                    }
                }
            }
            n = upstream.read(&mut buf) => {
                match n {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        client.write_all(&buf[..n]).await?;
                        bytes += n as u64;
                    }
                }
            }
        }
    }

    Ok(bytes)
}
