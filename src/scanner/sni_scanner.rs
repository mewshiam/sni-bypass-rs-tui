use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub host: String,
    pub is_working: bool,
    pub latency_ms: u64,
    pub tls_ok: bool,
    pub http_ok: bool,
    pub error: Option<String>,
}

pub struct SniScanner {
    concurrency: usize,
    timeout_secs: u64,
    ports: Vec<u16>,
}

impl SniScanner {
    pub fn new(concurrency: usize) -> Self {
        Self {
            concurrency,
            timeout_secs: 5,
            ports: vec![443, 80, 8080],
        }
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn with_ports(mut self, ports: Vec<u16>) -> Self {
        self.ports = ports;
        self
    }

    pub async fn scan_from_file(&self, path: &str) -> Result<Vec<ScanResult>> {
        let content = tokio::fs::read_to_string(path).await?;
        let hosts: Vec<String> = content
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(|l| l.trim().to_string())
            .collect();

        Ok(self.scan_hosts(hosts, |_| {}).await)
    }

    pub async fn scan_hosts<F>(
        &self,
        hosts: Vec<String>,
        on_result: F,
    ) -> Vec<ScanResult>
    where
        F: Fn(ScanResult) + Send + Sync + 'static,
    {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let on_result = Arc::new(on_result);
        let timeout_secs = self.timeout_secs;
        let ports = self.ports.clone();

        let mut tasks = Vec::with_capacity(hosts.len());

        for host in hosts {
            let sem = Arc::clone(&semaphore);
            let cb = Arc::clone(&on_result);
            let ports = ports.clone();

            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                let result = scan_single_host(&host, timeout_secs, &ports).await;
                cb(result.clone());
                Some(result)
            });
            tasks.push(task);
        }

        let mut results = Vec::new();
        for task in tasks {
            if let Ok(Some(result)) = task.await {
                results.push(result);
            }
        }

        results
    }
}

async fn scan_single_host(
    host: &str,
    timeout_secs: u64,
    ports: &[u16],
) -> ScanResult {
    let start = Instant::now();
    let dur = Duration::from_secs(timeout_secs);

    // Try HTTPS/TLS first
    let tls_result = timeout(dur, try_tls_connect(host, 443)).await;
    let (tls_ok, latency_ms) = match tls_result {
        Ok(Ok(ms)) => (true, ms),
        _ => (false, 0),
    };

    // Try HTTP fallback
    let http_result = if !tls_ok {
        timeout(dur, try_http_connect(host, 80)).await.ok()
            .and_then(|r| r.ok())
    } else {
        Some(latency_ms)
    };
    let http_ok = http_result.is_some();
    let effective_latency = if tls_ok {
        latency_ms
    } else if let Some(ms) = http_result {
        ms
    } else {
        start.elapsed().as_millis() as u64
    };

    let is_working = tls_ok || http_ok;

    ScanResult {
        host: host.to_string(),
        is_working,
        latency_ms: effective_latency,
        tls_ok,
        http_ok,
        error: if is_working { None } else { Some("Connection failed".to_string()) },
    }
}

async fn try_tls_connect(host: &str, port: u16) -> Result<u64> {
    use rustls::{ClientConfig, RootCertStore};
    use tokio_rustls::TlsConnector;
    use std::sync::Arc;

    let start = Instant::now();
    let addr = format!("{}:{}", host, port);

    let stream = TcpStream::connect(&addr).await?;

    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())?;

    let mut tls_stream = connector.connect(server_name, stream).await?;

    // Send minimal HTTP request
    let request = format!(
        "HEAD / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        host
    );
    tls_stream.write_all(request.as_bytes()).await?;

    let mut buf = [0u8; 256];
    let _ = tls_stream.read(&mut buf).await;

    Ok(start.elapsed().as_millis() as u64)
}

async fn try_http_connect(host: &str, port: u16) -> Result<u64> {
    let start = Instant::now();
    let addr = format!("{}:{}", host, port);

    let mut stream = TcpStream::connect(&addr).await?;

    let request = format!(
        "HEAD / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        host
    );
    stream.write_all(request.as_bytes()).await?;

    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).await?;

    if n > 0 && buf.starts_with(b"HTTP/") {
        Ok(start.elapsed().as_millis() as u64)
    } else {
        Err(anyhow::anyhow!("Not HTTP response"))
    }
}
