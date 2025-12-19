import { createReadStream, mkdirSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { Readable } from 'node:stream';

export type EvictionReason = 'ttl' | 'capacity';

export interface BundleStorageOptions {
  maxEntries: number;
  maxTotalBytes: number;
  ttlMs: number;
  directory: string;
}

interface BundleEntry {
  id: string;
  size: number;
  createdAt: number;
  expiresAt: number;
  path: string;
}

export interface BundleStream {
  stream: Readable;
  size: number;
  createdAt: number;
}

export interface StorageStats {
  totalEntries: number;
  totalBytes: number;
  directory: string;
}

export class LimitedBundleStorage {
  private readonly entries = new Map<string, BundleEntry>();
  private totalBytes = 0;
  private readonly directory: string;

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

    if (!options.directory) {
      throw new Error('directory is required for persistent bundle storage.');
    }

    this.directory = resolve(options.directory);
    mkdirSync(this.directory, { recursive: true });
    this.bootstrapFromDisk();
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
      this.evict(id, 'capacity');
    }

    while (
      (this.entries.size >= this.options.maxEntries || this.totalBytes + size > this.options.maxTotalBytes) &&
      this.entries.size > 0
    ) {
      const oldestKey = this.getOldestEntryId();
      if (!oldestKey) {
        break;
      }
      this.evict(oldestKey, 'capacity');
    }

    const filePath = this.buildPath(id);
    writeFileSync(filePath, buffer);

    const entry: BundleEntry = {
      id,
      path: filePath,
      size,
      createdAt: now,
      expiresAt: now + this.options.ttlMs
    };

    this.entries.set(id, entry);
    this.totalBytes += size;
    console.log(
      `Stored bundle ${id} (${size} bytes) at ${filePath}. ${this.entries.size} entries, ${this.totalBytes} bytes currently held.`
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

    try {
      return {
        stream: createReadStream(entry.path),
        size: entry.size,
        createdAt: entry.createdAt
      };
    } catch {
      // If the file was removed externally, evict it.
      this.evict(id, 'capacity');
      return undefined;
    }
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
      totalBytes: this.totalBytes,
      directory: this.directory
    };
  }

  private evict(id: string, reason: EvictionReason): void {
    const entry = this.entries.get(id);
    if (!entry) {
      return;
    }

    this.entries.delete(id);
    this.totalBytes -= entry.size;

    try {
      rmSync(entry.path, { force: true });
    } catch (error) {
      console.warn(`Failed to remove bundle ${id} from disk:`, error);
    }

    console.log(`Evicted bundle ${id} due to ${reason}.`);
  }

  private bootstrapFromDisk(now: number = Date.now()): void {
    try {
      const files = readdirSync(this.directory);
      for (const file of files) {
        if (!file.endsWith('.zip')) {
          continue;
        }
        const id = file.slice(0, -4);
        const path = this.buildPath(id);
        const stats = statSync(path);
        const createdAt = stats.birthtimeMs || stats.mtimeMs;
        const expiresAt = createdAt + this.options.ttlMs;

        if (expiresAt <= now) {
          this.evict(id, 'ttl');
          continue;
        }

        const entry: BundleEntry = {
          id,
          path,
          size: stats.size,
          createdAt,
          expiresAt
        };
        this.entries.set(id, entry);
        this.totalBytes += stats.size;
      }

      while (
        (this.entries.size > this.options.maxEntries || this.totalBytes > this.options.maxTotalBytes) &&
        this.entries.size > 0
      ) {
        const oldestKey = this.getOldestEntryId();
        if (!oldestKey) {
          break;
        }
        this.evict(oldestKey, 'capacity');
      }

      console.log(
        `Bootstrapped bundle storage from disk. ${this.entries.size} entries, ${this.totalBytes} bytes retained.`
      );
    } catch (error) {
      console.warn('Failed to bootstrap bundle storage from disk:', error);
    }
  }

  private getOldestEntryId(): string | undefined {
    const next = this.entries.keys().next();
    return next.done ? undefined : next.value;
  }

  private buildPath(id: string): string {
    return join(this.directory, `${id}.zip`);
  }
}
