import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { Session } from './types';
import { discoverSessions } from './scanner';
import { parseSessionMetadata } from './parser';
import { NameStore } from './names';

export class SessionStore {
  private sessions: Map<string, Session> = new Map();
  private nameStore: NameStore;
  private watcher: vscode.FileSystemWatcher | undefined;
  private _onDidChange = new vscode.EventEmitter<void>();
  readonly onDidChange = this._onDidChange.event;

  constructor() {
    this.nameStore = new NameStore();
  }

  get names(): NameStore { return this.nameStore; }

  getClaudeDir(): string {
    const config = vscode.workspace.getConfiguration('claudeman');
    const custom = config.get<string>('sessionDirectory');
    if (custom) return custom;
    return path.join(os.homedir(), '.claude');
  }

  refresh(): void {
    const discovered = discoverSessions(this.getClaudeDir());
    const newSessions = new Map<string, Session>();
    for (const d of discovered) {
      const existing = this.sessions.get(d.sessionId);
      if (existing && existing.jsonlPath === d.jsonlPath) {
        newSessions.set(d.sessionId, existing);
      } else {
        try {
          newSessions.set(d.sessionId, parseSessionMetadata(d));
        } catch { /* skip */ }
      }
    }
    this.sessions = newSessions;
    this._onDidChange.fire();
  }

  startWatching(): void {
    const pattern = new vscode.RelativePattern(this.getClaudeDir(), 'projects/**/*.jsonl');
    this.watcher = vscode.workspace.createFileSystemWatcher(pattern);
    this.watcher.onDidCreate(() => this.refresh());
    this.watcher.onDidChange(() => this.refresh());
    this.watcher.onDidDelete(() => this.refresh());
  }

  stopWatching(): void { this.watcher?.dispose(); }

  getAllSessions(): Session[] {
    return [...this.sessions.values()].sort((a, b) => b.lastActivity - a.lastActivity);
  }

  getSessionsByProject(): Map<string, Session[]> {
    const groups = new Map<string, Session[]>();
    for (const s of this.sessions.values()) {
      const list = groups.get(s.projectDir) || [];
      list.push(s);
      groups.set(s.projectDir, list);
    }
    for (const list of groups.values()) list.sort((a, b) => b.lastActivity - a.lastActivity);
    return groups;
  }

  getSessionsByDate(): Map<string, Session[]> {
    const groups = new Map<string, Session[]>();
    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    const yesterday = today - 86400000;
    const thisWeek = today - 7 * 86400000;
    const thisMonth = today - 30 * 86400000;

    for (const s of this.sessions.values()) {
      let label: string;
      if (s.lastActivity >= today) label = 'Today';
      else if (s.lastActivity >= yesterday) label = 'Yesterday';
      else if (s.lastActivity >= thisWeek) label = 'This Week';
      else if (s.lastActivity >= thisMonth) label = 'This Month';
      else label = 'Older';
      const list = groups.get(label) || [];
      list.push(s);
      groups.set(label, list);
    }
    for (const list of groups.values()) list.sort((a, b) => b.lastActivity - a.lastActivity);
    return groups;
  }

  getSession(sessionId: string): Session | undefined { return this.sessions.get(sessionId); }

  deleteSession(sessionId: string): void {
    const session = this.sessions.get(sessionId);
    if (session?.jsonlPath) {
      try { fs.unlinkSync(session.jsonlPath); } catch { /* file may already be gone */ }
    }
    this.sessions.delete(sessionId);
    this._onDidChange.fire();
  }

  dispose(): void {
    this.stopWatching();
    this._onDidChange.dispose();
  }
}
