# ClaudeMan — Claude Code Session Manager

## What We're Building

A Rust CLI tool that lets you find, search, and resume any Claude Code session from anywhere on your system. It solves the problem of sessions being siloed by directory — you no longer need to remember which directory a conversation started in.

You run `claudeman` from any directory, see all your sessions in an interactive TUI, search by content or project, and hit Enter to resume. Behind the scenes, it `cd`s into the session's original working directory before launching Claude, so Claude picks up the right project context (CLAUDE.md, git state, etc.) automatically.

**Phase 1:** CLI tool (this spec). **Phase 2:** VSCode extension (future spec).

## Architecture

**Language:** Rust

**Key dependencies:**
- **tantivy** — full-text search engine (indexes session content for instant search with fuzzy matching, BM25 ranking)
- **ratatui** + **crossterm** — terminal UI framework
- **serde** + **serde_json** — JSONL parsing
- **toml** — config file parsing
- **clap** — CLI argument parsing
- **dirs** — XDG directory resolution

**Binary name:** `claudeman`

## Data Sources

ClaudeMan reads from two locations in `~/.claude/`:

### Session metadata: `~/.claude/sessions/<pid>.json`
```json
{
  "pid": 81066,
  "sessionId": "2157a190-0a84-458b-912e-fe3e41d26b0c",
  "cwd": "/home/matt/projects",
  "startedAt": 1775231143367,
  "kind": "interactive",
  "entrypoint": "cli"
}
```

### Conversation content: `~/.claude/projects/<mangled-path>/<uuid>.jsonl`

Each line is a JSON object. Relevant fields:
- `type`: `"user"`, `"assistant"`, `"file-history-snapshot"`, `"progress"`
- `message.content`: the actual text (string or array for tool results)
- `timestamp`: ISO 8601
- `cwd`: working directory at time of message
- `sessionId`: UUID matching the directory name
- `version`: Claude Code version
- `permissionMode`: permission mode used

**Mangled path format:** Directory slashes replaced with dashes, leading slash becomes leading dash. Example: `-home-matt-projects` = `/home/matt/projects`.

**Subagent sessions** (`<uuid>/subagents/*.jsonl`) are ignored — only top-level conversation files are indexed.

## Index

### What gets indexed per session

| Field | Source | Indexed | Stored | Purpose |
|---|---|---|---|---|
| `session_id` | UUID from filename | yes | yes | Primary key, used for `--resume` |
| `project_dir` | Decoded from mangled parent dir | yes | yes | Grouping, display |
| `cwd` | First user message's `cwd` field | no | yes | Resume: cd here before exec |
| `started_at` | Metadata or first message timestamp | yes (as u64) | yes | Sorting, date filtering |
| `last_activity` | Last message timestamp | yes (as u64) | yes | Sorting |
| `name` | `--name` flag or first user message (truncated to 80 chars) | yes | yes | Display, search |
| `model` | From message metadata | no | yes | Display |
| `version` | From message metadata | no | yes | Display |
| `message_count` | Count of user+assistant messages | no | yes | Display |
| `user_messages` | All user message text, concatenated | yes (full-text) | no | Full-text search |
| `assistant_messages` | All assistant message text, concatenated | yes (full-text) | no | Full-text search |
| `first_exchange` | First user message + first assistant response (500 chars each) | no | yes | Preview pane content |
| `file_mtime` | JSONL file modification time | no | yes | Incremental indexing |

### Index location

`~/.config/claudeman/index/` (Tantivy index directory)

### Incremental indexing

On startup, ClaudeMan:
1. Scans `~/.claude/projects/` for all `*.jsonl` files (excluding `subagents/`)
2. Compares each file's mtime against the stored `file_mtime` in the index
3. Only re-parses and re-indexes files that are new or changed
4. Removes index entries for sessions whose JSONL files no longer exist

This makes startup fast — typically <100ms for an incremental check with no changes.

### Manual re-index

`r` keybinding in TUI or `claudeman index` on CLI forces a full re-index (ignores mtimes).

## CLI Interface

### Usage modes

```
claudeman                    # Launch interactive TUI (default)
claudeman search <query>     # Jump to TUI with search pre-filled
claudeman list               # Non-interactive list to stdout (for piping)
claudeman index              # Force full re-index
claudeman config             # Print config path and current values
```

### Configuration

**File:** `~/.config/claudeman/config.toml`

```toml
# Extra args appended when resuming: claude --resume <id> [these args]
claude_args = []

# Path to claude binary (auto-detected from PATH if not set)
# claude_bin = "claude"

# Claude data directory (auto-detected if not set)
# claude_dir = "~/.claude"
```

Example for the user's setup:
```toml
claude_args = ["--dangerously-skip-permissions"]
```

## TUI Design

### Layout

