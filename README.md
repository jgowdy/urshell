# URShell

A Chrome/Brave extension that passes the current page URL to configurable shell commands. URLs are properly quoted, eliminating manual copy-paste and escaping.

## Installation

### Download Release (Recommended)

1. Download `urshell.zip` from [GitHub Releases](https://github.com/jgowdy/urshell/releases)

2. Unzip anywhere on your computer

3. **Load the extension:**
   - Open `chrome://extensions` (or `brave://extensions`)
   - Enable **Developer mode**
   - Click **Load unpacked** and select the `urshell` folder

4. **Install the native host** (run from the `urshell` folder):

   **macOS / Linux:**
   ```bash
   ./install.sh
   ```

   **Windows:**
   ```
   install.bat
   ```

   The installer automatically detects your platform and which browsers have the extension installed.

   > **macOS note:** If you see "cannot be opened because the developer cannot be verified", the install script handles this automatically. If running the binary directly, first run: `xattr -d com.apple.quarantine ./native-host/macos-*/urshell-host`

### Build from Source

Requires [Rust](https://rustup.rs/).

1. Clone the repository

2. Load the extension:
   - Open `chrome://extensions` (or `brave://extensions`)
   - Enable **Developer mode**
   - Click **Load unpacked** and select the `extension/` directory

3. Build and install the native host:

   ```bash
   cd native-host
   cargo build --release
   ./target/release/urshell-host install
   ```

**Supported browsers:** Chrome, Brave, Chromium, Edge

## Configuration

Configure commands using one of these methods:

1. **Options page** (recommended): Right-click the extension icon → "Options", or click the gear icon in the popup
2. **Edit config file directly**:
   - macOS/Linux: `~/.config/urshell/config.json`
   - Windows: `%APPDATA%\urshell\config.json`

### Single Command (Auto-Execute)

With one command configured, clicking the icon **immediately executes** the command:

```json
{
  "commands": [
    {"name": "Open in Firefox", "command": "open -a Firefox"}
  ]
}
```

### Multiple Commands (Picker)

With multiple commands, clicking the icon **shows a picker** to choose which command to run:

```json
{
  "commands": [
    {"name": "Archive Page", "command": "wget -p -k"},
    {"name": "Copy to Clipboard", "command": "pbcopy"},
    {"name": "Open in Firefox", "command": "open -a Firefox"}
  ]
}
```

### URL Placement

| Pattern | Behavior |
|---------|----------|
| No `%` in command | URL appended to end |
| `%` in command | URL replaces `%` |
| `\%` in command | Literal `%` character |

**Examples:**

| Config | Executes |
|--------|----------|
| `{"command": "echo"}` | `echo 'https://...'` |
| `{"command": "curl -s % \| jq ."}` | `curl -s 'https://...' \| jq .` |
| `{"command": "wget -O /tmp/out.html"}` | `wget -O /tmp/out.html 'https://...'` |

## Usage

1. Navigate to any web page
2. Click the URShell icon in the toolbar (or press **Alt+U**)
3. **Single command**: executes immediately
4. **Multiple commands**: pick from the list
5. View output in the popup

## Security

### How URL Quoting Works

URLs are wrapped in single quotes with proper escaping:

- All characters inside single quotes are literal (no `$`, `` ` ``, `\` expansion)
- Single quotes in the URL are escaped as `'\''`

Example:
```
Input:  https://example.com/page?foo=bar&x=1
Output: 'https://example.com/page?foo=bar&x=1'
```

This is the standard POSIX shell quoting technique used by libraries like Python's `shlex`.

### Trust Model

| Component | Controlled By | If Compromised |
|-----------|---------------|----------------|
| Config file (`~/.config/urshell/config.json`) | You | Arbitrary command execution |
| Native host manifest | You | Could point to malicious binary |
| Extension | You (loaded unpacked) | N/A |
| URLs from web pages | Untrusted | Safely quoted by URShell |

**Key point**: If an attacker can write to your config directory or native messaging hosts directory, they already have code execution on your machine. URShell does not change your security posture.

### Extension Permissions

- `activeTab` - Can only read the URL of the current tab, only when you click the icon
- `nativeMessaging` - Can only communicate with the registered native host

The extension has no content scripts and cannot be triggered by web page content.

### Native Messaging Isolation

Chrome enforces that only the specific extension ID listed in the native host manifest can communicate with the host. No other extension or web page can invoke your command.

### Your Script's Responsibility

URShell safely delivers the quoted URL to your command. However, your script should still handle URLs carefully. For example, don't do this:

```bash
# BAD - re-introduces injection
eval "curl $1"
```

Instead:
```bash
# GOOD - use the argument directly
curl "$1"
```

## Requirements

- **Chrome**, **Brave**, **Chromium**, or **Edge**
- **macOS**, **Linux**, or **Windows**

## File Structure

```
urshell/
├── install.sh              # macOS/Linux installer
├── install.bat             # Windows installer
├── manifest.json
├── popup.html
├── popup.js
├── popup.css
├── background.js
├── options.html
├── options.js
├── options.css
├── icons/
└── native-host/
    ├── macos-arm64/urshell-host
    ├── macos-x64/urshell-host
    ├── linux-x64/urshell-host
    └── windows-x64/urshell-host.exe
```

## License

MIT
