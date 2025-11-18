/**
 * Text truncation and formatting utilities for the Content Visualizer
 */

/**
 * Truncate text to a maximum length with ellipsis
 */
export function truncateText(text: string, maxLength: number): string {
  if (!text || text.length <= maxLength) return text;
  return text.slice(0, maxLength) + '...';
}

/**
 * Format a date string to human-readable format
 */
export function formatDate(dateString: string): string {
  if (!dateString) return 'Unknown';
  try {
    const date = new Date(dateString);
    if (isNaN(date.getTime())) return dateString;
    return date.toLocaleString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      timeZoneName: 'short'
    });
  } catch {
    return dateString;
  }
}

/**
 * Format a number with commas
 */
export function formatNumber(num: number): string {
  return num.toLocaleString('en-US');
}

/**
 * Format token count with K suffix for thousands
 */
export function formatTokens(tokens: number): string {
  if (tokens >= 1000) {
    return `${(tokens / 1000).toFixed(1)}K`;
  }
  return tokens.toString();
}

/**
 * Truncate a hash or ID for display
 */
export function truncateHash(hash: string, prefixLen = 12, suffixLen = 8): string {
  if (!hash || hash.length <= prefixLen + suffixLen + 3) return hash;
  return `${hash.slice(0, prefixLen)}...${hash.slice(-suffixLen)}`;
}

/**
 * Parse and truncate JSON for preview
 */
export function truncateJson(jsonString: string | null | undefined, maxLength: number = 160): string {
  if (!jsonString) return '';
  try {
    // Try to parse and pretty-print
    const parsed = JSON.parse(jsonString);
    const pretty = JSON.stringify(parsed, null, 2);
    return truncateText(pretty, maxLength);
  } catch {
    // If invalid JSON, just truncate the string
    return truncateText(jsonString, maxLength);
  }
}
