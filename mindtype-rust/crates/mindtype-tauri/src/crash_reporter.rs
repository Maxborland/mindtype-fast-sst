//! Crash reporter module for collecting and submitting crash reports.
//!
//! Features:
//! - Breadcrumb collection for tracking user actions
//! - Panic hook with backtrace capture
//! - Sensitive data sanitization (API keys, paths, license keys)
//! - Crash report submission to server

use chrono::{Local, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use std::collections::VecDeque;
use std::fs;
use std::panic::PanicHookInfo;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tracing::{error, info, warn};

/// Maximum number of breadcrumbs to keep
const MAX_BREADCRUMBS: usize = 50;

/// Support email for crash reports
const SUPPORT_EMAIL: &str = "help@mindtype.space";

/// Global breadcrumb storage
static BREADCRUMBS: Lazy<RwLock<VecDeque<Breadcrumb>>> =
    Lazy::new(|| RwLock::new(VecDeque::with_capacity(MAX_BREADCRUMBS)));

/// Original panic hook (to chain to)
static ORIGINAL_HOOK: Lazy<Mutex<Option<Box<dyn Fn(&PanicHookInfo<'_>) + Send + Sync + 'static>>>> =
    Lazy::new(|| Mutex::new(None));

/// A breadcrumb entry tracking user actions
#[derive(Debug, Clone, Serialize)]
pub struct Breadcrumb {
    pub timestamp: String,
    pub message: String,
}

impl Breadcrumb {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            message: message.into(),
        }
    }
}

/// Add a breadcrumb to track user actions
pub fn add_breadcrumb(message: impl Into<String>) {
    if let Ok(mut breadcrumbs) = BREADCRUMBS.write() {
        breadcrumbs.push_back(Breadcrumb::new(message));

        // Keep only the last MAX_BREADCRUMBS
        while breadcrumbs.len() > MAX_BREADCRUMBS {
            breadcrumbs.pop_front();
        }
    }
}

/// Get recent breadcrumbs
pub fn get_breadcrumbs() -> Vec<Breadcrumb> {
    BREADCRUMBS
        .read()
        .map(|b| b.iter().cloned().collect())
        .unwrap_or_default()
}

/// Clear all breadcrumbs
pub fn clear_breadcrumbs() {
    if let Ok(mut breadcrumbs) = BREADCRUMBS.write() {
        breadcrumbs.clear();
    }
}

/// System information for crash reports
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub app_version: String,
    pub platform: String,
    pub os_version: String,
    pub architecture: String,
    pub rust_version: String,
    pub is_release: bool,
}

impl SystemInfo {
    pub fn collect() -> Self {
        Self {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            platform: std::env::consts::OS.to_string(),
            os_version: os_info::get().to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            rust_version: option_env!("CARGO_PKG_RUST_VERSION")
                .unwrap_or("unknown")
                .to_string(),
            is_release: !cfg!(debug_assertions),
        }
    }
}

/// Crash report payload for server submission
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub app_version: String,
    pub platform: String,
    pub rust_version: String,
    pub os_version: String,
    pub error_type: String,
    pub error_message: String,
    pub backtrace: String,
    pub breadcrumbs: Vec<String>,
    pub device_id: Option<String>,
    pub timestamp: String,
}

