# ClaudeMan VSCode Extension — Design Spec

## What We're Building

A VSCode sidebar extension that lets you browse, search, preview, and resume all your Claude Code sessions from within VSCode. It shows sessions from both the terminal CLI and the VSCode Claude Code extension — everything that lives in `~/.claude/projects/`.

The extension is fully self-contained in TypeScript. It does not depend on the `claudeman` Rust CLI being installed. It reads session data directly from disk and shares the `names.toml` rename store with the CLI tool.

**Architecture:** TreeView sidebar for session browsing + Webview panel for rich conversation preview.

## Extension Identity

- **Display name:** `ClaudeMan` (used in UI, commands, sidebar title)
- **Extension ID:** Defined in `package.json` — not hardcoded into source logic. Easy to change for rebranding or marketplace publishing.
- **Location:** `vscode-extension/` directory inside the claudeman monorepo

## Data Layer (Pure TypeScript)

### Scanner

Walks `~/.claude/projects/<mangled-path>/` finding `*.jsonl` files. Same logic as the Rust scanner:
- Skips subdirectories (subagents/, tool-results/)
- Validates UUID filenames (36 chars, 4 dashes)
- Records file mtime for change detection
- Decodes mangled paths: `-home-matt-projects` → `/home/matt/projects`

### Parser

Reads JSONL files line by line. For each line:
- `type == "user"`: extract message content (string or array), cwd, version, timestamp
- `type == "assistant"`: extract message content, model, timestamp
- Skip everything else (file-history-snapshot, progress, etc.)

For array content, extract `text` type items, skip `tool_result` items.

Produces a `Session` object with: sessionId, projectDir, cwd, startedAt, lastActivity, name, model, version, messageCount, firstUserMessage, firstAssistantMessage, entrypoint, jsonlPath.

### Lazy Loading

- **On activation:** Scanner runs, collects file paths and mtimes. Parses only the first few lines of each JSONL to get the session name and basic metadata (fast).
- **On preview click:** Full parse of the selected session's JSONL to load all conversation exchanges.
- **File watcher:** Watches `~/.claude/projects/` for new/changed `.jsonl` files. Triggers incremental refresh.

### Names Store

Reads and writes `~/.config/claudeman/names.toml` — the same file used by the CLI tool. Format:

```toml
[names]
"session-uuid" = "Custom display name"
```

Renaming in the extension is visible in the CLI and vice versa.

### Search / Filtering

Simple in-memory string matching — no full-text index needed at extension scale. The TreeView's built-in type-to-filter handles most cases. For the command palette search, we fuzzy-match across session name, project dir, and first user message.

## Sidebar TreeView

### Activity Bar

The extension contributes a view container to the activity bar with the ClaudeMan icon. Contains one view: the session tree.

### View Modes

Three modes, toggled via icons in the view title bar:

1. **All** — Flat list of all sessions, sorted by last activity (newest first). Each item shows:
   - Chat bubble icon (💬) — with a subtle terminal/IDE indicator based on `entrypoint` field
   - Session name (first user message, truncated, or custom name from names.toml)
   - Description line: project shortname · relative date · message count

2. **By Project** — Tree grouped by project directory. Top-level nodes are folder icons with project name and session count. Expand to see sessions under each project. Projects sorted by most recent session activity.

3. **Recent** — Grouped by time period: Today, Yesterday, This Week, This Month, Older. Each group is a collapsible node.

### Session Tree Item

Each session node displays:
- **Label:** Custom name (from names.toml) > session name (first user message truncated to 60 chars)
- **Description:** `project-shortname · 2h ago · 48 msgs`
- **Icon:** Chat bubble, with color/badge hinting at entrypoint (`cli` vs `ide`)
- **Tooltip:** Full session ID, working directory, model, start date

### Context Menu (Right-Click)

| Action | Description |
|---|---|
| Resume Session | Opens VSCode integrated terminal with `claude --resume <id>` |
| Fork Session | Same but adds `--fork-session` |
| Rename | Inline input to set custom name (saved to names.toml) |
| Preview Conversation | Opens/focuses the webview preview panel |
| Copy Session ID | Copies full UUID to clipboard |
| Delete from Index | Removes from the in-memory list (does not delete files) |

### Inline Actions

Small icons on hover:
- ▶ Resume (play icon)
- 👁 Preview (eye icon)

### Filter Input

VSCode TreeView supports built-in filtering when user types. This filters session names and descriptions. Additionally, a custom search command (`ClaudeMan: Search Sessions`) opens a QuickPick for fuzzy search across all content.

## Webview Preview Panel

When a session is clicked (single click) or "Preview Conversation" is chosen, a webview panel opens in the editor area.

### Layout

