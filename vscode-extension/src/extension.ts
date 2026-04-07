import * as vscode from 'vscode';
import { SessionStore } from './sessionStore';
import { SessionTreeProvider } from './treeProvider';
import { PreviewPanel } from './previewPanel';
import { registerCommands } from './commands';
import { ViewMode } from './types';

export function activate(context: vscode.ExtensionContext) {
  const store = new SessionStore();
  const treeProvider = new SessionTreeProvider(store);
  const previewPanel = new PreviewPanel(context.extensionUri);

  const config = vscode.workspace.getConfiguration('claudeman');
  const defaultView = config.get<ViewMode>('defaultView') || 'all';
  treeProvider.setViewMode(defaultView);

  const treeView = vscode.window.createTreeView('claudeman.sessions', {
    treeDataProvider: treeProvider,
    showCollapseAll: true,
  });
  context.subscriptions.push(treeView);

  registerCommands(context, store, treeProvider, previewPanel);

  store.refresh();
  store.startWatching();

  context.subscriptions.push({ dispose: () => { store.dispose(); previewPanel.dispose(); } });

  console.log(`ClaudeMan activated: ${store.getAllSessions().length} sessions found`);
}

export function deactivate() {}
