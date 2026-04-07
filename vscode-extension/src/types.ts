export interface DiscoveredSession {
  sessionId: string;
  projectDir: string;
  jsonlPath: string;
  fileMtime: number;
}

export interface Session {
  sessionId: string;
  projectDir: string;
  cwd: string;
  startedAt: number;
  lastActivity: number;
  name: string;
  model: string;
  version: string;
  messageCount: number;
  firstUserMessage: string;
  firstAssistantMessage: string;
  entrypoint: string;
  jsonlPath: string;
}

export interface Exchange {
  role: 'user' | 'assistant';
  text: string;
  timestamp?: number;
}

export type ViewMode = 'all' | 'projects' | 'recent';
