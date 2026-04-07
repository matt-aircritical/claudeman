import * as vscode from 'vscode';
import { ViewMode } from './types';
import { SessionStore } from './sessionStore';
import { SessionItem, GroupItem } from './treeItems';
import { projectShortName } from './utils';

export class SessionTreeProvider implements vscode.TreeDataProvider<vscode.TreeItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<vscode.TreeItem | undefined>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private viewMode: ViewMode = 'all';

  constructor(private store: SessionStore) {
    store.onDidChange(() => this._onDidChangeTreeData.fire(undefined));
  }

  setViewMode(mode: ViewMode): void {
    this.viewMode = mode;
    this._onDidChangeTreeData.fire(undefined);
  }

  getViewMode(): ViewMode { return this.viewMode; }

  refresh(): void { this._onDidChangeTreeData.fire(undefined); }

  getTreeItem(element: vscode.TreeItem): vscode.TreeItem { return element; }

  getChildren(element?: vscode.TreeItem): vscode.TreeItem[] {
    if (!element) return this.getRootItems();

    if (element instanceof GroupItem) {
      return element.sessions.map(
        s => new SessionItem(s, this.store.names.displayName(s.sessionId, s.name))
      );
    }

    return [];
  }

  private getRootItems(): vscode.TreeItem[] {
    switch (this.viewMode) {
      case 'all':
        return this.store.getAllSessions().map(
          s => new SessionItem(s, this.store.names.displayName(s.sessionId, s.name))
        );

      case 'projects': {
        const groups = this.store.getSessionsByProject();
        const sorted = [...groups.entries()].sort((a, b) => {
          const aMax = Math.max(...a[1].map(s => s.lastActivity));
          const bMax = Math.max(...b[1].map(s => s.lastActivity));
          return bMax - aMax;
        });
        return sorted.map(([dir, sessions]) =>
          new GroupItem(projectShortName(dir), sessions, 'project')
        );
      }

      case 'recent': {
        const groups = this.store.getSessionsByDate();
        const order = ['Today', 'Yesterday', 'This Week', 'This Month', 'Older'];
        const items: GroupItem[] = [];
        for (const label of order) {
          const sessions = groups.get(label);
          if (sessions?.length) items.push(new GroupItem(label, sessions, 'date'));
        }
        return items;
      }

      default: return [];
    }
  }
}
