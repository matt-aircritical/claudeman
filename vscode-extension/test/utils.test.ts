import { describe, it } from 'node:test';
import assert from 'node:assert';
import { decodeMangledPath, formatDate, shortenPath, truncateText, projectShortName, isUuid } from '../src/utils';

describe('decodeMangledPath', () => {
  it('decodes standard paths', () => {
    assert.strictEqual(decodeMangledPath('-home-matt-projects'), '/home/matt/projects');
    assert.strictEqual(decodeMangledPath('-home-matt-projects-nanoclaw'), '/home/matt/projects/nanoclaw');
  });
  it('handles paths without leading dash', () => {
    assert.strictEqual(decodeMangledPath('home-matt'), 'home/matt');
  });
});

describe('formatDate', () => {
  it('formats recent timestamps as relative', () => {
    const oneHourAgo = Date.now() - 3600 * 1000;
    assert.match(formatDate(oneHourAgo), /1h ago/);
  });
  it('formats old timestamps as absolute', () => {
    const d = new Date(2026, 0, 15); // local timezone: Jan 15
    assert.match(formatDate(d.getTime()), /Jan 15/);
  });
});

describe('shortenPath', () => {
  it('returns short paths unchanged', () => {
    assert.strictEqual(shortenPath('/home/matt', 30), '/home/matt');
  });
  it('truncates long paths with ellipsis', () => {
    const result = shortenPath('/home/matt/very/long/deeply/nested/path', 20);
    assert.ok(result.startsWith('\u2026'));
    assert.ok(result.length <= 20);
  });
});

describe('truncateText', () => {
  it('returns short text unchanged', () => {
    assert.strictEqual(truncateText('hello', 10), 'hello');
  });
  it('truncates with ellipsis', () => {
    const result = truncateText('hello world this is long', 10);
    assert.ok(result.length <= 11); // 9 chars + ellipsis char
    assert.ok(result.endsWith('\u2026'));
  });
  it('handles multi-byte characters safely', () => {
    assert.doesNotThrow(() => truncateText('hello \u2014 world', 7));
  });
});

describe('projectShortName', () => {
  it('extracts last path component', () => {
    assert.strictEqual(projectShortName('/home/matt/projects/nanoclaw'), 'nanoclaw');
  });
});

describe('isUuid', () => {
  it('accepts valid UUIDs', () => {
    assert.ok(isUuid('aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee'));
  });
  it('rejects non-UUIDs', () => {
    assert.ok(!isUuid('not-a-uuid'));
  });
});
