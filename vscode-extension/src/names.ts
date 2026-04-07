import * as fs from 'fs';
import * as path from 'path';
import * as TOML from 'smol-toml';

const DEFAULT_PATH = path.join(
  process.env.XDG_CONFIG_HOME || path.join(process.env.HOME || '~', '.config'),
  'claudeman',
  'names.toml'
);

export class NameStore {
  private names: Map<string, string> = new Map();
  private filePath: string;

  constructor(filePath?: string) {
    this.filePath = filePath || DEFAULT_PATH;
    this.load();
  }

  load(): void {
    try {
      if (!fs.existsSync(this.filePath)) return;
      const content = fs.readFileSync(this.filePath, 'utf-8');
      const parsed = TOML.parse(content) as any;
      if (parsed.names && typeof parsed.names === 'object') {
        this.names = new Map(Object.entries(parsed.names));
      }
    } catch { /* ignore */ }
  }

  get(sessionId: string): string | undefined {
    return this.names.get(sessionId);
  }

  set(sessionId: string, name: string): void {
    this.names.set(sessionId, name);
    this.save();
  }

  delete(sessionId: string): void {
    this.names.delete(sessionId);
    this.save();
  }

  displayName(sessionId: string, fallback: string): string {
    return this.names.get(sessionId) || fallback;
  }

  private save(): void {
    try {
      const dir = path.dirname(this.filePath);
      if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
      const obj: Record<string, string> = {};
      for (const [k, v] of this.names) obj[k] = v;
      const content = TOML.stringify({ names: obj });
      fs.writeFileSync(this.filePath, content, 'utf-8');
    } catch { /* ignore */ }
  }
}