/// Sanitize text by removing sensitive data
pub fn sanitize_text(text: &str) -> String {
    // Build all regex patterns
    static PATTERNS: Lazy<Vec<(Regex, &str)>> = Lazy::new(|| {
        vec![
            // OpenAI keys: sk-... or sk-proj-...
            (
                Regex::new(r"sk-[a-zA-Z0-9_-]{20,}").unwrap(),
                "sk-***REDACTED***",
            ),
            // Anthropic keys: sk-ant-...
            (
                Regex::new(r"sk-ant-[a-zA-Z0-9_-]{20,}").unwrap(),
                "sk-ant-***REDACTED***",
            ),
            // Google API keys: AIza...
            (
                Regex::new(r"AIza[a-zA-Z0-9_-]{30,}").unwrap(),
                "AIza***REDACTED***",
            ),
            // OpenRouter keys: sk-or-...
            (
                Regex::new(r"sk-or-[a-zA-Z0-9_-]{20,}").unwrap(),
                "sk-or-***REDACTED***",
            ),
            // Groq keys: gsk_...
            (
                Regex::new(r"gsk_[a-zA-Z0-9_-]{20,}").unwrap(),
                "gsk_***REDACTED***",
            ),
            // Long hex strings (Together AI, HMAC, etc.)
            (
                Regex::new(r"[a-f0-9]{64}").unwrap(),
                "***REDACTED_HEX64***",
            ),
            // Bearer tokens
            (
                Regex::new(r"[Bb]earer\s+[a-zA-Z0-9_.-]{20,}").unwrap(),
                "Bearer ***REDACTED***",
            ),
            // Authorization header values
            (
                Regex::new(r#"[Aa]uthorization[=:]\s*["']?[a-zA-Z0-9_.-]{20,}["']?"#).unwrap(),
                "Authorization=***REDACTED***",
            ),
            // Generic api_key, secret, token, password patterns
            (
                Regex::new(r#"(?i)(api[_-]?key|apikey|secret|token|password|credential)[=:]\s*["']?[a-zA-Z0-9_.-]{8,}["']?"#).unwrap(),
                "$1=***REDACTED***",
            ),
            // Environment variable secrets
            (
                Regex::new(r#"(OPENAI_API_KEY|ANTHROPIC_API_KEY|GOOGLE_API_KEY|OPENROUTER_API_KEY|API_KEY|SECRET_KEY|AUTH_TOKEN)[=:]\s*["']?[^\s"']{8,}["']?"#).unwrap(),
                "$1=***REDACTED***",
            ),
            // MindType license keys
            (
                Regex::new(r"MT[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}").unwrap(),
                "MT****-****-****-****",
            ),
            // Generic license key patterns (XXXX-XXXX-XXXX-XXXX)
            (
                Regex::new(r"\b[A-Z0-9]{4,5}-[A-Z0-9]{4,5}-[A-Z0-9]{4,5}-[A-Z0-9]{4,5}\b").unwrap(),
                "****-****-****-****",
            ),
            // HMAC/hash secrets
            (
                Regex::new(r"(?i)(hmac|secret|hash)[_-]?[a-fA-F0-9]{32,}").unwrap(),
                "$1_***REDACTED***",
            ),
            // Windows user paths
            (
                Regex::new(r"C:\\Users\\[^\\]+").unwrap(),
                r"C:\Users\<user>",
            ),
            (
                Regex::new(r"C:/Users/[^/]+").unwrap(),
                "C:/Users/<user>",
            ),
            // Unix user paths
            (Regex::new(r"/home/[^/]+").unwrap(), "/home/<user>"),
            (Regex::new(r"/Users/[^/]+").unwrap(), "/Users/<user>"),
            // Email addresses (partial masking)
            (
                Regex::new(r"([a-zA-Z0-9._%+-]{2})[a-zA-Z0-9._%+-]*@([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})").unwrap(),
                "$1***@$2",
            ),
        ]
    });

    let mut result = text.to_string();
    for (pattern, replacement) in PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).to_string();
    }
    result
}

/// Get the crashes directory
pub fn get_crashes_dir() -> PathBuf {
    let base = directories::ProjectDirs::from("com", "mindtype", "MindType")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            directories::BaseDirs::new()
                .map(|dirs| dirs.data_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        });

    let crashes_dir = base.join("crashes");
    let _ = fs::create_dir_all(&crashes_dir);
    crashes_dir
}

