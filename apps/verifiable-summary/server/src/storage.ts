import { Readable } from 'node:stream';

export type EvictionReason = 'ttl' | 'capacity';

export interface BundleStorageOptions {
  maxEntries: number;
  maxTotalBytes: number;
  ttlMs: number;
}

interface BundleEntry {
  buffer: Buffer;
  size: number;
  createdAt: number;
  expiresAt: number;
}

export interface BundleStream {
  stream: Readable;
  size: number;
  createdAt: number;
}

export interface StorageStats {
  totalEntries: number;
  totalBytes: number;
}

export class LimitedBundleStorage {
  private readonly entries = new Map<string, BundleEntry>();
  private totalBytes = 0;

  constructor(private readonly options: BundleStorageOptions) {
    if (!Number.isFinite(options.maxEntries) || options.maxEntries <= 0) {
      throw new Error('maxEntries must be a positive number.');
    }

    if (!Number.isFinite(options.maxTotalBytes) || options.maxTotalBytes <= 0) {
      throw new Error('maxTotalBytes must be a positive number.');
    }

    if (!Number.isFinite(options.ttlMs) || options.ttlMs <= 0) {
      throw new Error('ttlMs must be a positive number.');
    }
  }

  store(id: string, buffer: Buffer, now: number = Date.now()): void {
    const size = buffer.byteLength;
    if (size > this.options.maxTotalBytes) {
      throw new Error(
        `Bundle size (${size} bytes) exceeds maximum capacity (${this.options.maxTotalBytes} bytes).`
      );
    }

    this.cleanupExpired(now);

    const existing = this.entries.get(id);
    if (existing) {
      this.entries.delete(id);
      this.totalBytes -= existing.size;
    }

    while (
      (this.entries.size >= this.options.maxEntries || this.totalBytes + size > this.options.maxTotalBytes) &&
      this.entries.size > 0
    ) {
      const oldestKey = this.entries.keys().next().value as string | undefined;
      if (!oldestKey) {
        break;
      }
      this.evict(oldestKey, 'capacity');
    }

    const entry: BundleEntry = {
      buffer,
      size,
      createdAt: now,
      expiresAt: now + this.options.ttlMs
    };

    this.entries.set(id, entry);
    this.totalBytes += size;
    console.log(
      `Stored bundle ${id} (${size} bytes). ${this.entries.size} entries, ${this.totalBytes} bytes currently held.`
    );
  }

  getStream(id: string, now: number = Date.now()): BundleStream | undefined {
    const entry = this.entries.get(id);
    if (!entry) {
      return undefined;
    }

    if (entry.expiresAt <= now) {
      this.evict(id, 'ttl');
      return undefined;
    }

    // Update recency by reinserting the entry at the end of the Map
    this.entries.delete(id);
    this.entries.set(id, entry);

    return {
      stream: Readable.from(entry.buffer),
      size: entry.size,
      createdAt: entry.createdAt
    };
  }

  has(id: string, now: number = Date.now()): boolean {
    const entry = this.entries.get(id);
    if (!entry) {
      return false;
    }

    if (entry.expiresAt <= now) {
      this.evict(id, 'ttl');
      return false;
    }

    return true;
  }

  cleanupExpired(now: number = Date.now()): number {
    let removed = 0;
    for (const [id, entry] of this.entries) {
      if (entry.expiresAt <= now) {
        this.evict(id, 'ttl');
        removed += 1;
      }
    }
    return removed;
  }

  getStats(): StorageStats {
    return {
      totalEntries: this.entries.size,
      totalBytes: this.totalBytes
    };
  }

  private evict(id: string, reason: EvictionReason): void {
    const entry = this.entries.get(id);
    if (!entry) {
      return;
    }

    this.entries.delete(id);
    this.totalBytes -= entry.size;
    console.log(`Evicted bundle ${id} due to ${reason}.`);
  }
}
