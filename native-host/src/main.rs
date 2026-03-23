use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, BufRead, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

/// A single command configuration
#[derive(Deserialize, Serialize, Debug, Clone)]
struct CommandConfig {
    /// Display name for the command
    name: String,
    /// The shell command to run
    command: String,
}

/// Configuration for URShell
#[derive(Deserialize, Serialize, Debug)]
struct Config {
    /// List of available commands
    commands: Vec<CommandConfig>,
}

/// Request from the Chrome extension
#[derive(Deserialize, Debug)]
struct Request {
    action: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    command_index: Option<usize>,
    #[serde(default)]
    commands: Option<Vec<CommandConfig>>,
}

/// Response to the Chrome extension
#[derive(Serialize)]
struct Response {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commands: Option<Vec<CommandConfig>>,
}

impl Response {
    fn commands(cmds: Vec<CommandConfig>) -> Self {
        Self {
            status: "commands".to_string(),
            message: None,
            output: None,
            data: None,
            commands: Some(cmds),
        }
    }

    fn started() -> Self {
        Self {
            status: "started".to_string(),
            message: None,
            output: None,
            data: None,
            commands: None,
        }
    }

    fn output(data: &str) -> Self {
        Self {
            status: "output".to_string(),
            message: None,
            output: None,
            data: Some(data.to_string()),
            commands: None,
        }
    }

    fn complete(output: String) -> Self {
        Self {
            status: "complete".to_string(),
            message: None,
            output: Some(output),
            data: None,
            commands: None,
        }
    }

    fn error(message: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: Some(message.to_string()),
            output: None,
            data: None,
            commands: None,
        }
    }

    fn error_with_output(message: &str, output: String) -> Self {
        Self {
            status: "error".to_string(),
            message: Some(message.to_string()),
            output: Some(output),
            data: None,
            commands: None,
        }
    }

    fn saved() -> Self {
        Self {
            status: "saved".to_string(),
            message: None,
            output: None,
            data: None,
            commands: None,
        }
    }

    fn cancelled() -> Self {
        Self {
            status: "cancelled".to_string(),
            message: None,
            output: None,
            data: None,
            commands: None,
        }
    }
}

// ============================================================================
// Browser Detection and Installation
// ============================================================================

/// Information about a browser installation
#[derive(Debug, Clone)]
struct BrowserInfo {
    name: &'static str,
    display_name: &'static str,
    #[cfg(not(windows))]
    data_dir: PathBuf,
    #[cfg(not(windows))]
    native_messaging_dir: PathBuf,
    #[cfg(windows)]
    data_dir: PathBuf,
    #[cfg(windows)]
    registry_key: &'static str,
}

/// Result of scanning a browser for the extension
#[derive(Debug)]
struct ExtensionFound {
    browser: BrowserInfo,
    extension_id: String,
    profiles: Vec<String>,
}

