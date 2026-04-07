import { describe, it } from 'node:test';
import assert from 'node:assert';
import path from 'node:path';
import { discoverSessions } from '../src/scanner';

const fixturesDir = path.join(import.meta.dirname || __dirname, 'fixtures');

describe('discoverSessions', () => {
  it('discovers JSONL session files', () => {
    const sessions = discoverSessions(fixturesDir);
    assert.strictEqual(sessions.length, 1);
    assert.strictEqual(sessions[0].sessionId, 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee');
    assert.strictEqual(sessions[0].projectDir, '/home/test/project');
    assert.ok(sessions[0].jsonlPath.endsWith('.jsonl'));
  });

  it('returns empty for nonexistent directory', () => {
    const sessions = discoverSessions('/nonexistent/path');
    assert.strictEqual(sessions.length, 0);
  });
});
