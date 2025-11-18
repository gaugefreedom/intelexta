/**
 * ZIP parsing utilities for extracting CAR bundles and attachments
 */

import JSZip from 'jszip';
import type { Car, AttachmentPreview } from '../types/car';
import { truncateText } from './textHelpers';

const MAX_TEXT_PREVIEW_LENGTH = 800;
const MAX_TEXT_FILE_SIZE = 5 * 1024 * 1024; // 5MB limit for text files

/**
 * Parse a CAR ZIP bundle and extract car.json + attachments
 */
export async function parseCarZip(
  file: File
): Promise<{ car: Car; attachments: AttachmentPreview[] }> {
  const zip = await JSZip.loadAsync(file);

  // 1. Extract car.json
  const carEntry = zip.file('car.json');
  if (!carEntry) {
    throw new Error('Missing car.json in ZIP archive');
  }

  const carText = await carEntry.async('string');
  const car: Car = JSON.parse(carText);

  // 2. Extract attachments from attachments/ directory
  const attachments: AttachmentPreview[] = [];
  const attachmentEntries: Array<{ fileName: string; entry: JSZip.JSZipObject }> = [];

  zip.forEach((path, entry) => {
    if (!path.startsWith('attachments/') || entry.dir) return;
    const fileName = path.replace(/^attachments\//, '');
    if (!fileName) return; // Skip the directory itself

    attachmentEntries.push({ fileName, entry });
  });

  // Process each attachment
  for (const { fileName, entry } of attachmentEntries) {
    const size = entry._data?.uncompressedSize ?? 0;

    // Determine if this is a text file we can preview
    const isTextFile =
      fileName.endsWith('.txt') ||
      fileName.endsWith('.md') ||
      fileName.endsWith('.json');

    if (isTextFile && size < MAX_TEXT_FILE_SIZE) {
      // Load text content
      try {
        const fullText = await entry.async('string');
        const preview = truncateText(fullText, MAX_TEXT_PREVIEW_LENGTH);

        attachments.push({
          fileName,
          size,
          kind: 'text',
          preview,
          fullText
        });
      } catch (err) {
        console.warn(`Failed to read text attachment ${fileName}:`, err);
        attachments.push({
          fileName,
          size,
          kind: 'binary'
        });
      }
    } else {
      attachments.push({
        fileName,
        size,
        kind: 'binary'
      });
    }
  }

  // 3. Match attachments to provenance claims by hash
  if (car.provenance) {
    for (const att of attachments) {
      // The fileName should be the hash (without .txt extension)
      const baseName = att.fileName.replace(/\.(txt|md|json)$/i, '');

      // Find matching provenance claim
      const match = car.provenance.find((claim) => {
        const claimHash = claim.sha256.replace(/^sha256:/, '');
        return claimHash === baseName;
      });

      if (match) {
        att.claimType = match.claim_type;
        att.hashHex = match.sha256.replace(/^sha256:/, '');
      }
    }
  }

  return { car, attachments };
}

/**
 * Extract only car.json from a ZIP without processing attachments
 * Useful for verification-only mode
 */
export async function extractCarJson(file: File): Promise<string> {
  const zip = await JSZip.loadAsync(file);
  const carEntry = zip.file('car.json');

  if (!carEntry) {
    throw new Error('Missing car.json in ZIP archive');
  }

  return await carEntry.async('string');
}

/**
 * Format file size in human-readable format
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}
