use std::time::Duration;

use tokio::io::{copy_bidirectional, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, warn};

/// Standard bidirectional relay used after the bypass handshake completes.
pub async fn relay(mut client: TcpStream, mut upstream: TcpStream) -> Result<(), std::io::Error> {
    let (c2u, u2c) = copy_bidirectional(&mut client, &mut upstream).await?;
    debug!(c2u, u2c, "relay finished");
    Ok(())
}

/// Send the real TLS ClientHello to `upstream` in two separate TCP writes,
/// separated by a short sleep so the OS cannot coalesce them (Nagle bypass).
///
/// Fragment 1: the first `split_at` bytes of the ClientHello record header.
///   - DPI sees an incomplete TLS record and cannot extract the SNI field.
///   - It either times out, gives up, or defaults to allow.
///
/// Fragment 2: the remaining bytes, including the full SNI extension.
///   - The server's TCP stack reassembles both fragments in order using
///     sequence numbers and hands a complete ClientHello to TLS.
///
/// `TCP_NODELAY` must be set on the socket before calling this, otherwise
/// the OS may merge both writes into a single segment.
pub async fn send_fragmented_client_hello(
    upstream: &mut TcpStream,
    client_hello: &[u8],
    split_at: usize,
    delay_ms: u64,
) -> Result<(), std::io::Error> {
    // Clamp split point — must be at least 1 and at most len-1.
    let split_at = split_at.clamp(1, client_hello.len().saturating_sub(1).max(1));

    let (frag1, frag2) = client_hello.split_at(split_at);

    debug!(
        frag1_bytes = frag1.len(),
        frag2_bytes = frag2.len(),
        delay_ms,
        "sending fragmented ClientHello"
    );

    // Send fragment 1 — just the beginning of the TLS record header.
    upstream.write_all(frag1).await?;
    upstream.flush().await?;

    // Sleep so the kernel flushes frag1 as a standalone TCP segment before
    // we hand it frag2.  Even 1 ms is enough on a LAN; increase for lossy links.
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;

    // Send fragment 2 — the rest of the ClientHello including the SNI extension.
    upstream.write_all(frag2).await?;
    upstream.flush().await?;

    debug!("fragmented ClientHello sent successfully");
    Ok(())
}

/// Relay with optional initial-fragment bypass for the first upstream write.
///
/// Used in Fragment and Dual modes: after any fake-SNI desync has been
/// confirmed (or skipped), the first data the client sends to the proxy is
/// its real TLS ClientHello.  We intercept it, send it fragmented, then
/// hand off to the normal bidirectional relay for the rest of the session.
pub async fn relay_with_fragment(
    mut client: TcpStream,
    mut upstream: TcpStream,
    split_at: usize,
    delay_ms: u64,
) -> Result<(), std::io::Error> {
    use tokio::io::AsyncReadExt;

    // Read the client's first write — this should be the TLS ClientHello.
    // We use a 16 KB buffer; a ClientHello is always well under 2 KB.
    let mut buf = vec![0u8; 16384];
    let n = client.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let client_hello = &buf[..n];

    // Detect TLS ClientHello (record type 0x16, version 0x03xx).
    let looks_like_tls = n >= 3 && client_hello[0] == 0x16 && client_hello[1] == 0x03;

    if looks_like_tls {
        debug!("TLS ClientHello detected ({} bytes), fragmenting", n);
        send_fragmented_client_hello(&mut upstream, client_hello, split_at, delay_ms).await?;
    } else {
        // Not a TLS handshake (maybe plain TCP proxy usage) — forward as-is.
        warn!("first client write doesn't look like TLS ClientHello, forwarding normally");
        upstream.write_all(client_hello).await?;
        upstream.flush().await?;
    }

    // Standard relay for the rest of the session.
    let (c2u, u2c) = copy_bidirectional(&mut client, &mut upstream).await?;
    debug!(c2u, u2c, "relay finished");
    Ok(())
}
