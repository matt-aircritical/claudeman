import * as vscode from 'vscode';
import { Session, ViewMode } from './types';
import { SessionStore } from './sessionStore';
import { SessionItem, GroupItem } from './treeItems';
import { projectShortName } from './utils';

export class SessionTreeProvider implements vscode.TreeDataProvider<vscode.TreeItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<vscode.TreeItem | undefined>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private viewMode: ViewMode = 'all';
  private searchFilter = '';

  constructor(private store: SessionStore) {
    store.onDidChange(() => this._onDidChangeTreeData.fire(undefined));
  }

  setViewMode(mode: ViewMode): void {
    this.viewMode = mode;
    this._onDidChangeTreeData.fire(undefined);
  }

  getViewMode(): ViewMode { return this.viewMode; }

  setSearch(term: string): void {
    this.searchFilter = term.toLowerCase();
    vscode.commands.executeCommand('setContext', 'claudeman.searchActive', !!this.searchFilter);
    this._onDidChangeTreeData.fire(undefined);
  }

  clearSearch(): void { this.setSearch(''); }

  refresh(): void { this._onDidChangeTreeData.fire(undefined); }

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
    const filter = (list: Session[]) => list.filter(s => this.matchesSearch(s));

    switch (this.viewMode) {
      case 'all':
        return filter(this.store.getAllSessions()).map(
          s => new SessionItem(s, this.store.names.displayName(s.sessionId, s.name))
        );

      case 'projects': {
        const groups = this.store.getSessionsByProject();
        const sorted = [...groups.entries()]
          .map(([dir, sessions]) => [dir, filter(sessions)] as [string, Session[]])
          .filter(([, sessions]) => sessions.length > 0)
          .sort((a, b) => {
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
          const sessions = filter(groups.get(label) || []);
          if (sessions.length) items.push(new GroupItem(label, sessions, 'date'));
        }
        return items;
      }

      default: return [];
    }
  }
}
