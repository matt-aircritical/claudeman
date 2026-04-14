import * as vscode from 'vscode';
import * as crypto from 'crypto';
import * as fs from 'fs';
import * as path from 'path';
import { SessionStore } from './sessionStore';
import { SessionViewProvider } from './sessionViewProvider';
import { PreviewPanel } from './previewPanel';

export function registerCommands(
  context: vscode.ExtensionContext,
  store: SessionStore,
  viewProvider: SessionViewProvider,
  previewPanel: PreviewPanel,
): void {
  context.subscriptions.push(
    vscode.commands.registerCommand('claudeman.resumeSession', async (item?: any) => {
      const session = item?.session;
      if (session) await openInVscode(session.sessionId, session.cwd);
    }),

    vscode.commands.registerCommand('claudeman.forkSession', async (item?: any) => {
      const session = item?.session;
      if (!session) return;
      const jsonlPath = session.jsonlPath;
      if (jsonlPath) {
        const newId = forkFullSession(jsonlPath);
        if (newId) await openInVscode(newId, session.cwd);
      } else {
        await openInVscode(session.sessionId, session.cwd);
      }
    }),

    vscode.commands.registerCommand('claudeman.forkFromExchange', (msg: any) => {
      if (!msg?.jsonlPath || msg.lineIndex == null) return;
      forkFromExchange(msg.sessionId, msg.cwd, msg.jsonlPath, msg.lineIndex, msg.role);
    }),
  );
}

/**
 * Open a new VSCode window in the session's directory.
 * Claude Code's extension will show the session in its session list.
 */
async function openInVscode(sessionId: string, cwd: string): Promise<void> {
  const targetDir = fs.existsSync(cwd) ? cwd : undefined;
  if (!targetDir) {
    vscode.window.showErrorMessage(`Session directory not found: ${cwd}`);
    return;
  }

  // Write resume request for the new window's claudeman to pick up
  const resumeFile = path.join(require('os').homedir(), '.claude', '.claudeman-resume');
  fs.writeFileSync(resumeFile, JSON.stringify({ sessionId }));

  // Open the session's directory in a new window — Claude Code activates automatically
  await vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(targetDir), true);
}

/**
 * Fork entire session: copy all JSONL lines with a new session ID.
 */
function forkFullSession(jsonlPath: string): string | undefined {
  try {
    const content = fs.readFileSync(jsonlPath, 'utf-8');
    const lines = content.split('\n').filter(l => l.trim());
    const newId = crypto.randomUUID();
    const forkedLines = lines.map(line => {
      try {
        const data = JSON.parse(line);
        data.sessionId = newId;
        return JSON.stringify(data);
      } catch { return line; }
    });
    const forkPath = path.join(path.dirname(jsonlPath), `${newId}.jsonl`);
    fs.writeFileSync(forkPath, forkedLines.join('\n') + '\n');
    vscode.window.showInformationMessage(`Forked → new session ${newId.slice(0, 8)}`);
    return newId;
  } catch (e: any) {
    vscode.window.showErrorMessage(`Fork failed: ${e.message}`);
    return undefined;
  }
}

function forkFromExchange(sessionId: string, cwd: string, jsonlPath: string, lineIndex: number, role: string): void {
  try {
    const content = fs.readFileSync(jsonlPath, 'utf-8');
    const lines = content.split('\n').filter(l => l.trim());

    // If user message selected, include the following assistant response
    let lastLine = lineIndex;
    if (role === 'user' && lastLine + 1 < lines.length) {
      try {
        const next = JSON.parse(lines[lastLine + 1]);
        if (next.type === 'assistant') lastLine = lastLine + 1;
      } catch { /* skip */ }
    }

    if (lastLine >= lines.length) {
      vscode.window.showErrorMessage('Exchange index out of range');
      return;
    }

    // Generate new session ID and write truncated JSONL
    const newId = crypto.randomUUID();
    const forkedLines = lines.slice(0, lastLine + 1).map(line => {
      try {
        const data = JSON.parse(line);
        data.sessionId = newId;
        return JSON.stringify(data);
      } catch { return line; }
    });

    const forkPath = path.join(path.dirname(jsonlPath), `${newId}.jsonl`);
    fs.writeFileSync(forkPath, forkedLines.join('\n') + '\n');

    const exNum = Math.floor((lineIndex + 1) / 2) + 1;
    vscode.window.showInformationMessage(`Forked from exchange ${exNum} → new session ${newId.slice(0, 8)}`);
    openInVscode(newId, cwd);
  } catch (e: any) {
    vscode.window.showErrorMessage(`Fork failed: ${e.message}`);
  }
}
