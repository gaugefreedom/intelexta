import { describe, expect, it } from 'vitest';
import { Readable } from 'node:stream';

import { LimitedBundleStorage } from './storage.js';

async function readStream(stream: Readable): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of stream) {
    chunks.push(typeof chunk === 'string' ? Buffer.from(chunk) : chunk);
  }
  return Buffer.concat(chunks).toString('utf-8');
}

describe('LimitedBundleStorage', () => {
  it('evicts the least recently used bundle when exceeding max entries', async () => {
    const storage = new LimitedBundleStorage({
      maxEntries: 2,
      maxTotalBytes: 1024,
      ttlMs: 60_000
    });

    storage.store('a', Buffer.from('a'));
    storage.store('b', Buffer.from('b'));

    // Touch "a" so it becomes the most recently used entry
    storage.getStream('a')?.stream.resume();

    storage.store('c', Buffer.from('c'));

    expect(storage.getStream('b')).toBeUndefined();

    const bundleA = storage.getStream('a');
    const bundleC = storage.getStream('c');

    expect(bundleA).toBeDefined();
    expect(bundleC).toBeDefined();

    if (bundleA && bundleC) {
      await Promise.all([
        readStream(bundleA.stream).then((content) => expect(content).toBe('a')),
        readStream(bundleC.stream).then((content) => expect(content).toBe('c'))
      ]);
    }
  });

  it('evicts bundles to maintain the maximum total byte capacity', () => {
    const storage = new LimitedBundleStorage({
      maxEntries: 10,
      maxTotalBytes: 6,
      ttlMs: 60_000
    });

    storage.store('a', Buffer.from('aaa'));
    storage.store('b', Buffer.from('bb'));
    storage.store('c', Buffer.from('cccc'));

    expect(storage.has('a')).toBe(false);
    expect(storage.has('b')).toBe(true);
    expect(storage.has('c')).toBe(true);

    const stats = storage.getStats();
    expect(stats.totalEntries).toBe(2);
    expect(stats.totalBytes).toBe(6);
  });

  it('expires bundles after the configured TTL', () => {
    const storage = new LimitedBundleStorage({
      maxEntries: 2,
      maxTotalBytes: 1024,
      ttlMs: 1_000
    });

    storage.store('expiring', Buffer.from('data'), 0);

    expect(storage.has('expiring', 500)).toBe(true);
    storage.cleanupExpired(1_001);
    expect(storage.has('expiring', 1_001)).toBe(false);
    expect(storage.getStats().totalEntries).toBe(0);
  });

  it('throws when attempting to store a bundle larger than total capacity', () => {
    const storage = new LimitedBundleStorage({
      maxEntries: 2,
      maxTotalBytes: 4,
      ttlMs: 60_000
    });

    expect(() => storage.store('too-big', Buffer.from('12345'))).toThrow(
      /exceeds maximum capacity/
    );
  });
});
