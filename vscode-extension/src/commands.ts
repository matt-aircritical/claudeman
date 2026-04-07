import * as vscode from 'vscode';
import * as fs from 'fs';
import { SessionStore } from './sessionStore';
import { SessionTreeProvider } from './treeProvider';
import { PreviewPanel } from './previewPanel';
import { SessionItem } from './treeItems';

export function registerCommands(
  context: vscode.ExtensionContext,
  store: SessionStore,
  treeProvider: SessionTreeProvider,
  previewPanel: PreviewPanel,
): void {
  context.subscriptions.push(
    vscode.commands.registerCommand('claudeman.resumeSession', (item?: SessionItem | { session: { sessionId: string; cwd: string } }) => {
      const session = resolveSession(item);
      if (session) launchClaude(session.sessionId, session.cwd, false);
    }),

    vscode.commands.registerCommand('claudeman.forkSession', (item?: SessionItem | { session: { sessionId: string; cwd: string } }) => {
      const session = resolveSession(item);
      if (session) launchClaude(session.sessionId, session.cwd, true);
    }),

    vscode.commands.registerCommand('claudeman.renameSession', async (item?: SessionItem) => {
      if (!item?.session) return;
      const currentName = store.names.displayName(item.session.sessionId, item.session.name);
      const newName = await vscode.window.showInputBox({ prompt: 'Enter new session name', value: currentName });
      if (newName?.trim()) {
        store.names.set(item.session.sessionId, newName.trim());
        treeProvider.refresh();
        vscode.window.showInformationMessage(`Renamed to: ${newName.trim()}`);
      }
    }),

    vscode.commands.registerCommand('claudeman.previewSession', (item?: SessionItem) => {
      if (!item?.session) return;
      const name = store.names.displayName(item.session.sessionId, item.session.name);
      previewPanel.show(item.session, name);
    }),

    vscode.commands.registerCommand('claudeman.copySessionId', (item?: SessionItem) => {
      if (!item?.session) return;
      vscode.env.clipboard.writeText(item.session.sessionId);
      vscode.window.showInformationMessage(`Copied: ${item.session.sessionId}`);
    }),

    vscode.commands.registerCommand('claudeman.deleteSession', async (item?: SessionItem) => {
      if (!item?.session) return;
      const name = store.names.displayName(item.session.sessionId, item.session.name);
      const confirm = await vscode.window.showWarningMessage(
        `Delete "${name}" from index?`, 'Delete', 'Cancel'
      );
      if (confirm === 'Delete') {
        store.deleteSession(item.session.sessionId);
        vscode.window.showInformationMessage('Session removed from index');
      }
    }),

    vscode.commands.registerCommand('claudeman.refreshSessions', () => {
      store.refresh();
      vscode.window.showInformationMessage('Sessions refreshed');
    }),

    vscode.commands.registerCommand('claudeman.viewAll', () => treeProvider.setViewMode('all')),
    vscode.commands.registerCommand('claudeman.viewByProject', () => treeProvider.setViewMode('projects')),
    vscode.commands.registerCommand('claudeman.viewRecent', () => treeProvider.setViewMode('recent')),
  );
}

function resolveSession(item: any): { sessionId: string; cwd: string } | undefined {
  if (!item) return undefined;
  if (item instanceof SessionItem) return { sessionId: item.session.sessionId, cwd: item.session.cwd };
  if (item?.session) return item.session;
  return undefined;
}

async function launchClaude(sessionId: string, cwd: string, fork: boolean): Promise<void> {
  // Try to open in the Claude Code VSCode extension first
  const claudeExtension = vscode.extensions.getExtension('anthropic.claude-code');
  if (claudeExtension && !fork) {
    try {
      await vscode.commands.executeCommand('claude-vscode.editor.open', sessionId);
      return;
    } catch {
      // Fall through to terminal if the command fails
    }
  }

  // Fallback: open in integrated terminal (also used for fork, which needs CLI flags)
  const config = vscode.workspace.getConfiguration('claudeman');
  const claudeCmd = config.get<string>('claudeCommand') || 'claude';
  const extraArgs = config.get<string[]>('claudeArgs') || [];

  const args = ['--resume', sessionId];
  if (fork) args.push('--fork-session');
  args.push(...extraArgs);

  const terminalCwd = fs.existsSync(cwd) ? cwd : undefined;

  const terminal = vscode.window.createTerminal({
    name: `Claude: ${sessionId.slice(0, 8)}`,
    cwd: terminalCwd,
  });
  terminal.show();
  terminal.sendText(`${claudeCmd} ${args.join(' ')}`);
}
