import { lookup } from 'node:dns/promises';
import { isIP } from 'node:net';

const ALLOWED_PROTOCOLS = new Set(['http:', 'https:']);
const FETCH_TIMEOUT_MS = 5000;

function isBlockedHostname(hostname: string): boolean {
  const lowered = hostname.toLowerCase();
  return lowered === 'localhost' || lowered.endsWith('.localhost');
}

function parseIPv4(address: string): number[] | null {
  const parts = address.split('.').map((part) => Number(part));
  if (parts.length !== 4 || parts.some((part) => Number.isNaN(part) || part < 0 || part > 255)) {
    return null;
  }
  return parts;
}

function isPrivateIPv4(address: string): boolean {
  const octets = parseIPv4(address);
  if (!octets) {
    return true;
  }

  const [a, b] = octets;
  if (a === 10) return true;
  if (a === 172 && b >= 16 && b <= 31) return true;
  if (a === 192 && b === 168) return true;
  if (a === 127) return true;
  if (a === 169 && b === 254) return true;
  if (a === 0) return true;
  if (a === 100 && b >= 64 && b <= 127) return true;
  if (a === 198 && (b === 18 || b === 19)) return true;
  if (a === 255 && b === 255) return true;
  return false;
}

function isPrivateIPv6(address: string): boolean {
  const lowered = address.toLowerCase();

  if (lowered === '::' || lowered === '::1') {
    return true;
  }

  if (lowered.startsWith('::ffff:')) {
    const mapped = lowered.slice('::ffff:'.length);
    return isPrivateIPv4(mapped);
  }

  const firstHextet = lowered.split(':').find((part) => part.length > 0) ?? '';

  if (firstHextet.startsWith('fc') || firstHextet.startsWith('fd')) {
    return true;
  }

  if (
    firstHextet.startsWith('fe8') ||
    firstHextet.startsWith('fe9') ||
    firstHextet.startsWith('fea') ||
    firstHextet.startsWith('feb')
  ) {
    return true;
  }

  return false;
}

function isBlockedAddress(address: string, family: number): boolean {
  if (family === 4) {
    return isPrivateIPv4(address);
  }

  if (family === 6) {
    return isPrivateIPv6(address);
  }

  return true;
}

export async function validateRemoteFileUrl(rawUrl: string): Promise<URL> {
  let parsed: URL;
  try {
    parsed = new URL(rawUrl);
  } catch (error) {
    throw new Error('Invalid file URL');
  }

  if (!ALLOWED_PROTOCOLS.has(parsed.protocol)) {
    throw new Error('Unsupported URL protocol');
  }

  if (!parsed.hostname) {
    throw new Error('URL must include a hostname');
  }

  if (isBlockedHostname(parsed.hostname)) {
    throw new Error('Hostname is not allowed');
  }

  const host = parsed.hostname;

  if (isIP(host)) {
    const family = isIP(host);
    if (isBlockedAddress(host, family)) {
      throw new Error('IP address is not allowed');
    }
    return parsed;
  }

  const records = await lookup(host, { all: true });
  if (records.length === 0) {
    throw new Error('Failed to resolve hostname');
  }

  for (const record of records) {
    if (isBlockedAddress(record.address, record.family)) {
      throw new Error('Resolved IP address is not allowed');
    }
  }

  return parsed;
}

export async function fetchRemoteFile(url: URL): Promise<Response> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS);

  try {
    const response = await fetch(url.toString(), { signal: controller.signal });
    return response;
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error('File request timed out');
    }
    throw error;
  } finally {
    clearTimeout(timeout);
  }
}

export const internal = {
  isBlockedAddress,
  isBlockedHostname,
  isPrivateIPv4,
  isPrivateIPv6,
  parseIPv4
};

export { FETCH_TIMEOUT_MS };