/// Get list of supported browsers with their paths
fn get_browsers() -> Vec<BrowserInfo> {
    let home = dirs::home_dir().expect("Could not find home directory");
    let mut browsers = Vec::new();

    #[cfg(target_os = "macos")]
    {
        let app_support = home.join("Library/Application Support");

        browsers.push(BrowserInfo {
            name: "chrome",
            display_name: "Google Chrome",
            data_dir: app_support.join("Google/Chrome"),
            native_messaging_dir: app_support.join("Google/Chrome/NativeMessagingHosts"),
        });

        browsers.push(BrowserInfo {
            name: "brave",
            display_name: "Brave Browser",
            data_dir: app_support.join("BraveSoftware/Brave-Browser"),
            native_messaging_dir: app_support.join("BraveSoftware/Brave-Browser/NativeMessagingHosts"),
        });

        browsers.push(BrowserInfo {
            name: "chromium",
            display_name: "Chromium",
            data_dir: app_support.join("Chromium"),
            native_messaging_dir: app_support.join("Chromium/NativeMessagingHosts"),
        });

        browsers.push(BrowserInfo {
            name: "edge",
            display_name: "Microsoft Edge",
            data_dir: app_support.join("Microsoft Edge"),
            native_messaging_dir: app_support.join("Microsoft Edge/NativeMessagingHosts"),
        });
    }

    #[cfg(target_os = "linux")]
    {
        let config = home.join(".config");

        browsers.push(BrowserInfo {
            name: "chrome",
            display_name: "Google Chrome",
            data_dir: config.join("google-chrome"),
            native_messaging_dir: config.join("google-chrome/NativeMessagingHosts"),
        });

        browsers.push(BrowserInfo {
            name: "brave",
            display_name: "Brave Browser",
            data_dir: config.join("BraveSoftware/Brave-Browser"),
            native_messaging_dir: config.join("BraveSoftware/Brave-Browser/NativeMessagingHosts"),
        });

        browsers.push(BrowserInfo {
            name: "chromium",
            display_name: "Chromium",
            data_dir: config.join("chromium"),
            native_messaging_dir: config.join("chromium/NativeMessagingHosts"),
        });

        browsers.push(BrowserInfo {
            name: "edge",
            display_name: "Microsoft Edge",
            data_dir: config.join("microsoft-edge"),
            native_messaging_dir: config.join("microsoft-edge/NativeMessagingHosts"),
        });
    }

    #[cfg(windows)]
    {
        let local_app_data = dirs::data_local_dir().expect("Could not find local app data");

        browsers.push(BrowserInfo {
            name: "chrome",
            display_name: "Google Chrome",
            data_dir: local_app_data.join("Google\\Chrome\\User Data"),
            registry_key: "Software\\Google\\Chrome\\NativeMessagingHosts",
        });

        browsers.push(BrowserInfo {
            name: "brave",
            display_name: "Brave Browser",
            data_dir: local_app_data.join("BraveSoftware\\Brave-Browser\\User Data"),
            registry_key: "Software\\BraveSoftware\\Brave-Browser\\NativeMessagingHosts",
        });

        browsers.push(BrowserInfo {
            name: "chromium",
            display_name: "Chromium",
            data_dir: local_app_data.join("Chromium\\User Data"),
            registry_key: "Software\\Chromium\\NativeMessagingHosts",
        });

        browsers.push(BrowserInfo {
            name: "edge",
            display_name: "Microsoft Edge",
            data_dir: local_app_data.join("Microsoft\\Edge\\User Data"),
            registry_key: "Software\\Microsoft\\Edge\\NativeMessagingHosts",
        });
    }

    browsers
}

/// Get list of profile directories for a browser
fn get_profile_dirs(browser: &BrowserInfo) -> Vec<(String, PathBuf)> {
    let mut profiles = Vec::new();

    // Check both Preferences and Secure Preferences (Chrome/Brave use Secure Preferences)
    let pref_files = ["Secure Preferences", "Preferences"];

    // Default profile
    for pref_file in &pref_files {
        let default_prefs = browser.data_dir.join("Default").join(pref_file);
        if default_prefs.exists() {
            profiles.push(("Default".to_string(), default_prefs));
            break;
        }
    }

    // Numbered profiles (Profile 1, Profile 2, etc.)
    for i in 1..=20 {
        let profile_name = format!("Profile {}", i);
        for pref_file in &pref_files {
            let prefs = browser.data_dir.join(&profile_name).join(pref_file);
            if prefs.exists() {
                profiles.push((profile_name.clone(), prefs));
                break;
            }
        }
    }

    profiles
}

