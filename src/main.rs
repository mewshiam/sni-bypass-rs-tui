mod tui;
mod scanner;
mod bypass;
mod utils;

use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tui::App;
use utils::termux;

// ─────────────────────────────────────────────
// Config structs
// ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub version: Option<String>,
    pub proxy: ProxyConfig,
    pub scanner: ScannerConfig,
    pub tui: TuiConfig,
    pub log: LogConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub target_host: String,
    pub sni_host: String,
    pub timeout_secs: u64,
    pub buffer_size: usize,
    pub max_connections: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScannerConfig {
    pub hosts_file: String,
    pub concurrency: usize,
    pub timeout_secs: u64,
    pub ports: Vec<u16>,
    pub check_tls: bool,
    pub check_http: bool,
    pub save_results: bool,
    pub results_file: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TuiConfig {
    pub auto_scroll_logs: bool,
    pub max_log_entries: usize,
    pub refresh_ms: u64,
    pub show_help_on_start: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogConfig {
    pub file: String,
    pub level: String,
}

// ─────────────────────────────────────────────
// Config defaults + load/save
// ─────────────────────────────────────────────

impl Default for Config {
    fn default() -> Self {
        Self {
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            proxy: ProxyConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                target_host: String::new(),
                sni_host: String::new(),
                timeout_secs: 30,
                buffer_size: 8192,
                max_connections: 100,
            },
            scanner: ScannerConfig {
                hosts_file: "hosts.txt".to_string(),
                concurrency: 50,
                timeout_secs: 5,
                ports: vec![443, 80, 8080],
                check_tls: true,
                check_http: true,
                save_results: true,
                results_file: "scan_results.json".to_string(),
            },
            tui: TuiConfig {
                auto_scroll_logs: true,
                max_log_entries: 500,
                refresh_ms: 50,
                show_help_on_start: false,
            },
            log: LogConfig {
                file: "sni-bypass.log".to_string(),
                level: "info".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = "config.json";

        if Path::new(path).exists() {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    match serde_json::from_str::<Config>(&content) {
                        Ok(cfg) => {
                            return cfg;
                        }
                        Err(e) => {
                            eprintln!(
                                "[!] Invalid config.json: {} — using defaults",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[!] Cannot read config.json: {} — using defaults",
                        e
                    );
                }
            }
        }

        // Write default config if missing or invalid
        let default = Config::default();
        match serde_json::to_string_pretty(&default) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    eprintln!("[!] Could not write default config.json: {}", e);
                }
            }
            Err(e) => {
                eprintln!("[!] Could not serialize default config: {}", e);
            }
        }

        default
    }

    pub fn save(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write("config.json", json) {
                    eprintln!("[!] Could not save config.json: {}", e);
                }
            }
            Err(e) => {
                eprintln!("[!] Could not serialize config: {}", e);
            }
        }
    }
}

// ─────────────────────────────────────────────
// CLI args
// ─────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "sni-bypass-rs-tui",
    version = env!("CARGO_PKG_VERSION"),
    about = "SNI Bypass Tool with TUI — Termux Compatible",
    long_about = None
)]
struct Args {
    /// Run in headless/CLI mode (no TUI)
    #[arg(long, short = 'H')]
    headless: bool,

    /// Target host to bypass
    #[arg(long, short = 't')]
    target: Option<String>,

    /// SNI host to use (defaults to target if not set)
    #[arg(long, short = 's')]
    sni: Option<String>,

    /// Local proxy port
    #[arg(long, short = 'p')]
    port: Option<u16>,

    /// Run SNI scanner only then exit
    #[arg(long)]
    scan_only: bool,

    /// Hosts file for scanning
    #[arg(long, short = 'f')]
    hosts_file: Option<String>,

    /// Concurrency for scanner
    #[arg(long, short = 'c')]
    concurrency: Option<usize>,

    /// Config file path (default: config.json)
    #[arg(long, default_value = "config.json")]
    config: String,

    /// Print current config and exit
    #[arg(long)]
    print_config: bool,
}

// ─────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Load config
    let mut config = Config::load();

    // Print config and exit if requested
    if args.print_config {
        println!("{}", serde_json::to_string_pretty(&config)?);
        return Ok(());
    }

    // CLI args override config values
    apply_args_to_config(&args, &mut config);

    // Setup file logger (keep terminal clean for TUI)
    setup_logger(&config.log.file, &config.log.level);

    // Detect Termux environment
    let is_termux = termux::detect_termux();

    tracing::info!(
        "SNI Bypass RS-TUI v{} starting",
        env!("CARGO_PKG_VERSION")
    );
    tracing::info!(
        "Termux: {} | Mode: {}",
        is_termux,
        if args.headless || args.scan_only { "headless" } else { "tui" }
    );

    // Route to correct mode
    if args.scan_only {
        run_scan_only(&config).await
    } else if args.headless {
        run_headless(&config).await
    } else {
        run_tui(config, is_termux).await
    }
}

// ─────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────

