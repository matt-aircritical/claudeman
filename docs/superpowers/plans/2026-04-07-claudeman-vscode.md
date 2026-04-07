# ClaudeMan VSCode Extension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a VSCode sidebar extension that browses, searches, previews, and resumes Claude Code sessions using a native TreeView + Webview preview panel.

**Architecture:** TypeScript extension with a TreeDataProvider for the sidebar tree, a WebviewPanel for conversation preview, and a pure-TypeScript data layer that parses Claude Code JSONL session files directly from disk. Shares the names.toml rename store with the CLI tool.

**Tech Stack:** TypeScript, VSCode Extension API, Node.js fs/path, smol-toml, esbuild

---

## File Structure

```
vscode-extension/
├── package.json
├── tsconfig.json
├── esbuild.js
├── .vscodeignore
├── src/
│   ├── extension.ts
│   ├── types.ts
│   ├── scanner.ts
│   ├── parser.ts
│   ├── names.ts
│   ├── sessionStore.ts
│   ├── treeProvider.ts
│   ├── treeItems.ts
│   ├── previewPanel.ts
│   ├── commands.ts
│   └── utils.ts
├── media/
│   ├── icon.svg
│   └── preview.css
└── test/
    ├── fixtures/projects/-home-test-project/
    │   └── aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl
    ├── scanner.test.ts
    ├── parser.test.ts
    └── utils.test.ts
```

---

## Task 1: Project Scaffold

**Files:**
- Create: `vscode-extension/package.json`
- Create: `vscode-extension/tsconfig.json`
- Create: `vscode-extension/esbuild.js`
- Create: `vscode-extension/.vscodeignore`
- Create: `vscode-extension/src/types.ts`
- Create: `vscode-extension/src/extension.ts`

All contribution points (views, commands, menus, settings) defined upfront in package.json. Minimal extension.ts that just logs activation. Types define Session, Exchange, DiscoveredSession, ViewMode interfaces.

Dependencies: smol-toml, esbuild, @types/vscode, typescript, tsx, @vscode/vsce.

Verify: `cd vscode-extension && npm install && npm run build` produces dist/extension.js.

Commit: `feat(vscode): project scaffold with package.json, types, build`

---

## Task 2: Utility Functions

**Files:**
- Create: `vscode-extension/src/utils.ts`
- Create: `vscode-extension/test/utils.test.ts`

Functions: decodeMangledPath, formatDate, shortenPath, truncateText (char-safe), projectShortName, isUuid.

TDD: write tests first, verify fail, implement, verify pass.

Commit: `feat(vscode): utility functions for path decoding, date formatting`

---

## Task 3: Scanner

**Files:**
- Create: `vscode-extension/src/scanner.ts`
- Create: `vscode-extension/test/scanner.test.ts`
- Create: test fixture JSONL

Scanner walks claudeDir/projects/*/, finds *.jsonl with UUID names, skips subdirectories, records mtime. Returns DiscoveredSession[].

TDD with test fixtures.

Commit: `feat(vscode): session scanner discovers JSONL files`

---

## Task 4: Parser

**Files:**
- Create: `vscode-extension/src/parser.ts`
- Create: `vscode-extension/test/parser.test.ts`

Two functions:
- parseSessionMetadata(discovered) - reads JSONL, extracts Session object
- parseSessionExchanges(jsonlPath) - reads all user/assistant exchanges for preview

Handles string and array content formats. Char-safe truncation for name (80 chars).

TDD.

Commit: `feat(vscode): JSONL parser for session metadata and exchanges`

---

## Task 5: Names Store

**Files:**
- Create: `vscode-extension/src/names.ts`

NameStore class reads/writes ~/.config/claudeman/names.toml using smol-toml. Methods: load, get, set, delete, displayName. Shared with CLI tool.

Commit: `feat(vscode): names store reads/writes shared names.toml`

---

## Task 6: Session Store

**Files:**
- Create: `vscode-extension/src/sessionStore.ts`

SessionStore wraps scanner + parser + names. Methods:
- refresh() - scan and parse, reuse unchanged sessions
- startWatching/stopWatching - FileSystemWatcher on projects/**/*.jsonl
- getAllSessions() - sorted by lastActivity desc
- getSessionsByProject() - Map grouped by projectDir
- getSessionsByDate() - Map with Today/Yesterday/This Week/This Month/Older
- getSession(id), deleteSession(id)
- onDidChange event for tree refresh

Commit: `feat(vscode): session store with file watcher and grouping`

---

## Task 7: Tree Items

**Files:**
- Create: `vscode-extension/src/treeItems.ts`

SessionItem: label=displayName, description=project+date+count, icon=terminal/comment-discussion by entrypoint, contextValue='session', tooltip with details, click command=previewSession.

GroupItem: label=name, description=(count), icon=folder/calendar, Expanded state, contextValue='group'.

Commit: `feat(vscode): tree item classes for sessions and groups`

---

## Task 8: Tree Data Provider

**Files:**
- Create: `vscode-extension/src/treeProvider.ts`

TreeDataProvider with ViewMode state. getChildren returns flat SessionItems (all), GroupItems with children (projects/recent). setViewMode fires refresh. Listens to store.onDidChange.

Commit: `feat(vscode): tree data provider with all/projects/recent views`

---

## Task 9: Preview Panel

**Files:**
- Create: `vscode-extension/src/previewPanel.ts`
- Create: `vscode-extension/media/preview.css`

CSS uses VSCode theme variables for full theme compatibility. PreviewPanel creates/reuses WebviewPanel. Shows session header + action buttons + conversation exchanges. Loads first 50 exchanges, Load more button for rest. All session content escaped with escapeHtml before injection into HTML template. Proper Content-Security-Policy with nonces. postMessage for resume/fork/loadMore actions.

Commit: `feat(vscode): webview preview panel with conversation display`

---

## Task 10: Commands

**Files:**
- Create: `vscode-extension/src/commands.ts`

registerCommands function registers all 10 commands:
- resumeSession/forkSession - create VSCode terminal, cd to cwd, run claude --resume
- renameSession - showInputBox, save to names.toml
- previewSession - open webview panel
- copySessionId - clipboard
- deleteSession - confirm dialog, store.deleteSession
- refreshSessions - store.refresh()
- viewAll/viewByProject/viewRecent - treeProvider.setViewMode()

Commit: `feat(vscode): command handlers for resume, fork, rename, delete`

---

## Task 11: Activity Bar Icon

**Files:**
- Create: `vscode-extension/media/icon.svg`

24x24 chat bubble SVG using currentColor for theme compatibility.

Commit: `feat(vscode): activity bar chat icon`

---

## Task 12: Wire Everything in extension.ts

**Files:**
- Modify: `vscode-extension/src/extension.ts`

activate() creates SessionStore, SessionTreeProvider, PreviewPanel. Sets default view from settings. Registers tree view and commands. Calls store.refresh() and startWatching(). Disposes on deactivate.

Commit: `feat(vscode): wire up activation with store, tree, preview, commands`

---

## Task 13: Package, Install, and Smoke Test

Build: `npm run build && npx @vscode/vsce package --allow-missing-repository`
Install: `code --install-extension claudeman-0.1.0.vsix`

Smoke test: activity bar icon, session tree, view toggles, context menu, preview panel, resume terminal, rename.

Commit and merge to main.
