use std::path::Path;

/// Detect if running inside Termux (Android)
pub fn detect_termux() -> bool {
    // Check Termux-specific environment variables
    if std::env::var("TERMUX_VERSION").is_ok() {
        return true;
    }
    if std::env::var("PREFIX").map(|p| p.contains("com.termux")).unwrap_or(false) {
        return true;
    }
    // Check Termux prefix path
    if Path::new("/data/data/com.termux").exists() {
        return true;
    }
    if Path::new("/data/user/0/com.termux").exists() {
        return true;
    }
    false
}

/// Get Termux-specific install instructions
pub fn install_instructions() -> Vec<&'static str> {
    vec![
        "pkg update && pkg upgrade",
        "pkg install rust",
        "pkg install openssl",
        "pkg install pkg-config",
        "cargo build --release",
        "./target/release/sni-bypass-rs-tui",
    ]
}

/// Check if required Termux packages are available
pub fn check_termux_deps() -> Vec<(String, bool)> {
    let deps = vec![
        ("openssl", Path::new("/data/data/com.termux/files/usr/lib/libssl.so").exists()),
        ("pkg-config", which_exists("pkg-config")),
    ];
    deps.into_iter()
        .map(|(name, ok)| (name.to_string(), ok))
        .collect()
}

fn which_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Termux-optimized terminal size (smaller screens)
pub fn get_min_terminal_size() -> (u16, u16) {
    if detect_termux() {
        (70, 20) // Minimum for Termux
    } else {
        (80, 24) // Standard terminal
    }
}