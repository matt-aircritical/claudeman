# ClaudeMan

A session manager for [Claude Code](https://claude.ai/claude-code) that lets you find, search, preview, and resume any session from anywhere on your system.

Claude Code's built-in session picker is scoped to your current directory. If you started a conversation from `/home/matt/projects/app-a` but you're now in `/home/matt/projects/app-b`, you can't find it. ClaudeMan solves this by indexing every session across all directories into one searchable interface.

## What's Included

### CLI Tool (Rust)

An interactive terminal UI for browsing and resuming sessions.

```
claudeman              # Launch the TUI
claudeman search auth  # Search for sessions mentioning "auth"
claudeman list         # Print all sessions to stdout
claudeman index        # Force re-index all sessions
claudeman config       # Show config paths
```

**Features:**
- Full-text search across all conversation content (powered by Tantivy)
- Three views: All Sessions, By Project, By Date
- Split-pane layout with conversation preview
- Expand preview to read full conversations (`p` key)
- Rename sessions for easier identification (`n` key)
- Resume directly into Claude Code (`Enter`)
- Fork sessions to branch a conversation (`f` key)
- Delete sessions from the index (`d` key)
- Auto-indexes on startup, incremental updates

### VSCode Extension (TypeScript)

A sidebar extension that integrates session management into VSCode.

**Features:**
- Sidebar tree view with All / By Project / Recent groupings
- Click any session to preview the full conversation in a webview panel
- Right-click context menu: Resume in Terminal, Resume in New Window, Fork, Rename, Copy ID, Delete
- Resume in New Window opens VSCode in the session's original directory
- Shares renamed session names with the CLI tool
- File watcher auto-refreshes when new sessions appear
- Terminal/IDE icons distinguish where sessions originated

## Install

### CLI

Requires Rust toolchain.

```bash
git clone https://github.com/matt-adroited/claudeman.git
cd claudeman
cargo install --path .
```

Then run `claudeman` from any directory.

### VSCode Extension

```bash
cd claudeman/vscode-extension
npm install
npm run build
./node_modules/.bin/vsce package --allow-missing-repository
code --install-extension claudeman-0.2.0.vsix
```

Reload VSCode. The ClaudeMan icon appears in the activity bar.

## Configuration

### CLI Config

`~/.config/claudeman/config.toml`

```toml
# Extra args appended when resuming sessions
claude_args = ["--dangerously-skip-permissions"]

# Path to claude binary (auto-detected if not set)
# claude_bin = "claude"
```

### VSCode Settings

| Setting | Default | Description |
|---|---|---|
| `claudeman.claudeCommand` | `"claude"` | Path to the claude binary |
| `claudeman.claudeArgs` | `[]` | Extra arguments for resume |
| `claudeman.sessionDirectory` | auto-detect | Path to `.claude` directory |
| `claudeman.defaultView` | `"all"` | Default view: `"all"`, `"projects"`, `"recent"` |

### Shared State

Session renames are stored in `~/.config/claudeman/names.toml` and shared between the CLI and VSCode extension. Rename in one, see it in the other.

## How It Works

Claude Code stores session data in `~/.claude/projects/` as JSONL files. Each file contains the full conversation: user messages, assistant responses, tool calls, and metadata.

ClaudeMan reads these files directly:

- **CLI:** Parses JSONL files and indexes them into a Tantivy full-text search index at `~/.config/claudeman/index/`. Searches are instant.
- **VSCode Extension:** Parses JSONL files in TypeScript at startup with in-memory filtering. No external dependencies.

Neither tool modifies Claude Code's session files. The only files ClaudeMan writes are its own index and the shared `names.toml`.

## Key Bindings (CLI)

| Key | Action |
|---|---|
| `Up/Down` | Navigate sessions |
| `Enter` | Resume session |
| `f` | Fork session |
| `p` | Toggle expanded preview |
| `d` | Delete from index |
| `/` | Search |
| `n` | Rename session |
| `Tab` | Cycle views |
| `h` or `?` | Help screen |
| `q` | Quit |

## Tech Stack

- **CLI:** Rust, Tantivy (search), ratatui (TUI), clap (CLI), serde (parsing)
- **Extension:** TypeScript, VSCode Extension API, smol-toml, esbuild

## License

MIT
