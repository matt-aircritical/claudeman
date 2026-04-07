export function decodeMangledPath(mangled: string): string {
  if (mangled.startsWith('-')) {
    return '/' + mangled.slice(1).replace(/-/g, '/');
  }
  return mangled.replace(/-/g, '/');
}

export function formatDate(timestampMs: number): string {
  const diffMs = Date.now() - timestampMs;
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);
  if (diffHours < 24) return `${Math.max(0, diffHours)}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  const d = new Date(timestampMs);
  const months = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
  return `${months[d.getMonth()]} ${d.getDate()}`;
}

export function shortenPath(path: string, maxLen: number): string {
  if (path.length <= maxLen) return path;
  const keep = maxLen - 1;
  if (keep <= 0) return '\u2026';
  return '\u2026' + path.slice(path.length - keep);
}

export function truncateText(text: string, maxLen: number): string {
  const chars = [...text];
  if (chars.length <= maxLen) return text;
  return chars.slice(0, maxLen - 1).join('') + '\u2026';
}

export function projectShortName(projectDir: string): string {
  const parts = projectDir.split('/').filter(Boolean);
  return parts[parts.length - 1] || projectDir;
}

export function isUuid(s: string): boolean {
  return s.length === 36 && (s.match(/-/g) || []).length === 4;
}
