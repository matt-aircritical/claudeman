import * as fs from 'fs';
import * as path from 'path';
import { DiscoveredSession } from './types';
import { decodeMangledPath, isUuid } from './utils';

export function discoverSessions(claudeDir: string): DiscoveredSession[] {
  const projectsDir = path.join(claudeDir, 'projects');
  if (!fs.existsSync(projectsDir)) {
    return [];
  }

  const sessions: DiscoveredSession[] = [];

  let projectEntries: fs.Dirent[];
  try {
    projectEntries = fs.readdirSync(projectsDir, { withFileTypes: true });
  } catch {
    return sessions;
  }

  for (const projectEntry of projectEntries) {
    if (!projectEntry.isDirectory()) continue;

    const projectPath = path.join(projectsDir, projectEntry.name);
    const projectDir = decodeMangledPath(projectEntry.name);

    let fileEntries: fs.Dirent[];
    try {
      fileEntries = fs.readdirSync(projectPath, { withFileTypes: true });
    } catch {
      continue;
    }

    for (const entry of fileEntries) {
      if (entry.isDirectory()) continue;
      if (!entry.name.endsWith('.jsonl')) continue;

      const sessionId = entry.name.replace('.jsonl', '');
      if (!isUuid(sessionId)) continue;

      const fullPath = path.join(projectPath, entry.name);
      let mtime = 0;
      try {
        const stat = fs.statSync(fullPath);
        mtime = stat.mtimeMs;
      } catch { /* ignore */ }

      sessions.push({ sessionId, projectDir, jsonlPath: fullPath, fileMtime: mtime });
    }
  }

  return sessions;
}
