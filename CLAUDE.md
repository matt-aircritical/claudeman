# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

ClaudeMan is a session manager for Claude Code with two frontends:
- **Rust CLI/TUI** (`src/`): Terminal-based session browser with Tantivy full-text search
- **VS Code Extension** (`vscode-extension/`): Sidebar tree view with webview conversation preview

Both share session names via `~/.config/claudeman/names.toml`.

## Build & Test

```bash
# Rust CLI
cargo build --release
cargo test

# VS Code Extension (run from vscode-extension/)
npm install
npm run build              # esbuild bundle
npm test                   # node --import tsx --test
npm run package            # creates .vsix
code --install-extension claudeman-*.vsix
```

## Architecture

### Data Flow
1. **Scanner** discovers JSONL session files in `~/.claude/projects/` (path-mangled directory names where `-` replaces `/`)
2. **Parser** reads JSONL line-by-line extracting user/assistant messages, timestamps, model info
3. **Indexer** (Rust only) builds a Tantivy full-text index with incremental mtime-based updates
4. **TUI/Extension** renders sessions in multiple view modes (all, by-project, by-date, search)

### Rust Modules
- `main.rs` — CLI entry point with clap subcommands
- `scanner.rs` — filesystem discovery, path mangling/demangling
- `parser.rs` — JSONL parsing with error tolerance
- `indexer.rs` — Tantivy schema, indexing, search queries
- `session.rs` — core Session struct
- `tui/` — ratatui app state (`mod.rs`), rendering (`ui.rs`), input handling (`input.rs`)
- `resume.rs` — launches `claude --resume` with config-driven args

### VS Code Extension Modules
- `extension.ts` — activation, tree view setup
- `sessionStore.ts` — in-memory session cache with file watching
- `treeProvider.ts` — TreeDataProvider with view modes
- `previewPanel.ts` — webview HTML rendering with CSP nonce-based scripts
- `commands.ts` — resume, fork, fork-from-exchange, rename, delete handlers
- `parser.ts` / `scanner.ts` — TypeScript equivalents of Rust parsing/scanning

### Key Design Decisions
- **Read-only**: never modifies Claude Code's JSONL files (except fork, which writes new files)
- **Webview CSP**: uses `script-src 'nonce-...'`; all click handlers must use `addEventListener` in the nonce'd script block, never inline `onclick`
- **Resume/Fork flow**: opens a new VS Code window in the session's cwd, then deep-links via `vscode://anthropic.claude-code/resume?sessionId=<id>` to load the session in Claude Code
- **Session forking**: copies JSONL lines with a new UUID session ID; fork-from-exchange truncates at the selected message
- **Names persistence**: both CLI and extension read/write the same TOML file for cross-tool session name sync

## Tests

Rust integration tests in `tests/` use a shared fixture JSONL at `tests/fixtures/projects/-home-test-project/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl`. TypeScript unit tests in `vscode-extension/test/`.