```
┌─────────────────────────────────────────────────────────────┐
│ ⌕ Search: [_______________]        ↑↓:nav │ /:search │ q:quit│
├─────────────────────────────────────────────────────────────┤
│ [All (43)] [By Project] [By Date] [Search Results]          │
├──────────────────────────┬──────────────────────────────────┤
│ Session List             │ Preview                          │
│                          │                                  │
│ ▸ vibeinbox-plan read..  │ SESSION PREVIEW                  │
│   /home/matt/projects    │ ID: a9665305-9601-...            │
│   Mar 9 · 26 messages    │ Started: 2026-03-09 12:11        │
│                          │ CWD: /home/matt/projects         │
│   examine cienone Goo..  │ Model: claude-opus-4-6           │
│   /home/matt/projects    │                                  │
│   Apr 3 · 14 messages    │ YOU:                             │
│                          │ vibeinbox-plan.md read this      │
│   mouse pointer size..   │ file and ask clarifying...       │
│   /home/matt/projects    │                                  │
│   Apr 3 · 8 messages     │ CLAUDE:                          │
│                          │ I'll read the VibeInbox plan     │
│                          │ document and then ask you...     │
├──────────────────────────┴──────────────────────────────────┤
│ 43 sessions │ 20 projects   ENTER:resume f:fork r:reindex   │
└─────────────────────────────────────────────────────────────┘
```

### View modes (tabs)

1. **All** — every session, sorted by last activity (newest first)
2. **By Project** — grouped by project directory, collapsible groups
3. **By Date** — grouped by day/week
4. **Search Results** — shown when a search query is active, ranked by relevance

### Key bindings

| Key | Action |
|---|---|
| `↑` / `↓` | Navigate session list |
| `Enter` | Resume session (cd to original dir, exec claude --resume) |
| `Space` | Expand/collapse in grouped views |
| `/` | Activate search input |
| `Esc` | Clear search / deactivate search input |
| `Tab` | Cycle through view mode tabs |
| `f` | Fork session (resume with --fork-session) |
| `n` | Rename session (inline text input, pre-filled with current name) |
| `r` | Force re-index |
| `q` | Quit |

### Search behavior

- Typing in the search input filters in real-time using Tantivy
- Fuzzy matching enabled (handles typos)
- Results ranked by BM25 relevance
- Search queries across: session name, user messages, assistant messages, project directory

## Resume Behavior

When the user presses Enter on a session:

1. ClaudeMan reads the stored `cwd` for that session
2. If the directory exists, `cd` into it
3. If the directory doesn't exist, stay in current directory (non-fatal)
4. Exec: `claude --resume <session_id> [config.claude_args...]`
5. ClaudeMan's process is replaced by Claude (exec, not spawn)

When the user presses `f` (fork):

Same as above but adds `--fork-session` to the args.

## JSONL Parsing

### Extracting user messages

For each line in the JSONL:
- If `type == "user"` and `message.content` is a string → use it
- If `type == "user"` and `message.content` is an array → extract text items, skip tool_result items (they're noisy)
- If `type == "assistant"` → extract `message.content` text (skip tool_use blocks)
- Skip: `file-history-snapshot`, `progress`, and any type that isn't `user` or `assistant`

### Decoding mangled paths

The parent directory name uses this encoding:
- Leading `-` = leading `/`
- Internal `-` = `/`

So `-home-matt-projects-nanoclaw` → `/home/matt/projects/nanoclaw`

Edge case: directory names with actual dashes. We handle this by checking if the decoded path exists on disk. If not, try keeping segments with dashes together. In practice, Claude Code's encoding is deterministic and matches the filesystem.

## Session Renaming

Users can rename any session by pressing `n` in the TUI.

**Display name priority:** Custom name > `--name` flag from Claude > first user message (truncated to 80 chars).

**Storage:** Custom names are stored in a sidecar file at `~/.config/claudeman/names.toml`, mapping session IDs to names. We never modify Claude's own session files.

```toml
# ~/.config/claudeman/names.toml
[names]
"a9665305-9601-4aeb-b539-417b117e92b5" = "VibeInbox planning session"
"2157a190-0a84-458b-912e-fe3e41d26b0c" = "Mouse pointer & Wayland fixes"
```

**UX:** Pressing `n` opens an inline text input over the session name, pre-filled with the current name. Enter confirms, Esc cancels. The custom name is also indexed in Tantivy for search.

## Error Handling

- **Missing JSONL files:** Skip gracefully, warn on stderr
- **Malformed JSON lines:** Skip the line, continue parsing
- **Missing claude binary:** Error with helpful message pointing to install
- **Empty index:** Show "No sessions found. Run `claudeman index` or start a Claude session first."
- **Corrupted index:** Delete and rebuild automatically

## Not In Scope (Phase 1)

- VSCode extension (phase 2)
- Session tagging, favorites, or starring
- Session deletion from disk (only from index)
- Remote sessions
- Multi-user support
- Session export/import
