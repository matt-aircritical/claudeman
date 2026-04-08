import * as fs from 'fs';
import { DiscoveredSession, Session, Exchange } from './types';

export function parseSessionMetadata(discovered: DiscoveredSession): Session {
  const lines = readLines(discovered.jsonlPath);

  let cwd = discovered.projectDir;
  let cwdSet = false;
  let startedAt = 0;
  let lastActivity = 0;
  let model = '';
  let version = '';
  let messageCount = 0;
  let firstUserMessage = '';
  let firstAssistantMessage = '';
  let entrypoint = '';

  for (const line of lines) {
    let data: any;
    try { data = JSON.parse(line); } catch { continue; }

    const type = data.type;
    if (type === 'user') {
      messageCount++;
      const ts = parseTimestamp(data.timestamp);
      if (ts) {
        if (!startedAt) startedAt = ts;
        lastActivity = ts;
      }
      if (data.cwd && !cwdSet) { cwd = data.cwd; cwdSet = true; }
      if (!version && data.version) version = data.version;
      if (!entrypoint && data.entrypoint) entrypoint = data.entrypoint;
      const text = extractMessageText(data);
      if (text && !firstUserMessage) firstUserMessage = text.slice(0, 500);
    } else if (type === 'assistant') {
      messageCount++;
      const ts = parseTimestamp(data.timestamp);
      if (ts) lastActivity = ts;
      if (!model && data.model) model = data.model;
      const text = extractMessageText(data);
      if (text && !firstAssistantMessage) firstAssistantMessage = text.slice(0, 500);
    }
  }

  const chars = [...firstUserMessage];
  const name = chars.length > 80 ? chars.slice(0, 80).join('') : firstUserMessage;

  return {
    sessionId: discovered.sessionId,
    projectDir: discovered.projectDir,
    cwd,
    startedAt: startedAt || Date.now(),
    lastActivity: lastActivity || Date.now(),
    name, model, version, messageCount,
    firstUserMessage, firstAssistantMessage,
    entrypoint: entrypoint || 'cli',
    jsonlPath: discovered.jsonlPath,
  };
}

export function parseSessionExchanges(jsonlPath: string): Exchange[] {
  const lines = readLines(jsonlPath);
  const exchanges: Exchange[] = [];
  for (let i = 0; i < lines.length; i++) {
    let data: any;
    try { data = JSON.parse(lines[i]); } catch { continue; }
    const type = data.type;
    if (type !== 'user' && type !== 'assistant') continue;
    const text = extractMessageText(data);
    if (!text) continue;
    const ts = parseTimestamp(data.timestamp);
    exchanges.push({ role: type as 'user' | 'assistant', text, timestamp: ts || undefined, lineIndex: i });
  }
  return exchanges;
}

function readLines(filePath: string): string[] {
  try { return fs.readFileSync(filePath, 'utf-8').split('\n').filter(Boolean); }
  catch { return []; }
}

function parseTimestamp(ts: any): number | null {
  if (typeof ts === 'string') {
    const d = new Date(ts);
    return isNaN(d.getTime()) ? null : d.getTime();
  }
  if (typeof ts === 'number') return ts;
  return null;
}

function extractMessageText(data: any): string {
  const content = data?.message?.content;
  if (!content) return '';
  if (typeof content === 'string') return content;
  if (Array.isArray(content)) {
    return content
      .filter((item: any) => typeof item === 'string' || (item?.type === 'text' && item?.text))
      .map((item: any) => typeof item === 'string' ? item : item.text)
      .join(' ');
  }
  return '';
}