/// Generate a crash report from panic info
pub fn generate_crash_report(
    panic_info: &str,
    backtrace: &str,
    breadcrumbs: &[Breadcrumb],
) -> String {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let sys_info = SystemInfo::collect();

    let mut lines = vec![
        "=".repeat(70),
        "MINDTYPE CRASH REPORT".to_string(),
        format!("Time: {}", timestamp),
        "=".repeat(70),
        String::new(),
        "--- SYSTEM INFORMATION ---".to_string(),
        format!("app_version: {}", sys_info.app_version),
        format!("platform: {}", sys_info.platform),
        format!("os_version: {}", sys_info.os_version),
        format!("architecture: {}", sys_info.architecture),
        format!("rust_version: {}", sys_info.rust_version),
        format!("is_release: {}", sys_info.is_release),
        String::new(),
        "--- PANIC INFO ---".to_string(),
        panic_info.to_string(),
        String::new(),
        "--- BACKTRACE ---".to_string(),
        backtrace.to_string(),
        String::new(),
    ];

    if !breadcrumbs.is_empty() {
        lines.push("--- RECENT ACTIONS ---".to_string());
        for crumb in breadcrumbs.iter().rev().take(20) {
            lines.push(format!("[{}] {}", crumb.timestamp, crumb.message));
        }
        lines.push(String::new());
    }

    lines.push("=".repeat(70));
    lines.push("To submit this report:".to_string());
    lines.push(format!("1. Send this file to {}", SUPPORT_EMAIL));
    lines.push("2. Describe what you were doing before the error".to_string());
    lines.push("=".repeat(70));

    let report = lines.join("\n");
    sanitize_text(&report)
}

/// Save crash report to file
pub fn save_crash_report(report: &str) -> Option<PathBuf> {
    let crashes_dir = get_crashes_dir();
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("crash_{}.txt", timestamp);
    let filepath = crashes_dir.join(filename);

    match fs::write(&filepath, report) {
        Ok(_) => {
            info!("Crash report saved to: {:?}", filepath);
            Some(filepath)
        }
        Err(e) => {
            error!("Failed to save crash report: {}", e);
            None
        }
    }
}

/// Get the crash report API URL
pub fn get_crash_report_url() -> String {
    std::env::var("MINDTYPE_API_URL")
        .unwrap_or_else(|_| "https://mindtype.space".to_string())
        + "/api/crash-report"
}

/// Get device ID for crash reports
fn get_device_id() -> Option<String> {
    mindtype_platform::Platform::new()
        .ok()
        .and_then(|p| p.get_device_id().ok())
}

/// Send crash report to server
pub async fn send_crash_report_to_server(report: CrashReport) -> Result<(), String> {
    let url = get_crash_report_url();

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            format!("MindType/{}", env!("CARGO_PKG_VERSION")),
        )
        .json(&report)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status().is_success() {
        info!("Crash report sent successfully");
        Ok(())
    } else {
        let status = response.status();
        Err(format!("Server error: {}", status))
    }
}

/// Create a CrashReport from panic info
pub fn create_crash_report(panic_info: &str, backtrace: &str) -> CrashReport {
    let sys_info = SystemInfo::collect();
    let breadcrumbs = get_breadcrumbs();

    // Get device ID from platform crate if available
    let device_id = get_device_id();

    CrashReport {
        app_version: sys_info.app_version,
        platform: sys_info.platform,
        rust_version: sys_info.rust_version,
        os_version: sys_info.os_version,
        error_type: "Panic".to_string(),
        error_message: sanitize_text(panic_info).chars().take(500).collect(),
        backtrace: sanitize_text(backtrace).chars().take(50000).collect(),
        breadcrumbs: breadcrumbs
            .iter()
            .rev()
            .take(20)
            .map(|b| sanitize_text(&format!("[{}] {}", b.timestamp, b.message)))
            .collect(),
        device_id,
        timestamp: Utc::now().to_rfc3339(),
    }
}

