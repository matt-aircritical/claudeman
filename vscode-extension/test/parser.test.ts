import { describe, it } from 'node:test';
import assert from 'node:assert';
import path from 'node:path';
import { parseSessionMetadata, parseSessionExchanges } from '../src/parser';

const fixturePath = path.join(
  import.meta.dirname || __dirname,
  'fixtures/projects/-home-test-project/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl'
);

describe('parseSessionMetadata', () => {
  it('extracts session metadata from JSONL', () => {
    const session = parseSessionMetadata({
      sessionId: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
      projectDir: '/home/test/project',
      jsonlPath: fixturePath,
      fileMtime: 1000,
    });
    assert.strictEqual(session.sessionId, 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee');
    assert.strictEqual(session.cwd, '/home/test/project');
    assert.strictEqual(session.name, 'help me fix the login bug');
    assert.strictEqual(session.version, '2.1.71');
    assert.strictEqual(session.messageCount, 4);
    assert.ok(session.firstUserMessage.includes('login bug'));
    assert.ok(session.firstAssistantMessage.includes('take a look'));
  });
});

describe('parseSessionExchanges', () => {
  it('returns all user/assistant exchanges', () => {
    const exchanges = parseSessionExchanges(fixturePath);
    assert.strictEqual(exchanges.length, 4);
    assert.strictEqual(exchanges[0].role, 'user');
    assert.ok(exchanges[0].text.includes('login bug'));
    assert.strictEqual(exchanges[1].role, 'assistant');
    assert.ok(exchanges[1].text.includes('take a look'));
  });
});