/// Search for URShell extension in a Preferences file
fn find_extension_in_preferences(prefs_path: &PathBuf) -> Option<String> {
    let content = std::fs::read_to_string(prefs_path).ok()?;
    let prefs: serde_json::Value = serde_json::from_str(&content).ok()?;

    // Navigate to extensions.settings
    let settings = prefs.get("extensions")?.get("settings")?.as_object()?;

    for (ext_id, ext_data) in settings {
        // Check if this is an unpacked extension (location == 4)
        let location = ext_data.get("location").and_then(|v| v.as_i64());
        if location != Some(4) {
            continue;
        }

        // First try: check the manifest name directly in preferences
        if let Some(manifest) = ext_data.get("manifest") {
            if let Some(name) = manifest.get("name").and_then(|v| v.as_str()) {
                if name == "URShell" {
                    return Some(ext_id.clone());
                }
            }
        }

        // Second try: for unpacked extensions, read manifest.json from the path
        if let Some(path) = ext_data.get("path").and_then(|v| v.as_str()) {
            let manifest_path = std::path::Path::new(path).join("manifest.json");
            if let Ok(manifest_content) = std::fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_content) {
                    if let Some(name) = manifest.get("name").and_then(|v| v.as_str()) {
                        if name == "URShell" {
                            return Some(ext_id.clone());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Scan all browsers for the URShell extension
fn scan_for_extension() -> Vec<ExtensionFound> {
    let mut results = Vec::new();
    let debug = std::env::var("URSHELL_DEBUG").is_ok();

    for browser in get_browsers() {
        if !browser.data_dir.exists() {
            if debug {
                eprintln!("DEBUG: {} data_dir does not exist: {:?}", browser.display_name, browser.data_dir);
            }
            continue;
        }

        let profiles = get_profile_dirs(&browser);
        if debug {
            eprintln!("DEBUG: {} found {} profiles", browser.display_name, profiles.len());
            for (name, path) in &profiles {
                eprintln!("DEBUG:   {} -> {:?}", name, path);
            }
        }

        let mut found_profiles = Vec::new();
        let mut extension_id = None;

        for (profile_name, prefs_path) in profiles {
            if debug {
                eprintln!("DEBUG: Checking {} / {}", browser.display_name, profile_name);
            }
            if let Some(id) = find_extension_in_preferences(&prefs_path) {
                if debug {
                    eprintln!("DEBUG:   FOUND extension: {}", id);
                }
                found_profiles.push(profile_name);
                extension_id = Some(id);
            } else if debug {
                eprintln!("DEBUG:   not found in this profile");
            }
        }

        if let Some(id) = extension_id {
            results.push(ExtensionFound {
                browser,
                extension_id: id,
                profiles: found_profiles,
            });
        }
    }

    results
}

/// Install native messaging manifest for a browser
#[cfg(not(windows))]
fn install_manifest(browser: &BrowserInfo, extension_id: &str, host_binary: &str) -> Result<(), String> {
    // Create directory if needed
    std::fs::create_dir_all(&browser.native_messaging_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let manifest_path = browser.native_messaging_dir.join("com.urshell.host.json");

    let manifest = serde_json::json!({
        "name": "com.urshell.host",
        "description": "Native messaging host for URShell extension",
        "path": host_binary,
        "type": "stdio",
        "allowed_origins": [
            format!("chrome-extension://{}/", extension_id)
        ]
    });

    let manifest_str = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    std::fs::write(&manifest_path, manifest_str)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(())
}

#[cfg(windows)]
fn install_manifest(browser: &BrowserInfo, extension_id: &str, host_binary: &str) -> Result<(), String> {
    use std::process::Command;

    // On Windows, we need to:
    // 1. Write the manifest JSON file somewhere
    // 2. Add a registry key pointing to it

    let manifest_dir = get_config_path()
        .ok_or("Could not determine config directory")?
        .parent()
        .ok_or("Invalid config path")?
        .to_path_buf();

    std::fs::create_dir_all(&manifest_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let manifest_path = manifest_dir.join("com.urshell.host.json");

    let manifest = serde_json::json!({
        "name": "com.urshell.host",
        "description": "Native messaging host for URShell extension",
        "path": host_binary,
        "type": "stdio",
        "allowed_origins": [
            format!("chrome-extension://{}/", extension_id)
        ]
    });

    let manifest_str = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    std::fs::write(&manifest_path, &manifest_str)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    // Add registry key
    let reg_key = format!("{}\\com.urshell.host", browser.registry_key);
    let manifest_path_str = manifest_path.to_string_lossy();

    let output = Command::new("reg")
        .args(["add", &format!("HKCU\\{}", reg_key), "/ve", "/t", "REG_SZ", "/d", &manifest_path_str, "/f"])
        .output()
        .map_err(|e| format!("Failed to run reg command: {}", e))?;

    if !output.status.success() {
        return Err(format!("Failed to add registry key: {}", String::from_utf8_lossy(&output.stderr)));
    }

    Ok(())
}

/// Create default config file if it doesn't exist
fn create_default_config() -> Result<PathBuf, String> {
    let config_path = get_config_path().ok_or("Could not determine config directory")?;

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    if !config_path.exists() {
        let default_config = Config {
            commands: vec![],
        };

        let json = serde_json::to_string_pretty(&default_config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(&config_path, json)
            .map_err(|e| format!("Failed to write config: {}", e))?;
    }

    Ok(config_path)
}

/// Run the install process
fn run_install() {
    // ANSI colors
    const GREEN: &str = "\x1b[32m";
    const RED: &str = "\x1b[31m";
    const YELLOW: &str = "\x1b[33m";
    const CYAN: &str = "\x1b[36m";
    const RESET: &str = "\x1b[0m";
    const CHECK: &str = "✓";
    const CROSS: &str = "✗";

    println!();
    println!("{}Scanning for browsers...{}", YELLOW, RESET);
    println!();

    // Get path to this binary
    let host_binary = env::current_exe()
        .expect("Could not determine binary path")
        .to_string_lossy()
        .to_string();

    // Scan for extension
    let found = scan_for_extension();
    let browsers = get_browsers();

    // Report findings
    let mut installed_count = 0;
    let mut found_browsers: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for result in &found {
        found_browsers.insert(result.browser.name);
        let profile_list = result.profiles.join(", ");
        println!(
            "{}{}{} {}: Found URShell in {} (id: {}...)",
            GREEN, CHECK, RESET,
            result.browser.display_name,
            profile_list,
            &result.extension_id[..8.min(result.extension_id.len())]
        );
    }

    // Report browsers without extension
    for browser in &browsers {
        if !found_browsers.contains(browser.name) {
            if browser.data_dir.exists() {
                println!("{}{}{} {}: Extension not found", RED, CROSS, RESET, browser.display_name);
            } else {
                println!("{}{}{} {}: Not installed", RED, CROSS, RESET, browser.display_name);
            }
        }
    }

    if found.is_empty() {
        println!();
        println!("{}Error:{} URShell extension not found in any browser.", RED, RESET);
        println!();
        println!("Please install the extension first:");
        println!("  1. Open {}chrome://extensions{} (or {}brave://extensions{})", CYAN, RESET, CYAN, RESET);
        println!("  2. Enable \"{}Developer mode{}\"", YELLOW, RESET);
        println!("  3. Click \"{}Load unpacked{}\"", YELLOW, RESET);

        // Try to find extension directory relative to binary
        if let Ok(exe_path) = env::current_exe() {
            // Try multiple possible locations
            let possible_paths = [
                // When running from native-host/target/release/urshell-host (4 levels up)
                exe_path.parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                    .map(|p| p.join("extension")),
                // When running from native-host/urshell-host (pre-built, 2 levels up)
                exe_path.parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join("extension")),
            ];

            for ext_path in possible_paths.into_iter().flatten() {
                if ext_path.exists() {
                    println!("  4. Select: {}{}{}", CYAN, ext_path.display(), RESET);
                    break;
                }
            }
        }

        println!("  5. Run this installer again");
        println!();
        std::process::exit(1);
    }

    // Install manifests
    println!();
    println!("{}Installing native messaging host...{}", YELLOW, RESET);
    println!();

    for result in &found {
        match install_manifest(&result.browser, &result.extension_id, &host_binary) {
            Ok(()) => {
                println!(
                    "{}{}{} {}: Installed manifest",
                    GREEN, CHECK, RESET,
                    result.browser.display_name
                );
                installed_count += 1;
            }
            Err(e) => {
                println!(
                    "{}{}{} {}: Failed - {}",
                    RED, CROSS, RESET,
                    result.browser.display_name,
                    e
                );
            }
        }
    }

    // Create config
    match create_default_config() {
        Ok(path) => {
            println!("{}{}{} Config: {}", GREEN, CHECK, RESET, path.display());
        }
        Err(e) => {
            println!("{}{}{}  Config: Failed - {}", RED, CROSS, RESET, e);
        }
    }

    println!();

    if installed_count > 0 {
        println!("{}========================================{}", GREEN, RESET);
        println!("{}Installation complete!{}", GREEN, RESET);
        println!("{}========================================{}", GREEN, RESET);
        println!();
        println!("{}Configuration:{}", YELLOW, RESET);
        if let Some(config_path) = get_config_path() {
            println!("  {}{}{}", CYAN, config_path.display(), RESET);
        }
        println!();
        println!("{}Example config (single command, auto-executes):{}", YELLOW, RESET);
        println!("  {{");
        println!("    \"commands\": [");
        println!("      {{\"name\": \"Open in Firefox\", \"command\": \"open -a Firefox\"}}");
        println!("    ]");
        println!("  }}");
        println!();
        println!("{}Example config (multiple commands, shows picker):{}", YELLOW, RESET);
        println!("  {{");
        println!("    \"commands\": [");
        println!("      {{\"name\": \"Archive Page\", \"command\": \"wget -p -k\"}},");
        println!("      {{\"name\": \"Copy to Clipboard\", \"command\": \"pbcopy\"}}");
        println!("    ]");
        println!("  }}");
        println!();
        println!("{}URL Placement:{}", YELLOW, RESET);
        println!("  - No % in command  → URL appended to end");
        println!("  - % in command     → URL replaces %");
        println!("  - \\% in command    → literal % character");
        println!();
    } else {
        println!("{}Installation failed - no manifests were installed.{}", RED, RESET);
        std::process::exit(1);
    }
}

// ============================================================================
// Native Messaging Protocol
// ============================================================================

/// Read a native messaging message from stdin
fn read_message() -> io::Result<Option<Request>> {
    let mut stdin = io::stdin().lock();

    // Read 4-byte length prefix (little-endian)
    let mut len_bytes = [0u8; 4];
    match stdin.read_exact(&mut len_bytes) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }

    let len = u32::from_le_bytes(len_bytes) as usize;

    // Sanity check on message length (max 1MB)
    if len > 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Message too large",
        ));
    }

    // Read the JSON message
    let mut buffer = vec![0u8; len];
    stdin.read_exact(&mut buffer)?;

    let request: Request = serde_json::from_slice(&buffer).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("Invalid JSON: {}", e))
    })?;

    Ok(Some(request))
}

