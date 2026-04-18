# SNI Bypass RS-TUI

<div align="center">

```
███████╗███╗   ██╗██╗    ██████╗ ██╗   ██╗██████╗  █████╗ ███████╗███████╗
██╔════╝████╗  ██║██║    ██╔══██╗╚██╗ ██╔╝██╔══██╗██╔══██╗██╔════╝██╔════╝
███████╗██╔██╗ ██║██║    ██████╔╝ ╚████╔╝ ██████╔╝███████║███████╗███████╗
╚════██║██║╚██╗██║██║    ██╔══██╗  ╚██╔╝  ██╔═══╝ ██╔══██║╚════██║╚════██║
███████║██║ ╚████║██║    ██████╔╝   ██║   ██║     ██║  ██║███████║███████║
╚══════╝╚═╝  ╚═══╝╚═╝   ╚═════╝    ╚═╝   ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝
```

**A fast, cross-platform SNI bypass proxy with TUI — built in Rust**

[![Release](https://img.shields.io/github/v/release/mewshiam/sni-bypass-rs-tui?style=flat-square&color=cyan)](https://github.com/mewshiam/sni-bypass-rs-tui/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/mewshiam/sni-bypass-rs-tui/release.yaml?style=flat-square&label=build)](https://github.com/mewshiam/sni-bypass-rs-tui/actions)
[![License](https://img.shields.io/github/license/mewshiam/sni-bypass-rs-tui?style=flat-square&color=blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange?style=flat-square&logo=rust)](https://rustup.rs)
[![Termux](https://img.shields.io/badge/Android-Termux-green?style=flat-square&logo=android)](https://termux.dev)

</div>

---

## Table of Contents

- [What is this?](#what-is-this)
- [Features](#features)
- [Preview](#preview)
- [Installation](#installation)
  - [Linux / macOS](#linux--macos)
  - [Windows](#windows)
  - [Android (Termux)](#android-termux)
  - [Build from source](#build-from-source)
- [Usage](#usage)
  - [TUI Mode](#tui-mode)
  - [CLI / Headless Mode](#cli--headless-mode)
- [TUI Guide](#tui-guide)
  - [Dashboard Tab](#dashboard-tab)
  - [Scanner Tab](#scanner-tab)
  - [Results Tab](#results-tab)
  - [Logs Tab](#logs-tab)
  - [Help Tab](#help-tab)
- [Key Bindings](#key-bindings)
- [SNI Scanner](#sni-scanner)
- [Hosts File Format](#hosts-file-format)
- [How It Works](#how-it-works)
- [Supported Platforms](#supported-platforms)
- [FAQ](#faq)
- [Contributing](#contributing)
- [License](#license)

---

## What is this?

**SNI Bypass RS-TUI** is a terminal-based proxy tool that lets you bypass
SNI-based traffic filtering and censorship. It works by manipulating the
**Server Name Indication (SNI)** field in TLS handshakes — allowing you to
connect to a target host while presenting a different (allowed) SNI to
deep packet inspection systems.

Built entirely in Rust for speed, safety, and tiny binary size. Comes with
a full **interactive TUI**, an integrated **SNI scanner** to discover working
hosts, and **Termux/Android** support out of the box.

---

## Features

```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  🖥️  Full TUI         Interactive terminal interface     │
│  🔍  SNI Scanner      Concurrent host discovery         │
│  🛡️  SNI Bypass       HTTPS/HTTP proxy with SNI spoof   │
│  📱  Termux Ready     Native Android/Termux support     │
│  📊  Live Stats       Real-time connection metrics      │
│  ⚡  Fast             Async Rust, zero overhead         │
│  🔒  rustls           No OpenSSL dependency             │
│  🌍  Cross-platform   Linux, Windows, macOS, Android    │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## Preview

```
┌─ ◈ SNI BYPASS RS-TUI ──────┬─ 1:Dashboard  2:Scanner  3:Results  4:Logs  5:Help ─┐
│                             │                                                      │
│  ┌─ ⚙ Proxy Configuration ─────────────────┐  ┌─ ◈ Proxy Status ───────────────┐ │
│  │                                          │  │                                 │ │
│  │  Target Host:  ▶ speedtest.net           │  │  Status:  ▶ RUNNING             │ │
│  │  SNI Host   :    cdn.cloudflare.net      │  │  Listen:  0.0.0.0:8080          │ │
│  │  Port       :    8080                    │  │  Target:  speedtest.net         │ │
│  │                                          │  │  SNI:     cdn.cloudflare.net    │ │
│  └──────────────────────────────────────────┘  └─────────────────────────────────┘ │
│                                                                                     │
│  ┌─ ◈ Statistics ──────────────────────────┐  ┌─ ◈ Quick Reference ─────────────┐ │
│  │                                          │  │                                 │ │
│  │  ↕ Transferred :  24.31 MB              │  │  [1-5]  Switch tabs             │ │
│  │  ⇌ Active Conns:  3                     │  │  [e/i]  Edit fields             │ │
│  │  ∑ Total Conns :  147                   │  │  [s]    Start/Stop proxy        │ │
│  │  ⚡ Req/sec     :  12.4                  │  │  [S]    Start SNI scan          │ │
│  │                                          │  │  [u]    Use selected SNI        │ │
│  └──────────────────────────────────────────┘  │  [?]    Help  [q] Quit         │ │
│                                                 └─────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────────────────────┤
│ NORMAL  ▶ PROXY ON  [q]uit  [?]help  [Tab]navigate                                 │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

```
┌─ ◈ SNI BYPASS RS-TUI ──────┬─ 1:Dashboard  2:Scanner  3:Results  4:Logs  5:Help ─┐
│                                                                                     │
│  ┌─ ◈ Scan Results [12/50 working] ─────────────────────────────────────────────┐  │
│  │  #    Host                       Latency    Status   TLS    HTTP              │  │
│  │ ─────────────────────────────────────────────────────────────────────────────│  │
│  │   1   cdn.cloudflare.net         43ms       ✓        ✓      ✓                │  │
│  │   2   workers.dev                67ms       ✓        ✓      ✓                │  │
│  │   3   pages.dev                  71ms       ✓        ✓      ✓                │  │
│  │ ► 4   storage.googleapis.com     89ms       ✓        ✓      ✓                │  │
│  │   5   ajax.googleapis.com        94ms       ✓        ✓      ✓                │  │
│  │   6   fonts.gstatic.com          102ms      ✓        ✓      ✗                │  │
│  │   7   badhost.example.com        timeout    ✗        ✗      ✗                │  │
│  │                                                                               │  │
│  └───────────────────────────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────────────────────────┐   │
│  │ [↑↓/jk] Navigate  [u] Use selected SNI  [PgUp/PgDn] Page  [g/G] Top/Bottom │   │
│  └──────────────────────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────────────────────┤
│ NORMAL  ▶ PROXY ON  ✓ SCAN DONE  [q]uit  [?]help  [Tab]navigate                   │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Installation

### Linux / macOS

Download the latest binary from the
[Releases](https://github.com/mewshiam/sni-bypass-rs-tui/releases) page:

```bash
# Linux x86_64
wget https://github.com/mewshiam/sni-bypass-rs-tui/releases/latest/download/sni-bypass-rs-tui-linux-x86_64.tar.gz
tar xzf sni-bypass-rs-tui-linux-x86_64.tar.gz
cd sni-bypass-rs-tui-linux-x86_64
chmod +x sni-bypass-rs-tui
./sni-bypass-rs-tui
```

```bash
# Linux ARM64
wget https://github.com/mewshiam/sni-bypass-rs-tui/releases/latest/download/sni-bypass-rs-tui-linux-aarch64.tar.gz
tar xzf sni-bypass-rs-tui-linux-aarch64.tar.gz
cd sni-bypass-rs-tui-linux-aarch64
chmod +x sni-bypass-rs-tui
./sni-bypass-rs-tui
```

```bash
# macOS Apple Silicon
wget https://github.com/mewshiam/sni-bypass-rs-tui/releases/latest/download/sni-bypass-rs-tui-macos-aarch64.tar.gz
tar xzf sni-bypass-rs-tui-macos-aarch64.tar.gz
cd sni-bypass-rs-tui-macos-aarch64
chmod +x sni-bypass-rs-tui
./sni-bypass-rs-tui
```

### Windows

Download `sni-bypass-rs-tui-windows-x86_64.zip` from the
[Releases](https://github.com/mewshiam/sni-bypass-rs-tui/releases) page,
extract, and run `sni-bypass-rs-tui.exe` in **Windows Terminal** or **CMD**.

> **Note:** Windows Terminal gives the best TUI experience.
> PowerShell and CMD both work fine.

### Android (Termux)

> Requires [Termux](https://f-droid.org/packages/com.termux/) from F-Droid.
> The Play Store version is outdated — use F-Droid.

**Option 1 — Download prebuilt binary (recommended):**

```bash
# Inside Termux
pkg install wget

# ARM64 (most modern Android devices)
wget https://github.com/mewshiam/sni-bypass-rs-tui/releases/latest/download/sni-bypass-rs-tui-android-aarch64-termux.tar.gz
tar xzf sni-bypass-rs-tui-android-aarch64-termux.tar.gz
cd sni-bypass-rs-tui-android-aarch64-termux
chmod +x sni-bypass-rs-tui
./sni-bypass-rs-tui
```

```bash
# ARMv7 (older 32-bit Android devices)
wget https://github.com/mewshiam/sni-bypass-rs-tui/releases/latest/download/sni-bypass-rs-tui-android-armv7-termux.tar.gz
tar xzf sni-bypass-rs-tui-android-armv7-termux.tar.gz
cd sni-bypass-rs-tui-android-armv7-termux
chmod +x sni-bypass-rs-tui
./sni-bypass-rs-tui
```

**Option 2 — Build from source in Termux:**

```bash
pkg update && pkg upgrade
pkg install rust openssl pkg-config git
git clone https://github.com/mewshiam/sni-bypass-rs-tui
cd sni-bypass-rs-tui
chmod +x build-termux.sh
./build-termux.sh
```

> **Tip:** Enable the **extra keys row** in Termux settings for easier
> navigation (`Volume Down + K` → Extra Keys).

### Build from source

Requires [Rust](https://rustup.rs) 1.75+

```bash
git clone https://github.com/mewshiam/sni-bypass-rs-tui
cd sni-bypass-rs-tui
cargo build --release
./target/release/sni-bypass-rs-tui
```

---

## Usage

### TUI Mode

Just run the binary with no arguments to launch the full TUI:

```bash
./sni-bypass-rs-tui
```

Optional flags to pre-fill values:

```bash
./sni-bypass-rs-tui --target speedtest.net --sni cdn.cloudflare.net --port 8080
```

### CLI / Headless Mode

For scripting or running without a terminal:

```bash
# Start proxy in headless mode
./sni-bypass-rs-tui --headless \
  --target speedtest.net \
  --sni cdn.cloudflare.net \
  --port 8080 \
  --fragment \
  --frag-split 1 \
  --frag-delay-ms 3

# Run scanner only (outputs to stdout)
./sni-bypass-rs-tui --scan-only \
  --hosts-file hosts.txt \
  --concurrency 100

# All flags
./sni-bypass-rs-tui --help
```

```
USAGE:
    sni-bypass-rs-tui [OPTIONS]

OPTIONS:
    -t, --target <HOST>          Target host to bypass
    -s, --sni <HOST>             SNI host to use (defaults to target)
    -p, --port <PORT>            Local proxy port [default: 8080]
    -f, --hosts-file <FILE>      Hosts file for SNI scanner
    -c, --concurrency <NUM>      Scanner concurrency [default: 50]
    -H, --headless               Run without TUI
        --scan-only              Run scanner only then exit
        --fragment               Force-enable CONNECT payload fragmentation
        --no-fragment            Force-disable CONNECT payload fragmentation
        --frag-split <BYTES>     Split point for first CONNECT payload
        --frag-delay-ms <MS>     Delay between first and second fragment
        --config <FILE>          Config file path [default: config.json]
        --print-config           Print current config and exit
    -h, --help                   Print help
    -V, --version                Print version
```

### Fragment mode (new)

The proxy now supports **first-payload fragmentation** for CONNECT/TLS tunnels.
When enabled, the first client payload is split and sent in two writes with a
small delay. This is useful in environments where TCP/TLS segmentation behavior
impacts SNI filtering.

Defaults:

- `fragment_enabled = false`
- `frag_split = 1`
- `frag_delay_ms = 1`

You can tune these from:

- `config.json` (`proxy.fragment_enabled`, `proxy.frag_split`, `proxy.frag_delay_ms`)
- CLI flags (`--fragment`, `--no-fragment`, `--frag-split`, `--frag-delay-ms`)

---

## TUI Guide

### Dashboard Tab

The main control panel. Set your proxy configuration and start/stop the proxy.

```
┌─ ⚙ Proxy Configuration ─┐    ┌─ ◈ Proxy Status ──────┐
│  Target Host: ...        │    │  Status:  ▶ RUNNING    │
│  SNI Host   : ...        │    │  Listen:  0.0.0.0:8080 │
│  Port       : 8080       │    │  Target:  ...          │
└──────────────────────────┘    └───────────────────────-┘

┌─ ◈ Statistics ───────────┐    ┌─ ◈ Quick Reference ───┐
│  ↕ Transferred: 24.31 MB │    │  [e] Edit fields       │
│  ⇌ Active Conns: 3       │    │  [s] Start/Stop proxy  │
│  ∑ Total Conns: 147      │    │  [S] Start scan        │
│  ⚡ Req/sec: 12.4         │    │  [?] Help  [q] Quit    │
└──────────────────────────┘    └────────────────────────┘
```

**Workflow:**
1. Press `e` to enter edit mode
2. Fill in **Target Host** (the site you want to reach)
3. Fill in **SNI Host** (the allowed host to spoof — or leave same as target)
4. Set **Port** (default `8080`)
5. Press `Esc` then `s` to start the proxy
6. Set your device/browser proxy to `127.0.0.1:8080`

---

### Scanner Tab

Scan a list of hosts to find working SNI candidates.

```
┌─ ◈ Scanner Configuration ────────────────────────┐
│  Hosts File  : hosts.txt                         │
│  Concurrency : 50                                │
│  [e] Edit  [S] Start Scan  [x] Stop Scan         │
└──────────────────────────────────────────────────┘

┌─ ◈ Progress [23/100] ────────────────────────────┐
│  ████████████░░░░░░░░░░  Scanning... (23%)        │
└──────────────────────────────────────────────────┘
```

**Workflow:**
1. Press `e` to set your hosts file path
2. Adjust concurrency (higher = faster, more CPU)
3. Press `S` to start — results appear live in the Results tab
4. Press `3` to jump to Results while scanning

---

### Results Tab

View all scan results sorted by working status then latency.

```
┌─ ◈ Scan Results [12/50 working] ──────────────────────────────┐
│  #    Host                    Latency   Status  TLS   HTTP     │
│ ──────────────────────────────────────────────────────────────│
│   1   cdn.cloudflare.net      43ms      ✓       ✓     ✓       │
│   2   workers.dev             67ms      ✓       ✓     ✓       │
│ ► 3   storage.googleapis.com  89ms      ✓       ✓     ✓       │  ← selected
│   4   badhost.example.com     timeout   ✗       ✗     ✗       │
└───────────────────────────────────────────────────────────────┘
```

- Navigate with `↑↓` or `j/k`
- Press `u` to instantly use the selected host as your SNI
- `g/G` to jump to top/bottom
- `PgUp/PgDn` for page scrolling

---

### Logs Tab

Real-time log output from both the proxy and scanner.

```
┌─ ◈ Logs [47/47] [AUTO] ───────────────────────────────────────┐
│  10:23:41 [INFO]    SNI Bypass Tool started                    │
│  10:23:41 [INFO]    Termux environment detected                │
│  10:23:55 [INFO]    Starting proxy on port 8080...             │
│  10:23:55 [OK]      Proxy running!                             │
│  10:24:01 [INFO]    Scanning hosts.txt with concurrency 50     │
│  10:24:03 [OK]      ✓ cdn.cloudflare.net - 43ms               │
│  10:24:03 [OK]      ✓ workers.dev - 67ms                       │
└───────────────────────────────────────────────────────────────┘
```

- Press `a` to toggle auto-scroll
- Scroll manually with `↑↓` when auto-scroll is off
- Last 500 log entries are kept

---

### Help Tab

Full keybinding reference built into the TUI. Press `5` or `?` to access.

---

## Key Bindings

### Global

| Key | Action |
|-----|--------|
| `1` | Dashboard tab |
| `2` | Scanner tab |
| `3` | Results tab |
| `4` | Logs tab |
| `5` | Help tab |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `?` | Toggle help popup |
| `q` / `Ctrl+C` | Quit |

### Editing

| Key | Action |
|-----|--------|
| `e` / `i` | Enter edit mode |
| `Esc` | Exit edit mode |
| `Tab` | Next input field |
| `Shift+Tab` | Previous input field |
| `Enter` | Confirm & next field |
| `Backspace` | Delete character |
| `n` / `p` | Next / prev field (normal mode) |

### Proxy (Dashboard)

| Key | Action |
|-----|--------|
| `s` | Start / Stop proxy |
| `Enter` | Start / Stop proxy |

### Scanner

| Key | Action |
|-----|--------|
| `s` / `S` | Start scan (or stop if already scanning) |
| `x` | Stop scan |

### Results

| Key | Action |
|-----|--------|
| `↑` / `k` | Select previous |
| `↓` / `j` | Select next |
| `PgUp` | Page up |
| `PgDn` | Page down |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `u` | Use selected host as SNI |

### Logs

| Key | Action |
|-----|--------|
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `a` | Toggle auto-scroll |
| `g` | Jump to top |
| `G` | Jump to bottom |

---

## SNI Scanner

The built-in scanner is a native Rust port of
[sni-scanner](https://github.com/seramo/sni-scanner).

It concurrently probes each host in your hosts file for:

| Check | Description |
|-------|-------------|
| **TLS** | Full TLS handshake on port 443 |
| **HTTP** | HTTP response check on port 80 |
| **Latency** | Round-trip time in milliseconds |

Results are sorted automatically — working hosts first, then by latency
(fastest at top). The best working hosts make ideal SNI candidates.

**Concurrency guide:**

| Device | Recommended concurrency |
|--------|------------------------|
| Desktop/Server | `100` – `500` |
| Raspberry Pi / VPS | `50` – `100` |
| Android (Termux) | `20` – `50` |
| Slow connection | `10` – `20` |

---

## Hosts File Format

Plain text, one host per line. Blank lines and `#` comments are ignored.

```text
# hosts.txt — SNI candidates to scan

# Cloudflare
cloudflare.com
cdn.cloudflare.net
workers.dev
pages.dev

# Google CDN
googleapis.com
gstatic.com
storage.googleapis.com
ajax.googleapis.com
fonts.googleapis.com

# AWS CloudFront
cloudfront.net

# Akamai
akamaiedge.net
akamaized.net

# Fastly
fastly.net
cdn.fastly.net
```

A sample `hosts.txt` is included in every release archive.

---

## How It Works

```
Your App / Browser
       │
       │  HTTP/HTTPS traffic
       ▼
┌──────────────────┐
│  Local Proxy     │  127.0.0.1:8080
│  (this tool)     │
└──────────────────┘
       │
       │  TCP connection to TARGET host
       │  TLS handshake with SNI = SPOOFED host
       ▼
┌──────────────────┐
│  DPI / Firewall  │  Sees SNI = allowed host ✓
└──────────────────┘
       │
       ▼
┌──────────────────┐
│  Target Server   │  Receives actual request
└──────────────────┘
```

1. Your app connects to the local proxy via HTTP CONNECT
2. The proxy opens a TCP connection to the real target
3. During TLS handshake, it presents the **spoofed SNI** instead of the target
4. DPI/firewall sees an allowed hostname → lets traffic through
5. The target server responds normally
6. All traffic is transparently forwarded

> This works because many firewalls filter on SNI but route based on IP.
> The actual TLS certificate validation still uses the real target host.

---

## Supported Platforms

| Platform | Architecture | Binary |
|----------|-------------|--------|
| 🐧 Linux | x86_64 | `sni-bypass-rs-tui-linux-x86_64.tar.gz` |
| 🐧 Linux | ARM64 | `sni-bypass-rs-tui-linux-aarch64.tar.gz` |
| 🐧 Linux | ARMv7 | `sni-bypass-rs-tui-linux-armv7.tar.gz` |
| 📱 Android (Termux) | ARM64 | `sni-bypass-rs-tui-android-aarch64-termux.tar.gz` |
| 📱 Android (Termux) | ARMv7 | `sni-bypass-rs-tui-android-armv7-termux.tar.gz` |
| 🪟 Windows | x86_64 | `sni-bypass-rs-tui-windows-x86_64.zip` |
| 🍎 macOS | x86_64 (Intel) | `sni-bypass-rs-tui-macos-x86_64.tar.gz` |
| 🍎 macOS | ARM64 (Apple Silicon) | `sni-bypass-rs-tui-macos-aarch64.tar.gz` |

---

## FAQ

**Q: My terminal shows garbled characters / broken boxes**

The TUI uses Unicode box-drawing characters. Make sure your terminal uses
a font that supports them (e.g. JetBrains Mono, Fira Code, Nerd Fonts).
On Termux, install a Nerd Font via the
[Termux Styling](https://f-droid.org/packages/com.termux.styling/) app.

---

**Q: How do I know which SNI host to use?**

Use the built-in scanner (tab `2`). Add common CDN hosts to `hosts.txt`,
run the scan, then pick the fastest working host from the Results tab and
press `u` to use it automatically.

---

**Q: The proxy starts but nothing loads**

- Check that your browser/app proxy is set to `127.0.0.1:<port>`
- Make sure the target host is reachable (ping test)
- Try a different SNI host from the scanner results
- Check the Logs tab (`4`) for connection errors

---

**Q: Does this work with UDP / QUIC / HTTP3?**

No. This tool handles TCP-based connections (HTTP/HTTPS). QUIC (UDP) is
not supported.

---

**Q: Termux build fails with OpenSSL error**

```bash
pkg install openssl-tool openssl
export OPENSSL_DIR=$PREFIX
cargo build --release
```

Or use the provided `build-termux.sh` which handles this automatically.

---

**Q: Can I use this as a system-wide proxy on Android?**

Yes. After starting the proxy on port `8080`, go to:
`WiFi Settings → Long press network → Modify → Advanced → Proxy → Manual`
Set host `127.0.0.1` and port `8080`.

---

**Q: Is this legal?**

This tool is for **educational and research purposes**. Whether bypassing
network filtering is legal depends on your country and the network you're
on. You are responsible for how you use this tool.

---

## Contributing

Contributions are welcome! Please open an issue first to discuss what you'd
like to change.

```bash
# Clone and setup
git clone https://github.com/mewshiam/sni-bypass-rs-tui
cd sni-bypass-rs-tui

# Run in dev mode
cargo run

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Lint
cargo clippy -- -D warnings
```

---

## License

MIT — see [LICENSE](LICENSE)

---

<div align="center">

Made with ❤️ and Rust

**[⬆ Back to top](#sni-bypass-rs-tui)**

</div>
