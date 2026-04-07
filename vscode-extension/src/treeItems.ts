import * as vscode from 'vscode';
import { Session } from './types';
import { formatDate, projectShortName } from './utils';

export class SessionItem extends vscode.TreeItem {
  constructor(
    public readonly session: Session,
    public readonly displayName: string,
  ) {
    super(displayName, vscode.TreeItemCollapsibleState.None);

    const project = projectShortName(session.projectDir);
    const date = formatDate(session.lastActivity);
    this.description = `${project} \u00b7 ${date} \u00b7 ${session.messageCount} msgs`;

    this.tooltip = new vscode.MarkdownString(
      `**${displayName}**\n\n` +
      `ID: \`${session.sessionId}\`\n\n` +
      `Dir: \`${session.cwd}\`\n\n` +
      `Model: ${session.model || 'unknown'}\n\n` +
      `Messages: ${session.messageCount}\n\n` +
      `Started: ${new Date(session.startedAt).toLocaleString()}`
    );

    this.iconPath = new vscode.ThemeIcon(
      session.entrypoint === 'ide' ? 'comment-discussion' : 'terminal'
    );

    this.contextValue = 'session';

    this.command = {
      command: 'claudeman.previewSession',
      title: 'Preview',
      arguments: [this],
    };
  }
}

export class GroupItem extends vscode.TreeItem {
  constructor(
    public readonly groupLabel: string,
    public readonly sessions: Session[],
    public readonly groupType: 'project' | 'date',
  ) {
    super(groupLabel, vscode.TreeItemCollapsibleState.Expanded);
    this.description = `(${sessions.length})`;
    this.iconPath = new vscode.ThemeIcon(groupType === 'project' ? 'folder' : 'calendar');
    this.contextValue = 'group';
  }
}
