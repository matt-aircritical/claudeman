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
      if (session) resumeInTerminal(session.sessionId, session.cwd, false);
    }),

    vscode.commands.registerCommand('claudeman.resumeInWindow', async (item?: SessionItem) => {
      const session = item?.session;
      if (!session) { return; }
      await resumeInNewWindow(session.sessionId, session.cwd);
    }),

    vscode.commands.registerCommand('claudeman.forkSession', (item?: SessionItem | { session: { sessionId: string; cwd: string } }) => {
      const session = resolveSession(item);
      if (session) resumeInTerminal(session.sessionId, session.cwd, true);
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

/**
 * Resume in a new VSCode window opened in the session's directory.
 * Claude Code's session list will have the session available there.
 */
async function resumeInNewWindow(sessionId: string, cwd: string): Promise<void> {
  const targetDir = fs.existsSync(cwd) ? cwd : undefined;
  if (!targetDir) {
    vscode.window.showErrorMessage(`Session directory not found: ${cwd}`);
    return;
  }

  const uri = vscode.Uri.file(targetDir);
  // Open in a new window (forceNewWindow = true)
  await vscode.commands.executeCommand('vscode.openFolder', uri, true);
  vscode.window.showInformationMessage(
    `Opening ${targetDir} — find session ${sessionId.slice(0, 8)} in Claude Code's session list`
  );
}

/**
 * Resume in the integrated terminal with cd to the session's directory.
 */
function resumeInTerminal(sessionId: string, cwd: string, fork: boolean): void {
  const config = vscode.workspace.getConfiguration('claudeman');
  const claudeCmd = config.get<string>('claudeCommand') || 'claude';
  const extraArgs = config.get<string[]>('claudeArgs') || [];

  const args = ['--resume', sessionId];
  if (fork) args.push('--fork-session');
  args.push(...extraArgs);

  const targetDir = fs.existsSync(cwd) ? cwd : undefined;

  const terminal = vscode.window.createTerminal({
    name: `Claude: ${sessionId.slice(0, 8)}`,
    cwd: targetDir,
  });
  terminal.show();
  if (targetDir) {
    terminal.sendText(`cd ${targetDir} && ${claudeCmd} ${args.join(' ')}`);
  } else {
    terminal.sendText(`${claudeCmd} ${args.join(' ')}`);
  }
}