/// Apply CLI argument overrides on top of loaded config
fn apply_args_to_config(args: &Args, config: &mut Config) {
    if let Some(target) = &args.target {
        config.proxy.target_host = target.clone();
    }
    if let Some(sni) = &args.sni {
        config.proxy.sni_host = sni.clone();
    }
    if let Some(port) = args.port {
        config.proxy.port = port;
    }
    if let Some(hosts_file) = &args.hosts_file {
        config.scanner.hosts_file = hosts_file.clone();
    }
    if let Some(concurrency) = args.concurrency {
        config.scanner.concurrency = concurrency;
    }

    // If sni not set, default to target
    if config.proxy.sni_host.is_empty() && !config.proxy.target_host.is_empty() {
        config.proxy.sni_host = config.proxy.target_host.clone();
    }
}

/// Setup tracing logger writing to a file
fn setup_logger(log_file: &str, level: &str) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_new(level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    match std::fs::File::create(log_file) {
        Ok(file) => {
            tracing_subscriber::fmt()
                .with_writer(file)
                .with_env_filter(filter)
                .with_ansi(false)
                .init();
        }
        Err(e) => {
            eprintln!("[!] Could not create log file '{}': {}", log_file, e);
            // Fallback: log to stderr
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_ansi(false)
                .init();
        }
    }
}

// ─────────────────────────────────────────────
// Run modes
// ─────────────────────────────────────────────

/// Full TUI mode
// ── Fix E0609: was config.port, must be config.proxy.port ──
async fn run_tui(config: Config, is_termux: bool) -> Result<()> {
    let mut app = App::new(&config, is_termux)?;

    if !config.proxy.target_host.is_empty() {
        app.set_target(config.proxy.target_host.clone());
    }
    if !config.proxy.sni_host.is_empty() {
        app.set_sni(config.proxy.sni_host.clone());
    }

    app.run().await
}

/// Headless proxy mode (no TUI)
async fn run_headless(config: &Config) -> Result<()> {
    use bypass::ProxyServer;

    let target = if config.proxy.target_host.is_empty() {
        eprintln!("[!] Error: --target is required in headless mode");
        eprintln!("    Example: sni-bypass-rs-tui --headless -t example.com");
        std::process::exit(1);
    } else {
        config.proxy.target_host.clone()
    };

    let sni = if config.proxy.sni_host.is_empty() {
        target.clone()
    } else {
        config.proxy.sni_host.clone()
    };

    let port = config.proxy.port;

    println!(
        "╔══════════════════════════════════════╗"
    );
    println!(
        "║      SNI Bypass RS-TUI v{}           ║",
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "╚══════════════════════════════════════╝"
    );
    println!();
    println!("[*] Mode    : Headless");
    println!("[*] Target  : {}", target);
    println!("[*] SNI     : {}", sni);
    println!("[*] Port    : {}", port);
    println!("[*] Log     : {}", config.log.file);
    println!();
    println!("[*] Starting proxy... (Ctrl+C to stop)");
    println!("[*] Set your proxy to 127.0.0.1:{}", port);

    let server = ProxyServer::new(port, target, sni);
    server.run().await
}

/// Scanner only mode — scan then print results and exit
async fn run_scan_only(config: &Config) -> Result<()> {
    use scanner::SniScanner;

    let hosts_file = &config.scanner.hosts_file;
    let concurrency = config.scanner.concurrency;
    let save = config.scanner.save_results;
    let results_file = &config.scanner.results_file;

    println!(
        "╔══════════════════════════════════════╗"
    );
    println!(
        "║      SNI Bypass RS-TUI v{}           ║",
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "╚══════════════════════════════════════╝"
    );
    println!();
    println!("[*] Mode        : Scanner only");
    println!("[*] Hosts file  : {}", hosts_file);
    println!("[*] Concurrency : {}", concurrency);
    println!("[*] Timeout     : {}s", config.scanner.timeout_secs);
    println!();

    // Validate hosts file exists
    if !Path::new(hosts_file).exists() {
        eprintln!("[!] Hosts file not found: {}", hosts_file);
        eprintln!("    Create it or use --hosts-file to specify a path");
        std::process::exit(1);
    }

    println!("[*] Starting scan...");
    println!();

    let scanner = SniScanner::new(concurrency)
        .with_timeout(config.scanner.timeout_secs);

    // Live progress callback
     let results = scanner.scan_from_file(&hosts_file).await?;

    // Summary
    let working: Vec<_> = results.iter().filter(|r| r.is_working).collect();
    let total = results.len();

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Scan complete");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Total scanned : {}", total);
    println!(" Working       : {}", working.len());
    println!(" Failed        : {}", total - working.len());

    if let Some(best) = working.first() {
        println!(" Best SNI      : {} ({}ms)", best.host, best.latency_ms);
    }

    println!();

    if !working.is_empty() {
        println!("[+] Working hosts (sorted by latency):");
        for r in &working {
            println!(
                "    {:<40} {}ms",
                r.host, r.latency_ms
            );
        }
        println!();
    }

    // Save results to JSON if enabled
    if save {
        match serde_json::to_string_pretty(&results) {
            Ok(json) => {
                match std::fs::write(results_file, json) {
                    Ok(_) => println!("[*] Results saved to {}", results_file),
                    Err(e) => eprintln!("[!] Could not save results: {}", e),
                }
            }
            Err(e) => eprintln!("[!] Could not serialize results: {}", e),
        }
    }

    Ok(())
}
