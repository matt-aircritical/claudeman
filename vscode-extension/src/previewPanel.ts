import * as vscode from 'vscode';
import { Session, Exchange } from './types';
import { parseSessionExchanges } from './parser';
import { formatDate } from './utils';

export class PreviewPanel {
  private panel: vscode.WebviewPanel | undefined;
  private currentSessionId = '';
  private cssUri: vscode.Uri;

  constructor(private extensionUri: vscode.Uri) {
    this.cssUri = vscode.Uri.joinPath(extensionUri, 'media', 'preview.css');
  }

  show(session: Session, displayName: string): void {
    if (this.panel) {
      this.panel.reveal(vscode.ViewColumn.One, true);
    } else {
      this.panel = vscode.window.createWebviewPanel(
        'claudeman.preview', 'ClaudeMan Preview',
        { viewColumn: vscode.ViewColumn.One, preserveFocus: true },
        {
          enableScripts: true,
          localResourceRoots: [vscode.Uri.joinPath(this.extensionUri, 'media')],
        }
      );

      this.panel.onDidDispose(() => {
        this.panel = undefined;
        this.currentSessionId = '';
      });

      this.panel.webview.onDidReceiveMessage((msg) => {
        switch (msg.command) {
          case 'resume':
            vscode.commands.executeCommand('claudeman.resumeSession', { session: msg.session });
            break;
          case 'fork':
            vscode.commands.executeCommand('claudeman.forkSession', { session: msg.session });
            break;
          case 'forkFromExchange':
            vscode.commands.executeCommand('claudeman.forkFromExchange', msg);
            break;
          case 'loadMore':
            this.loadAllExchanges(msg.sessionId, msg.jsonlPath);
            break;
        }
      });
    }

    this.currentSessionId = session.sessionId;
    this.panel.title = `Preview: ${displayName}`;

    const allExchanges = parseSessionExchanges(session.jsonlPath);
    const initial = allExchanges.slice(0, 50);
    const hasMore = allExchanges.length > 50;

    this.panel.webview.html = this.getHtml(session, displayName, initial, hasMore, allExchanges.length);
  }

  private loadAllExchanges(sessionId: string, jsonlPath: string): void {
    if (!this.panel || this.currentSessionId !== sessionId) return;
    const exchanges = parseSessionExchanges(jsonlPath);
    this.panel.webview.postMessage({ command: 'allExchanges', exchanges });
  }

  private getHtml(session: Session, displayName: string, exchanges: Exchange[], hasMore: boolean, totalCount: number): string {
    const cssHref = this.panel!.webview.asWebviewUri(this.cssUri);
    const nonce = getNonce();

    const exchangeHtml = exchanges.map((e, i) =>
      `<div class="exchange" data-index="${i}" data-line="${e.lineIndex}" onclick="selectExchange(this, ${i}, ${e.lineIndex})">` +
      `<div class="role ${escapeHtml(e.role)}">${e.role === 'user' ? 'YOU' : 'CLAUDE'} [${i + 1}]</div>` +
      `<div class="text">${escapeHtml(e.text)}</div>` +
      `</div>`
    ).join('');

    const loadMoreHtml = hasMore
      ? `<div class="load-more"><button onclick="loadMore()">Load all ${totalCount} exchanges</button></div>`
      : '';

    const sessionJson = JSON.stringify({ sessionId: session.sessionId, cwd: session.cwd });
    const jsonlPathJson = JSON.stringify(session.jsonlPath);

    return `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy"
    content="default-src 'none'; style-src ${this.panel!.webview.cspSource}; script-src 'nonce-${nonce}';">
  <link href="${cssHref}" rel="stylesheet">
</head>
<body>
  <div class="header">
    <h1>${escapeHtml(displayName)}</h1>
    <div class="meta">
      <span>${escapeHtml(session.cwd)}</span>
      <span>${escapeHtml(formatDate(session.startedAt))} &rarr; ${escapeHtml(formatDate(session.lastActivity))}</span>
      <span>${session.messageCount} messages</span>
      ${session.model ? `<span>${escapeHtml(session.model)}</span>` : ''}
    </div>
    <div class="actions">
      <button onclick="resume()">&#9654; Resume</button>
      <button class="secondary" onclick="fork()">&#9095; Fork</button>
    </div>
  </div>
  <div class="separator"></div>
  <div id="exchanges">${exchangeHtml}</div>
  ${loadMoreHtml}
  <div id="fork-bar" class="fork-bar" style="display:none;">
    <span id="fork-label"></span>
    <button onclick="forkFromHere()">Fork from here</button>
  </div>
  <script nonce="${nonce}">
    const vscode = acquireVsCodeApi();
    const sessionData = ${sessionJson};
    const jsonlPath = ${jsonlPathJson};
    let selectedLineIndex = -1;
    let selectedExchangeIndex = -1;
    let selectedRole = '';

    function resume() { vscode.postMessage({ command: 'resume', session: sessionData }); }
    function fork() { vscode.postMessage({ command: 'fork', session: sessionData }); }
    function loadMore() { vscode.postMessage({ command: 'loadMore', sessionId: sessionData.sessionId, jsonlPath }); }

    function selectExchange(el, index, lineIndex) {
      document.querySelectorAll('.exchange').forEach(e => e.classList.remove('selected'));
      el.classList.add('selected');
      selectedExchangeIndex = index;
      selectedLineIndex = lineIndex;
      selectedRole = el.querySelector('.role').textContent.startsWith('YOU') ? 'user' : 'assistant';
      const bar = document.getElementById('fork-bar');
      const label = document.getElementById('fork-label');
      if (bar && label) {
        label.textContent = 'Fork point: exchange ' + (index + 1);
        bar.style.display = 'flex';
      }
    }

    function forkFromHere() {
      if (selectedLineIndex < 0) return;
      vscode.postMessage({
        command: 'forkFromExchange',
        sessionId: sessionData.sessionId,
        cwd: sessionData.cwd,
        jsonlPath: jsonlPath,
        lineIndex: selectedLineIndex,
        role: selectedRole,
        exchangeIndex: selectedExchangeIndex
      });
    }

    window.addEventListener('message', (event) => {
      const msg = event.data;
      if (msg.command === 'allExchanges') {
        const container = document.getElementById('exchanges');
        if (!container) return;
        container.textContent = '';
        msg.exchanges.forEach((e, i) => {
          const div = document.createElement('div');
          div.className = 'exchange';
          div.dataset.index = String(i);
          div.dataset.line = String(e.lineIndex);
          div.onclick = function() { selectExchange(div, i, e.lineIndex); };
          const role = document.createElement('div');
          role.className = 'role ' + (e.role === 'user' ? 'user' : 'assistant');
          role.textContent = (e.role === 'user' ? 'YOU' : 'CLAUDE') + ' [' + (i + 1) + ']';
          const text = document.createElement('div');
          text.className = 'text';
          text.textContent = e.text;
          div.appendChild(role);
          div.appendChild(text);
          container.appendChild(div);
        });
        const loadMoreEl = document.querySelector('.load-more');
        if (loadMoreEl) loadMoreEl.remove();
      }
    });
  </script>
</body>
</html>`;
  }

  dispose(): void { this.panel?.dispose(); }
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
