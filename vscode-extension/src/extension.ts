import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

import { SessionStore } from './sessionStore';
import { SessionViewProvider } from './sessionViewProvider';
import { PreviewPanel } from './previewPanel';
import { registerCommands } from './commands';
import { ViewMode } from './types';

const RESUME_FILE = path.join(os.homedir(), '.claude', '.claudeman-resume');

export function activate(context: vscode.ExtensionContext) {
  const store = new SessionStore();
  const previewPanel = new PreviewPanel(context.extensionUri);
  const viewProvider = new SessionViewProvider(context.extensionUri, store, previewPanel);

  const config = vscode.workspace.getConfiguration('claudeman');
  const defaultView = config.get<ViewMode>('defaultView') || 'all';
  viewProvider.setViewMode(defaultView);

  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider('claudeman.sessions', viewProvider, {
      webviewOptions: { retainContextWhenHidden: true },
    }),
  );

  registerCommands(context, store, viewProvider, previewPanel);

  context.subscriptions.push(
    vscode.commands.registerCommand('claudeman.viewAll', () => viewProvider.setViewMode('all')),
    vscode.commands.registerCommand('claudeman.viewByProject', () => viewProvider.setViewMode('projects')),
    vscode.commands.registerCommand('claudeman.viewRecent', () => viewProvider.setViewMode('recent')),
    vscode.commands.registerCommand('claudeman.refreshSessions', () => {
      store.refresh();
      vscode.window.showInformationMessage('Sessions refreshed');
    }),
    vscode.commands.registerCommand('claudeman.searchSessions', async () => {
      const term = await vscode.window.showInputBox({ prompt: 'Search sessions', placeHolder: 'Enter search term...' });
      if (term !== undefined) viewProvider.setSearch(term);
    }),
    vscode.commands.registerCommand('claudeman.clearSearch', () => {
      viewProvider.clearSearch();
    }),
  );

  store.refresh();
  store.startWatching();

  // Check for resume request from another claudeman window
  checkResumeRequest();

  context.subscriptions.push({ dispose: () => { store.dispose(); previewPanel.dispose(); } });

  console.log(`ClaudeMan activated: ${store.getAllSessions().length} sessions found`);
}

function checkResumeRequest(): void {
  try {
    if (!fs.existsSync(RESUME_FILE)) return;
    const raw = fs.readFileSync(RESUME_FILE, 'utf-8');
    fs.unlinkSync(RESUME_FILE);
    const req = JSON.parse(raw);
    if (!req.sessionId) return;

    // Poll until Claude Code extension is active, then open the session directly
    let attempts = 0;
    const interval = setInterval(() => {
      attempts++;
      const claudeExt = vscode.extensions.getExtension('anthropic.claude-code');
      if (claudeExt?.isActive || attempts >= 30) {
        clearInterval(interval);
        if (claudeExt?.isActive) {
          setTimeout(() => {
            vscode.commands.executeCommand('claude-vscode.primaryEditor.open', req.sessionId);
          }, 2000);
        }
      }
    }, 1000);
  } catch { /* ignore */ }
}

export function deactivate() {}