```
┌──────────────────────────────────────────────────┐
│ ClaudeMan Preview                           [x]  │
├──────────────────────────────────────────────────┤
│ Session: "session manager for claude code"       │
│ /home/matt/projects/claudeman                    │
│ Started 0h ago · 412 messages · claude-opus-4-6  │
│ [▶ Resume]  [⑂ Fork]  [✎ Rename]                │
│ ──────────────────────────────────────────────── │
│                                                  │
│ YOU [1]                                          │
│ good morning claude, So I have a problem...      │
│                                                  │
│ CLAUDE [2]                                       │
│ Good morning! This is a great idea...            │
│                                                  │
│ YOU [3]                                          │
│ yes that is correct except...                    │
│                                                  │
│ (... scrollable ...)                             │
│                                                  │
│ [Load more messages]                             │
└──────────────────────────────────────────────────┘
```

### Behavior

- **Single click** on tree item → preview panel opens/updates to that session
- **Double click** on tree item → resume immediately (opens terminal)
- **Preview panel reuse:** Same panel updates when you click different sessions (like Markdown preview). Only one preview panel at a time.
- **Lazy message loading:** Loads first 50 exchanges on open, "Load more" button at bottom fetches the rest
- **Action buttons:** Resume, Fork, Rename wired to the same commands as the context menu
- **Styled conversation:** User messages in green header, assistant messages in blue header. Monospace code blocks. Clean readable layout.

### Webview Implementation

- HTML/CSS rendered in a VSCode webview panel
- Extension sends session data to webview via `postMessage`
- Webview sends button clicks back to extension via `postMessage`
- Uses VSCode's webview CSS variables for theme compatibility (works in dark/light themes)
- Content Security Policy set appropriately

## Commands

| Command | Title | Description |
|---|---|---|
| `claudeman.resumeSession` | Resume Session | QuickPick to search and resume a session |
| `claudeman.forkSession` | Fork Session | Fork the selected session |
| `claudeman.renameSession` | Rename Session | Input box to rename selected session |
| `claudeman.previewSession` | Preview Conversation | Open webview preview for selected session |
| `claudeman.copySessionId` | Copy Session ID | Copy UUID to clipboard |
| `claudeman.deleteSession` | Delete from Index | Remove session from list |
| `claudeman.refreshSessions` | Refresh Sessions | Force re-scan of session files |
| `claudeman.toggleView` | Toggle View | Cycle All → Projects → Recent |

## Settings

| Setting | Type | Default | Description |
|---|---|---|---|
| `claudeman.claudeCommand` | string | `"claude"` | Path to claude binary |
| `claudeman.claudeArgs` | array | `[]` | Extra args appended to resume command |
| `claudeman.sessionDirectory` | string | `""` (auto-detect) | Path to `.claude` directory |
| `claudeman.defaultView` | enum | `"all"` | Default view mode: `"all"`, `"projects"`, `"recent"` |
| `claudeman.previewOnClick` | boolean | `true` | Auto-open preview when clicking a session |

## Resume Behavior

When resuming a session:
1. Get the session's original `cwd` from parsed metadata
2. Create a new VSCode integrated terminal
3. If `cwd` exists on disk, set the terminal's working directory to it
4. Run: `{claudeCommand} --resume {sessionId} {claudeArgs...}`
5. Focus the terminal

Fork is identical but adds `--fork-session` to the command.

## File Structure

```
vscode-extension/
├── package.json          # Extension manifest, commands, settings, views
├── tsconfig.json
├── .vscodeignore
├── src/
│   ├── extension.ts      # Activation, registration, disposal
│   ├── scanner.ts        # Discovers JSONL session files
│   ├── parser.ts         # Parses JSONL content into Session objects
│   ├── types.ts          # Session, Exchange, and other shared types
│   ├── names.ts          # Read/write names.toml (shared with CLI)
│   ├── sessionStore.ts   # In-memory session store, file watcher, refresh
│   ├── treeProvider.ts   # TreeDataProvider for sidebar views
│   ├── treeItems.ts      # TreeItem subclasses (session, project group, date group)
│   ├── previewPanel.ts   # Webview panel manager for conversation preview
│   ├── commands.ts       # Command handlers (resume, fork, rename, delete, etc.)
│   └── utils.ts          # Path decoding, date formatting, string helpers
├── media/
│   ├── icon.svg          # Activity bar icon
│   ├── chat.svg          # Session item icon
│   └── preview.css       # Webview preview stylesheet
└── test/
    ├── scanner.test.ts
    ├── parser.test.ts
    └── names.test.ts
```

## Entrypoint Indicator

Sessions have an `entrypoint` field in the metadata JSON (`"cli"` or `"ide"`). The tree item icon will have a subtle visual distinction:
- **CLI sessions:** Standard chat bubble icon
- **IDE sessions:** Chat bubble with a small code bracket badge

This is informational only — both types are fully browsable, previewable, and resumable.

## Not In Scope

- Marketplace publishing (local VSIX install for now)
- Full-text search index (simple string filter is sufficient)
- Editing or modifying conversation content
- Multi-workspace session isolation
- Session export/import
- Keyboard shortcuts (users bind via VSCode's keybinding system)
