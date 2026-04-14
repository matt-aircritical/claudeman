import * as vscode from 'vscode';
import { Session, ViewMode } from './types';
import { SessionStore } from './sessionStore';
import { PreviewPanel } from './previewPanel';
import { formatDate, projectShortName } from './utils';

export class SessionViewProvider implements vscode.WebviewViewProvider {
  private view: vscode.WebviewView | undefined;
  private viewMode: ViewMode = 'all';
  private searchFilter = '';
  private cssUri: vscode.Uri;

  constructor(
    private extensionUri: vscode.Uri,
    private store: SessionStore,
    private previewPanel: PreviewPanel,
  ) {
    this.cssUri = vscode.Uri.joinPath(extensionUri, 'media', 'sessions.css');
    store.onDidChange(() => this.update());
  }

  resolveWebviewView(webviewView: vscode.WebviewView): void {
    this.view = webviewView;
    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [vscode.Uri.joinPath(this.extensionUri, 'media')],
    };

    webviewView.webview.onDidReceiveMessage((msg) => this.handleMessage(msg));
    webviewView.onDidDispose(() => { this.view = undefined; });
    this.update();
  }

  setViewMode(mode: ViewMode): void {
    this.viewMode = mode;
    this.update();
  }

  setSearch(term: string): void {
    this.searchFilter = term.toLowerCase();
    vscode.commands.executeCommand('setContext', 'claudeman.searchActive', !!this.searchFilter);
    this.update();
  }

  clearSearch(): void { this.setSearch(''); }

  refresh(): void { this.update(); }

  private update(): void {
    if (!this.view) return;
    this.view.webview.html = this.getHtml();
  }

  private matchesSearch(s: Session): boolean {
    if (!this.searchFilter) return true;
    const q = this.searchFilter;
    const name = this.store.names.displayName(s.sessionId, s.name);
    return name.toLowerCase().includes(q)
      || s.firstUserMessage.toLowerCase().includes(q)
      || s.firstAssistantMessage.toLowerCase().includes(q)
      || s.cwd.toLowerCase().includes(q)
      || s.projectDir.toLowerCase().includes(q);
  }

  private getSessions(): { label?: string; sessions: Session[] }[] {
    switch (this.viewMode) {
      case 'all':
        return [{ sessions: this.store.getAllSessions().filter(s => this.matchesSearch(s)) }];

      case 'projects': {
        const groups = this.store.getSessionsByProject();
        return [...groups.entries()]
          .map(([dir, sessions]) => ({
            label: projectShortName(dir),
            sessions: sessions.filter(s => this.matchesSearch(s)),
          }))
          .filter(g => g.sessions.length > 0)
          .sort((a, b) => {
            const aMax = Math.max(...a.sessions.map(s => s.lastActivity));
            const bMax = Math.max(...b.sessions.map(s => s.lastActivity));
            return bMax - aMax;
          });
      }

      case 'recent': {
        const groups = this.store.getSessionsByDate();
        const order = ['Today', 'Yesterday', 'This Week', 'This Month', 'Older'];
        return order
          .map(label => ({
            label,
            sessions: (groups.get(label) || []).filter(s => this.matchesSearch(s)),
          }))
          .filter(g => g.sessions.length > 0);
      }

      default: return [];
    }
  }

  private handleMessage(msg: any): void {
    const session = msg.sessionId ? this.store.getSession(msg.sessionId) : undefined;
    switch (msg.command) {
      case 'click':
        if (session) {
          const name = this.store.names.displayName(session.sessionId, session.name);
          this.previewPanel.show(session, name);
        }
        break;
      case 'dblclick':
        if (session) {
          vscode.commands.executeCommand('claudeman.resumeSession', { session: { sessionId: session.sessionId, cwd: session.cwd } });
        }
        break;
      case 'contextAction':
        if (!session) break;
        switch (msg.action) {
          case 'resume':
            vscode.commands.executeCommand('claudeman.resumeSession', { session: { sessionId: session.sessionId, cwd: session.cwd } });
            break;
          case 'fork':
            vscode.commands.executeCommand('claudeman.forkSession', { session: { sessionId: session.sessionId, cwd: session.cwd, jsonlPath: session.jsonlPath } });
            break;
          case 'rename':
            this.renameSession(session);
            break;
          case 'copyId':
            vscode.env.clipboard.writeText(session.sessionId);
            vscode.window.showInformationMessage(`Copied: ${session.sessionId}`);
            break;
          case 'delete':
            this.deleteSession(session);
            break;
        }
        break;
      case 'search':
        this.setSearch(msg.term || '');
        break;
      case 'viewMode':
        if (['all', 'projects', 'recent'].includes(msg.mode)) {
          this.setViewMode(msg.mode);
        }
        break;
    }
  }

  private async renameSession(session: Session): Promise<void> {
    const currentName = this.store.names.displayName(session.sessionId, session.name);
    const newName = await vscode.window.showInputBox({ prompt: 'Enter new session name', value: currentName });
    if (newName?.trim()) {
      this.store.names.set(session.sessionId, newName.trim());
      this.update();
      vscode.window.showInformationMessage(`Renamed to: ${newName.trim()}`);
    }
  }

  private async deleteSession(session: Session): Promise<void> {
    const name = this.store.names.displayName(session.sessionId, session.name);
    const confirm = await vscode.window.showWarningMessage(
      `Delete "${name}" and its conversation file?`, 'Delete', 'Cancel'
    );
    if (confirm === 'Delete') {
      this.store.deleteSession(session.sessionId);
      vscode.window.showInformationMessage('Session and file deleted');
    }
  }

  private getHtml(): string {
    const webview = this.view!.webview;
    const cssHref = webview.asWebviewUri(this.cssUri);
    const nonce = getNonce();
    const groups = this.getSessions();

    let bodyHtml = '';
    const totalSessions = groups.reduce((n, g) => n + g.sessions.length, 0);

    if (totalSessions === 0) {
      bodyHtml = `<div class="empty-state">${this.searchFilter ? 'No sessions match your search' : 'No sessions found'}</div>`;
    } else {
      for (const group of groups) {
        if (group.label) {
          bodyHtml += `<div class="group-header">${escapeHtml(group.label)} (${group.sessions.length})</div>`;
        }
        for (const s of group.sessions) {
          const name = this.store.names.displayName(s.sessionId, s.name);
          const project = projectShortName(s.projectDir);
          const icon = s.entrypoint === 'ide' ? '&#128488;' : '&#9002;';
          bodyHtml += `<div class="session-card" data-id="${escapeHtml(s.sessionId)}" data-cwd="${escapeHtml(s.cwd)}" data-jsonl="${escapeHtml(s.jsonlPath)}">` +
            `<div class="session-name"><span class="session-icon">${icon}</span>${escapeHtml(name)}</div>` +
            `<div class="session-project">${escapeHtml(project)}</div>` +
            `<div class="session-dates">created: ${escapeHtml(formatDate(s.startedAt))} · updated: ${escapeHtml(formatDate(s.lastActivity))}</div>` +
            `<div class="session-meta">${s.messageCount} msgs${s.model ? ' · ' + escapeHtml(s.model) : ''}</div>` +
            `</div>`;
        }
      }
    }

    const viewBtnClass = (mode: ViewMode) => this.viewMode === mode ? 'btn active' : 'btn';

    return `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy"
    content="default-src 'none'; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';">
  <link href="${cssHref}" rel="stylesheet">
</head>
<body>
  <div class="toolbar">
    <input class="search-box" type="text" placeholder="Search sessions..." value="${escapeHtml(this.searchFilter)}">
    <button class="${viewBtnClass('all')}" data-view="all" title="All Sessions">&#9776;</button>
    <button class="${viewBtnClass('projects')}" data-view="projects" title="By Project">&#128193;</button>
    <button class="${viewBtnClass('recent')}" data-view="recent" title="Recent">&#128337;</button>
  </div>
  <div id="sessions">${bodyHtml}</div>
  <div class="context-menu" id="ctx-menu">
    <div class="context-menu-item" data-action="resume">Resume</div>
    <div class="context-menu-item" data-action="fork">Fork Session</div>
    <div class="context-menu-separator"></div>
    <div class="context-menu-item" data-action="rename">Rename</div>
    <div class="context-menu-item" data-action="copyId">Copy Session ID</div>
    <div class="context-menu-separator"></div>
    <div class="context-menu-item" data-action="delete">Delete</div>
  </div>
  <script nonce="${nonce}">
    const vscode = acquireVsCodeApi();
    let ctxSessionId = null;

    // Search
    const searchBox = document.querySelector('.search-box');
    let searchTimeout;
    searchBox.addEventListener('input', () => {
      clearTimeout(searchTimeout);
      searchTimeout = setTimeout(() => {
        vscode.postMessage({ command: 'search', term: searchBox.value });
      }, 300);
    });

    // View mode buttons
    document.querySelectorAll('.toolbar .btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const mode = btn.dataset.view;
        if (mode) vscode.postMessage({ command: 'viewMode', mode });
      });
    });

    // Session click / double-click
    const sessions = document.getElementById('sessions');
    let clickTimer = null;
    sessions.addEventListener('click', (e) => {
      const card = e.target.closest('.session-card');
      if (!card) return;
      const id = card.dataset.id;
      if (clickTimer) { clearTimeout(clickTimer); clickTimer = null; return; }
      clickTimer = setTimeout(() => {
        clickTimer = null;
        vscode.postMessage({ command: 'click', sessionId: id });
      }, 250);
    });
    sessions.addEventListener('dblclick', (e) => {
      const card = e.target.closest('.session-card');
      if (!card) return;
      if (clickTimer) { clearTimeout(clickTimer); clickTimer = null; }
      vscode.postMessage({ command: 'dblclick', sessionId: card.dataset.id });
    });

    // Context menu
    const ctxMenu = document.getElementById('ctx-menu');
    sessions.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      const card = e.target.closest('.session-card');
      if (!card) { ctxMenu.classList.remove('visible'); return; }
      ctxSessionId = card.dataset.id;
      ctxMenu.style.left = e.clientX + 'px';
      ctxMenu.style.top = e.clientY + 'px';
      ctxMenu.classList.add('visible');
    });
    document.addEventListener('click', () => ctxMenu.classList.remove('visible'));
    ctxMenu.querySelectorAll('.context-menu-item').forEach(item => {
      item.addEventListener('click', () => {
        if (ctxSessionId) {
          vscode.postMessage({ command: 'contextAction', sessionId: ctxSessionId, action: item.dataset.action });
        }
        ctxMenu.classList.remove('visible');
      });
    });
  </script>
</body>
</html>`;
  }

  dispose(): void {}
}

function getNonce(): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let text = '';
  for (let i = 0; i < 32; i++) text += chars.charAt(Math.floor(Math.random() * chars.length));
  return text;
}

function escapeHtml(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}
