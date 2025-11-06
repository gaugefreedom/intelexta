/**
 * Security utilities for SSRF protection and URL validation
 */

import { URL } from 'node:url';
import { lookup } from 'node:dns/promises';

// Private IP ranges that should be blocked
const BLOCKED_IP_RANGES = [
  /^127\./,                    // Loopback
  /^10\./,                     // Private network
  /^172\.(1[6-9]|2[0-9]|3[0-1])\./, // Private network
  /^192\.168\./,               // Private network
  /^169\.254\./,               // Link-local (metadata)
  /^0\./,                      // Non-routable
  /^224\./,                    // Multicast
  /^255\.255\.255\.255$/,      // Broadcast
  /^::1$/,                     // IPv6 loopback
  /^fe80:/i,                   // IPv6 link-local
  /^fc00:/i,                   // IPv6 unique local
  /^fd00:/i,                   // IPv6 unique local
];

/**
 * Validates if a URL is safe to fetch (SSRF protection)
 * @param urlString The URL to validate
 * @throws Error if URL is unsafe
 */
export async function validateSafeUrl(urlString: string): Promise<void> {
  let parsedUrl: URL;

  try {
    parsedUrl = new URL(urlString);
  } catch (error) {
    throw new Error('Invalid URL format');
  }

  // 1. Protocol validation - only allow http and https
  if (parsedUrl.protocol !== 'http:' && parsedUrl.protocol !== 'https:') {
    throw new Error(`Protocol "${parsedUrl.protocol}" is not allowed. Only http: and https: are permitted.`);
  }

  // 2. Resolve hostname to IP address
  const hostname = parsedUrl.hostname;
  let resolvedIps: string[];

  try {
    // Use DNS lookup to resolve hostname to IP
    const addresses = await lookup(hostname, { all: true });
    resolvedIps = addresses.map(addr => addr.address);
  } catch (error) {
    throw new Error(`Failed to resolve hostname: ${hostname}`);
  }

  // 3. Check all resolved IPs against blocked ranges
  for (const ip of resolvedIps) {
    for (const blockedRange of BLOCKED_IP_RANGES) {
      if (blockedRange.test(ip)) {
        throw new Error(`Access to private/internal IP address (${ip}) is not allowed`);
      }
    }

    // Additional check for localhost
    if (hostname === 'localhost' || ip === '127.0.0.1' || ip === '::1') {
      throw new Error('Access to localhost is not allowed');
    }
  }

  // 4. Check for common cloud metadata endpoints
  const cloudMetadataHosts = [
    '169.254.169.254',  // AWS, GCP, Azure metadata
    'metadata.google.internal',
  ];

  if (cloudMetadataHosts.includes(hostname.toLowerCase())) {
    throw new Error('Access to cloud metadata endpoints is not allowed');
  }
}

/**
 * Validates file size from response headers
 * @param contentLength Content-Length header value
 * @param maxSizeBytes Maximum allowed file size in bytes
 * @throws Error if file is too large
 */
export function validateFileSize(contentLength: string | null, maxSizeBytes: number): void {
  if (contentLength) {
    const size = parseInt(contentLength, 10);
    if (isNaN(size)) {
      throw new Error('Invalid Content-Length header');
    }
    if (size > maxSizeBytes) {
      const maxSizeMB = (maxSizeBytes / (1024 * 1024)).toFixed(1);
      const actualSizeMB = (size / (1024 * 1024)).toFixed(1);
      throw new Error(`File size (${actualSizeMB}MB) exceeds maximum allowed size (${maxSizeMB}MB)`);
    }
  }
}
