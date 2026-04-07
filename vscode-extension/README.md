# ClaudeMan — VSCode Extension

Browse, search, preview, and resume all your Claude Code sessions from a sidebar in VSCode.

## Features

- **Session Browser** — See every Claude Code session across all projects in one sidebar, regardless of which directory started it
- **Three Views** — Browse all sessions flat, grouped by project, or grouped by date (Today, Yesterday, This Week, etc.)
- **Conversation Preview** — Click any session to see the full conversation in a richly formatted webview panel
- **Resume in Claude Code** — Select a session and resume it directly in the Claude Code VSCode extension panel (falls back to integrated terminal if the extension isn't installed)
- **Fork Sessions** — Create a new branch of any conversation with `--fork-session`
- **Rename Sessions** — Give sessions custom names for easier identification (shared with the ClaudeMan CLI)
- **Auto-Refresh** — File watcher detects new sessions as they're created
- **Entrypoint Icons** — Terminal icon for CLI sessions, chat icon for IDE sessions

## Installation

### From VSIX (local build)

```bash
cd vscode-extension
npm install
npm run build
./node_modules/.bin/vsce package --allow-missing-repository
code --install-extension claudeman-0.1.0.vsix
```

Then reload VSCode (`Ctrl+Shift+P` > "Developer: Reload Window").

## Usage

1. Click the **ClaudeMan** icon in the activity bar (left sidebar)
2. Browse your sessions in the tree view
3. Use the view toggle icons in the title bar to switch between All / By Project / Recent
4. **Click** a session to preview the conversation
5. **Right-click** for actions: Resume, Fork, Rename, Copy ID, Delete

### Keyboard

All commands are available via `Ctrl+Shift+P`:
- `ClaudeMan: Resume Session`
- `ClaudeMan: Fork Session`
- `ClaudeMan: Rename Session`
- `ClaudeMan: Preview Conversation`
- `ClaudeMan: Refresh Sessions`

## Settings

| Setting | Default | Description |
|---|---|---|
| `claudeman.claudeCommand` | `"claude"` | Path to the claude binary (used for terminal fallback and fork) |
| `claudeman.claudeArgs` | `[]` | Extra arguments appended when resuming via terminal |
| `claudeman.sessionDirectory` | auto-detect | Path to `.claude` directory |
| `claudeman.defaultView` | `"all"` | Default view: `"all"`, `"projects"`, or `"recent"` |
| `claudeman.previewOnClick` | `true` | Auto-open preview when clicking a session |

## How It Works

The extension reads session data directly from `~/.claude/projects/` — the same JSONL files that Claude Code writes. It parses them in TypeScript with no external dependencies (no Rust CLI required).

Session renames are stored in `~/.config/claudeman/names.toml` and shared with the ClaudeMan CLI tool if installed.

## Requirements

- VSCode 1.85+
- Claude Code sessions in `~/.claude/` (created by Claude Code CLI or VSCode extension)
- For resume: [Claude Code VSCode extension](https://marketplace.visualstudio.com/items?itemName=anthropic.claude-code) (recommended) or `claude` CLI in PATH