/// Install the crash handler panic hook
pub fn install_crash_handler() {
    // Store the original hook
    let original = std::panic::take_hook();
    *ORIGINAL_HOOK.lock().unwrap() = Some(original);

    std::panic::set_hook(Box::new(|panic_info| {
        // Capture backtrace
        let backtrace = Backtrace::force_capture();
        let backtrace_str = format!("{}", backtrace);

        // Format panic info
        let panic_message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let panic_info_str = format!("{} at {}", panic_message, location);

        // Log the panic
        error!("PANIC: {}", panic_info_str);
        error!("Backtrace:\n{}", backtrace_str);

        // Get breadcrumbs
        let breadcrumbs = get_breadcrumbs();

        // Generate and save report
        let report = generate_crash_report(&panic_info_str, &backtrace_str, &breadcrumbs);
        if let Some(path) = save_crash_report(&report) {
            eprintln!("\n{}", "=".repeat(50));
            eprintln!("CRITICAL ERROR - Crash report saved to:");
            eprintln!("{}", path.display());
            eprintln!("{}\n", "=".repeat(50));
        }

        // Try to send report to server (best effort, don't block)
        let crash_report = create_crash_report(&panic_info_str, &backtrace_str);

        // Spawn a thread to send the report since we can't use async in panic hook
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();

            if let Ok(rt) = rt {
                let _ = rt.block_on(async {
                    if let Err(e) = send_crash_report_to_server(crash_report).await {
                        warn!("Failed to send crash report: {}", e);
                    }
                });
            }
        });

        // Give the sender thread a moment to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Call original hook if any
        if let Ok(guard) = ORIGINAL_HOOK.lock() {
            if let Some(ref original) = *guard {
                original(panic_info);
            }
        }
    }));

    info!("Crash handler installed");
}

/// Uninstall the crash handler and restore original hook
pub fn uninstall_crash_handler() {
    if let Ok(mut guard) = ORIGINAL_HOOK.lock() {
        if let Some(original) = guard.take() {
            std::panic::set_hook(original);
            info!("Crash handler uninstalled");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_openai_key() {
        let text = "api_key=sk-1234567890abcdefghijklmnop";
        let sanitized = sanitize_text(text);
        assert!(!sanitized.contains("1234567890"));
        assert!(sanitized.contains("REDACTED"));
    }

    #[test]
    fn test_sanitize_anthropic_key() {
        let text = "key: sk-ant-abc123xyz789defghijklmnop";
        let sanitized = sanitize_text(text);
        assert!(!sanitized.contains("abc123xyz789"));
        assert!(sanitized.contains("REDACTED"));
    }

    #[test]
    fn test_sanitize_windows_path() {
        let text = r"Error at C:\Users\JohnDoe\Documents\file.txt";
        let sanitized = sanitize_text(text);
        assert!(!sanitized.contains("JohnDoe"));
        assert!(sanitized.contains("<user>"));
    }

    #[test]
    fn test_sanitize_unix_path() {
        let text = "Error at /home/johndoe/projects/file.rs";
        let sanitized = sanitize_text(text);
        assert!(!sanitized.contains("johndoe"));
        assert!(sanitized.contains("<user>"));
    }

    #[test]
    fn test_sanitize_license_key() {
        let text = "License: MTAB12-CD34-EF56-GH78";
        let sanitized = sanitize_text(text);
        assert!(!sanitized.contains("AB12"));
        assert!(sanitized.contains("****"));
    }

    #[test]
    fn test_breadcrumbs() {
        // Note: These tests share global state, so we use a unique prefix
        clear_breadcrumbs();
        add_breadcrumb("TestBreadcrumbs_Action 1");
        add_breadcrumb("TestBreadcrumbs_Action 2");
        let crumbs = get_breadcrumbs();
        // Check that our breadcrumbs were added (may have others from parallel tests)
        assert!(crumbs.iter().any(|c| c.message == "TestBreadcrumbs_Action 1"));
        assert!(crumbs.iter().any(|c| c.message == "TestBreadcrumbs_Action 2"));
    }

    #[test]
    fn test_breadcrumb_limit() {
        clear_breadcrumbs();
        for i in 0..100 {
            add_breadcrumb(format!("TestLimit_Action {}", i));
        }
        let crumbs = get_breadcrumbs();
        // Should not exceed MAX_BREADCRUMBS
        assert!(crumbs.len() <= MAX_BREADCRUMBS);
    }
}