/// Write a native messaging message to stdout
fn write_message(response: &Response) -> io::Result<()> {
    let json = serde_json::to_vec(response)?;
    let len = json.len() as u32;

    let mut stdout = io::stdout().lock();
    stdout.write_all(&len.to_le_bytes())?;
    stdout.write_all(&json)?;
    stdout.flush()?;

    Ok(())
}

/// Get config directory path
fn get_config_path() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    {
        // Windows: %APPDATA%\urshell\config.json
        dirs::config_dir().map(|p| p.join("urshell").join("config.json"))
    }
    #[cfg(not(windows))]
    {
        // macOS/Linux: ~/.config/urshell/config.json
        dirs::home_dir().map(|p| p.join(".config").join("urshell").join("config.json"))
    }
}

/// Load configuration
fn load_config() -> Result<Config, String> {
    let config_path = get_config_path().ok_or("Could not determine config directory")?;

    if !config_path.exists() {
        return Err(format!(
            "Config file not found. Please create: {}",
            config_path.display()
        ));
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    let config: Config =
        serde_json::from_str(&content).map_err(|e| format!("Invalid config JSON: {}", e))?;

    if config.commands.is_empty() {
        return Err("Config must contain at least one command".to_string());
    }

    Ok(config)
}

/// Save configuration to file
fn save_config(commands: Vec<CommandConfig>) -> Result<(), String> {
    // Validate commands
    if commands.is_empty() {
        return Err("At least one command is required".to_string());
    }

    for (i, cmd) in commands.iter().enumerate() {
        if cmd.name.trim().is_empty() {
            return Err(format!("Command {} has empty name", i + 1));
        }
        if cmd.command.trim().is_empty() {
            return Err(format!("Command '{}' has empty command", cmd.name));
        }
    }

    let config_path = get_config_path().ok_or("Could not determine config directory")?;

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Build config structure
    let config = Config { commands };

    // Serialize to pretty JSON
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    // Write to file
    std::fs::write(&config_path, json)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Shell-quote a string for Unix shells using single quotes.
/// Single quotes within the string are escaped as: '\''
#[cfg(not(windows))]
fn shell_quote(s: &str) -> String {
    // Replace each ' with '\'' (end quote, escaped quote, start quote)
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

/// Shell-quote a string for Windows cmd.exe using double quotes.
/// - Double quotes are escaped as ""
/// - Percent signs are escaped as %% to prevent variable expansion
/// - Trailing backslashes before the closing quote are doubled
#[cfg(windows)]
fn shell_quote(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');

    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for (i, &c) in chars.iter().enumerate() {
        match c {
            '"' => result.push_str("\"\""),
            '%' => result.push_str("%%"),
            '\\' => {
                // Count consecutive backslashes
                let mut num_backslashes = 1;
                let mut j = i + 1;
                while j < len && chars[j] == '\\' {
                    num_backslashes += 1;
                    j += 1;
                }
                // If backslashes are followed by a quote or end of string, double them
                if j == len || chars[j] == '"' {
                    for _ in 0..num_backslashes {
                        result.push_str("\\\\");
                    }
                    // Skip the backslashes we've already processed
                    // (but we only handle current char, loop will handle rest)
                } else {
                    result.push(c);
                }
            }
            _ => result.push(c),
        }
    }

    result.push('"');
    result
}

/// Build the full command by inserting the URL.
/// - If command contains unescaped %, replace % with the quoted URL
/// - If no %, append the quoted URL to the end
/// - \% is treated as a literal %
fn build_command(command: &str, quoted_url: &str) -> String {
    let mut result = String::new();
    let mut chars = command.chars().peekable();
    let mut found_placeholder = false;

    while let Some(c) = chars.next() {
        if c == '\\' {
            // Check if next char is %
            if chars.peek() == Some(&'%') {
                // Escaped %, output literal %
                chars.next();
                result.push('%');
            } else {
                // Regular backslash
                result.push(c);
            }
        } else if c == '%' {
            // Unescaped %, replace with URL
            result.push_str(quoted_url);
            found_placeholder = true;
        } else {
            result.push(c);
        }
    }

    // If no placeholder found, append URL to the end
    if !found_placeholder {
        result.push(' ');
        result.push_str(quoted_url);
    }

    result
}
/// Shared state for running process
struct RunningProcess {
    child: Option<Child>,
}

/// Run as native messaging host (handle Chrome requests)
fn run_native_host() {
    use std::sync::mpsc;
    use std::time::Duration;

    // Shared state for the running child process
    let running: Arc<Mutex<RunningProcess>> = Arc::new(Mutex::new(RunningProcess { child: None }));

    // Channel for output from command thread
    let (output_tx, output_rx) = mpsc::channel::<Response>();

    // Wrap stdin reading in a thread so we can also check output channel
    let (stdin_tx, stdin_rx) = mpsc::channel::<Request>();
    thread::spawn(move || {
        loop {
            match read_message() {
                Ok(Some(req)) => {
                    if stdin_tx.send(req).is_err() {
                        break;
                    }
                }
                Ok(None) => break, // EOF
                Err(_) => break,
            }
        }
    });

    loop {
        // Check for messages from either stdin or command output
        // Use a small timeout to be responsive
        if let Ok(response) = output_rx.try_recv() {
            let _ = write_message(&response);
            // If complete or error, clear the running process
            if response.status == "complete" || response.status == "error" || response.status == "cancelled" {
                if let Ok(mut r) = running.lock() {
                    r.child = None;
                }
            }
            continue;
        }

        // Check for stdin with timeout
        match stdin_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(request) => {
                match request.action.as_str() {
                    "get_commands" => {
                        match load_config() {
                            Ok(config) => {
                                let _ = write_message(&Response::commands(config.commands));
                            }
                            Err(_) => {
                                let _ = write_message(&Response::commands(vec![]));
                            }
                        }
                    }
                    "save_config" => {
                        let commands = match request.commands {
                            Some(cmds) => cmds,
                            None => {
                                let _ = write_message(&Response::error("Missing commands"));
                                continue;
                            }
                        };

                        match save_config(commands) {
                            Ok(()) => {
                                let _ = write_message(&Response::saved());
                            }
                            Err(e) => {
                                let _ = write_message(&Response::error(&e));
                            }
                        }
                    }
                    "cancel" => {
                        // Kill the running process
                        if let Ok(mut r) = running.lock() {
                            if let Some(ref mut child) = r.child {
                                let _ = child.kill();
                                r.child = None;
                                let _ = write_message(&Response::cancelled());
                            }
                        }
                    }
                    "run" => {
                        let config = match load_config() {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = write_message(&Response::error(&e));
                                continue;
                            }
                        };

                        let url = match request.url {
                            Some(u) => u,
                            None => {
                                let _ = write_message(&Response::error("Missing URL"));
                                continue;
                            }
                        };

                        let index = request.command_index.unwrap_or(0);

                        if index >= config.commands.len() {
                            let _ = write_message(&Response::error(&format!(
                                "Invalid command index: {}",
                                index
                            )));
                            continue;
                        }

                        let cmd = config.commands[index].command.clone();
                        let running_clone = Arc::clone(&running);
                        let output_tx_clone = output_tx.clone();

                        // Spawn command in a thread
                        thread::spawn(move || {
                            if let Err(e) = run_command_async(&cmd, &url, running_clone, output_tx_clone) {
                                // Error already sent via channel
                                let _ = e;
                            }
                        });
                    }
                    _ => {
                        let _ = write_message(&Response::error(&format!(
                            "Unknown action: {}",
                            request.action
                        )));
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // No message, continue loop to check output
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Stdin closed, exit
                return;
            }
        }
    }
}

/// Run a command asynchronously, sending output via channel
fn run_command_async(
    command: &str,
    url: &str,
    running: Arc<Mutex<RunningProcess>>,
    output_tx: std::sync::mpsc::Sender<Response>,
) -> io::Result<()> {
    let quoted_url = shell_quote(url);
    let full_command = build_command(command, &quoted_url);

    // Send started message
    let _ = output_tx.send(Response::started());

    // Use user's shell with login mode to get their PATH and environment
    #[cfg(not(windows))]
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    #[cfg(not(windows))]
    let mut child = Command::new(&shell)
        .arg("-l")  // Login shell - sources profile
        .arg("-c")
        .arg(&full_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to start command: {}", e),
            )
        })?;

    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .arg("/c")
        .arg(&full_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to start command: {}", e),
            )
        })?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Store child handle for cancellation
    {
        if let Ok(mut r) = running.lock() {
            r.child = Some(child);
        }
    }

    // Read output
    let mut all_output = String::new();
    let reader = std::io::BufReader::new(stdout);

    for line in reader.lines() {
        // Check if we were cancelled
        {
            if let Ok(r) = running.lock() {
                if r.child.is_none() {
                    // Cancelled
                    return Ok(());
                }
            }
        }

        if let Ok(line) = line {
            all_output.push_str(&line);
            all_output.push('\n');
            let _ = output_tx.send(Response::output(&line));
        }
    }

    // Wait for process and get status
    let status = {
        if let Ok(mut r) = running.lock() {
            if let Some(ref mut c) = r.child {
                c.wait().ok()
            } else {
                None // Cancelled
            }
        } else {
            None
        }
    };

    // Read stderr
    let mut stderr_content = String::new();
    std::io::BufReader::new(stderr).read_to_string(&mut stderr_content)?;
    if !stderr_content.is_empty() {
        all_output.push_str(&stderr_content);
    }

    if let Some(status) = status {
        if status.success() {
            let _ = output_tx.send(Response::complete(all_output));
        } else {
            let error_msg = format!("Command exited with code: {}", status.code().unwrap_or(-1));
            let _ = output_tx.send(Response::error_with_output(&error_msg, all_output));
        }
    }

    Ok(())
}

fn print_help() {
    println!("URShell Native Messaging Host");
    println!();
    println!("Usage:");
    println!("  urshell-host              Run as native messaging host (called by browser)");
    println!("  urshell-host install      Detect browsers and install native messaging manifests");
    println!("  urshell-host --help       Show this help message");
    println!();
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for our explicit commands first
    if args.len() > 1 {
        match args[1].as_str() {
            "install" => {
                run_install();
                return;
            }
            "--help" | "-h" | "help" => {
                print_help();
                return;
            }
            _ => {
                // Browser passes origin URL and other args - ignore them and run as host
                // e.g.: chrome-extension://abc123/ --parent-window=0
            }
        }
    }

    // Run as native messaging host (default, or when browser passes origin args)
    run_native_host();
}
