# sni-bypass-rs

A dual-layer DPI bypass tool for v2ray/xray connections through Cloudflare.

Built on top of [therealaleph/sni-spoofing-rust](https://github.com/therealaleph/sni-spoofing-rust)
(which itself is a Rust port of [patterniha/SNI-Spoofing](https://github.com/patterniha/SNI-Spoofing)).

---

## What's new vs the original

The original tool implements **one** bypass technique: fake ClientHello TCP desync.

This tool adds a **second independent layer**: TLS ClientHello fragmentation.

| Mode | How it works | When to use |
|---|---|---|
| `fake_sni` | Original: inject fake ClientHello with out-of-window seq#. DPI whitelists the SNI, server drops the packet. | DPI has a whitelist of allowed SNIs to exploit. |
| `fragment` | Split the real ClientHello across 2 TCP segments. DPI can't parse the SNI from an incomplete record, defaults to allow. | DPI whitelist is gone / changed. No fake SNI needed. |
| `dual` (**default**) | Both techniques applied together. Fake desync first, then real ClientHello fragmented. | Best bypass rate. Works even if one layer fails. |

---

## How fragmentation works

```
Client sends: [TLS ClientHello — 517 bytes]

Without fragmentation:
  TCP segment → [0x16 0x03 0x01 ... SNI="myserver.example.com" ...]
  DPI reads SNI → checks whitelist → blocks if not listed

With fragmentation (frag_split=1):
  TCP segment 1 → [0x16]                        ← just 1 byte, incomplete record
  (1 ms sleep)
  TCP segment 2 → [0x03 0x01 ... SNI=... ]      ← rest of ClientHello

  DPI receives segment 1:
    → sees 0x16 (TLS record header byte 1 only)
    → cannot parse SNI — record is incomplete
    → times out or defaults to ALLOW

  Server's TCP stack:
    → buffers both segments, reassembles by sequence number
    → sees complete, valid ClientHello — TLS handshake succeeds
```

The key insight: DPI is a simplified parser under time pressure. It cannot hold every partial
TCP stream in a reassembly buffer indefinitely. Sending just 1 byte of the TLS record header
guarantees DPI never sees a parseable SNI field.

---

## Requirements

Same as the original:

- **Linux**: `sudo` or `CAP_NET_RAW` (uses AF_PACKET raw sockets)
- **macOS**: `sudo` (uses BPF)
- **Windows**: Administrator (uses WinDivert driver)

Your server must be behind Cloudflare (CDN-based VLESS/VMess configs).

---

## Build

```bash
cargo build --release
# Binary: target/release/sni-bypass-rs
```

---

## Usage

```bash
# Linux / macOS
sudo ./sni-bypass-rs config.json

# Windows (run as Administrator)
sni-bypass-rs.exe config.json
```

Then point your v2ray/xray client at `127.0.0.1:1080` (or whatever `listen` is set to).

---

## config.json

```json
{
  "listeners": [
    {
      "listen":        "127.0.0.1:1080",
      "connect":       "1.2.3.4:443",
      "fake_sni":      "www.google.com",
      "mode":          "dual",
      "frag_split":    1,
      "frag_delay_ms": 1
    }
  ]
}
```

| Field | Required | Default | Description |
|---|---|---|---|
| `listen` | yes | — | Local address to accept connections on |
| `connect` | yes | — | Upstream address (your Cloudflare-fronted server IP:port) |
| `fake_sni` | yes | — | Domain injected into the fake ClientHello (must be DPI-whitelisted). Ignored in `fragment` mode. |
| `mode` | no | `"dual"` | `"fake_sni"`, `"fragment"`, or `"dual"` |
| `frag_split` | no | `1` | Bytes in fragment 1. `1` is the most effective — keeps the TLS record header incomplete. |
| `frag_delay_ms` | no | `1` | Milliseconds between fragments. Increase to `5`–`10` on high-latency or lossy links. |

Multiple listeners are supported — add more objects to the array.

---

## Log levels

```bash
# Silent (default)
sudo ./sni-bypass-rs config.json

# Show connection events
RUST_LOG=info sudo ./sni-bypass-rs config.json

# Full debug output
RUST_LOG=debug sudo ./sni-bypass-rs config.json
```

---

## Credits

- Original idea and Windows implementation: [@patterniha](https://github.com/patterniha/SNI-Spoofing)
- Rust + Linux port: [@therealaleph](https://github.com/therealaleph/sni-spoofing-rust)
- Fragmentation layer: this repo
