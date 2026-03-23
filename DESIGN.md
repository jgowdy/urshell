# URShell - URL to Shell Extension

## Overview

URShell is a Chrome/Brave extension that passes the current page URL to a configurable shell command. The URL is properly shell-quoted, eliminating the need to manually copy, escape, and paste URLs.

## Use Cases

- Run a download script on the current page
- Send URLs to a bookmark manager
- Trigger automation workflows
- Archive web pages
- Any command-line tool that accepts a URL

## Architecture

Chrome extensions run in a sandboxed environment and cannot execute shell commands directly. URShell uses Chrome's **Native Messaging** API to bridge the extension to a local Rust binary that executes the configured command.

```
┌─────────────────┐      Native Messaging      ┌──────────────────┐
│                 │         Protocol           │                  │
│  Chrome/Brave   │ ◄─────────────────────────►│  URShell Host    │
│  Extension      │    (JSON over stdin/out)   │  (Rust binary)   │
│                 │                            │                  │
└─────────────────┘                            └────────┬─────────┘
                                                        │
                                                        │ sh -c
                                                        ▼
                                               ┌──────────────────┐
                                               │  Your Command    │
                                               │  + quoted URL    │
                                               └──────────────────┘
```

## Components

### 1. Chrome Extension

**Manifest (manifest.json)** - Manifest V3 format
- Permissions: `activeTab`, `nativeMessaging`
- Action: Browser toolbar button with popup
- Keyboard shortcut: Alt+U

**Popup UI (popup.html / popup.js / popup.css)**
- Dark themed interface showing current URL
- "Run Command" button
- Real-time output display
- Status indicator (running, complete, error)

**Background Service Worker (background.js)**
- Handles keyboard shortcut (Alt+U)
- Shows browser notifications on completion

### 2. Native Messaging Host (Rust)

**Host Application (urshell-host)**
- Compiled Rust binary for safety
- Reads configuration from `~/.config/urshell/config.json`
- Implements Chrome native messaging protocol
- Shell-quotes the URL using single quotes with proper escaping
- Executes command via `sh -c` and streams output back

**Configuration File (~/.config/urshell/config.json)**

Single command (auto-executes on click):
```json
{
  "commands": [
    {"name": "Download", "command": "~/bin/dl"}
  ]
}
```

Multiple commands (shows picker):
```json
{
  "commands": [
    {"name": "Download Video", "command": "~/bin/dl"},
    {"name": "Archive Page", "command": "~/bin/archive"}
  ]
}
```

**URL Placement:**
- No `%` in command → URL is appended to the end
- `%` in command → URL replaces `%` at that position
- `\%` → literal `%` character

Examples:
```json
{"name": "Download", "command": "~/bin/dl"}
→ ~/bin/dl 'https://example.com/page'

{"name": "Fetch", "command": "curl -o /tmp/page.html %"}
→ curl -o /tmp/page.html 'https://example.com/page'
```

## File Structure

**Release package (urshell.zip):**
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
│   ├── icon16.png
│   ├── icon48.png
│   └── icon128.png
└── native-host/
    ├── macos-arm64/urshell-host
    ├── macos-x64/urshell-host
    ├── linux-x64/urshell-host
    └── windows-x64/urshell-host.exe
```

**Source repository:**
```
urshell/
├── extension/           # Browser extension source
├── native-host/         # Rust native host source
│   ├── Cargo.toml
│   └── src/main.rs
├── README.md
└── DESIGN.md
```

## Message Protocol

### Extension → Native Host

```json
{
  "action": "run",
  "url": "https://example.com/page?id=123"
}
```

### Native Host → Extension

**Started:**
```json
{
  "status": "started"
}
```

**Output (streamed line by line):**
```json
{
  "status": "output",
  "data": "Processing..."
}
```

**Completion:**
```json
{
  "status": "complete",
  "output": "Full command output here"
}
```

**Error:**
```json
{
  "status": "error",
  "message": "Command exited with code: 1",
  "output": "Error output here"
}
```

## URL Quoting

URLs are quoted using single quotes with proper escaping:

| URL Character | Handling |
|--------------|----------|
| Most characters | Preserved inside single quotes |
| Single quote `'` | Escaped as `'\''` |

Example:
- Input: `https://example.com/page?foo=bar&baz=qux`
- Quoted: `'https://example.com/page?foo=bar&baz=qux'`

Edge case with single quote:
- Input: `https://example.com/it's-a-page`
- Quoted: `'https://example.com/it'\''s-a-page'`

## Installation

### Extension
1. Open Chrome/Brave: `chrome://extensions` or `brave://extensions`
2. Enable "Developer mode"
3. Click "Load unpacked" and select the `urshell/` folder

### Native Host
1. Run the installer from the `urshell` folder:
   - macOS/Linux: `./install.sh`
   - Windows: `install.bat`
2. The installer automatically:
   - Scans for Chrome, Brave, Chromium, and Edge
   - Detects which browsers have URShell installed
   - Installs native messaging manifests for each
   - Creates config at `~/.config/urshell/config.json`
3. Edit the config file to set your commands

## Security Considerations

- **Native messaging isolation**: Only the specific extension ID can communicate with the host
- **Proper quoting**: Single-quote escaping prevents shell injection
- **No shell interpolation**: URLs are never interpreted by the shell
- **Memory safety**: Rust prevents buffer overflows and memory corruption
- **Message size limits**: 1MB maximum prevents memory exhaustion

## Dependencies

- **Rust** (for building the native host)
- **Chrome, Brave, or Chromium browser**
- **macOS, Linux, or Windows**

## Examples

### Example 1: Simple append (URL at end)
```json
{"command": "echo"}
```
Executes: `echo 'https://...'`

### Example 2: Custom script
```json
{"command": "~/bin/dl"}
```
Executes: `~/bin/dl 'https://...'`

### Example 3: URL in middle of command
```json
{"command": "curl -s % | jq ."}
```
Executes: `curl -s 'https://...' | jq .`

### Example 4: Open in different browser
```json
{"command": "open -a Firefox"}
```
Executes: `open -a Firefox 'https://...'`

### Example 5: Wget with output file
```json
{"command": "wget -O /tmp/download.html"}
```
Executes: `wget -O /tmp/download.html 'https://...'`

### Example 6: Literal percent sign
```json
{"command": "echo Downloading at 100\\% speed:"}
```
Executes: `echo Downloading at 100% speed: 'https://...'`
