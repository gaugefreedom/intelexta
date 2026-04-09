/**
 * Verifiable Summary MCP Server
 *
 * OpenAI Apps SDK server that provides a "summarize_content" tool
 * with cryptographic proof bundles.
 */

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { z } from 'zod';
import express from 'express';
import helmet from 'helmet';
import rateLimit from 'express-rate-limit';
import JSZip from 'jszip';
import { createHash, createHmac, randomUUID, timingSafeEqual } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { pipeline } from 'node:stream/promises';
import { fileURLToPath } from 'node:url';
import { config } from 'dotenv';

import { generateProofBundle } from './provenance.js';
import { summarize } from './summarizer.js';
import { LimitedBundleStorage } from './storage.js';

// Load environment variables
config();

// Configuration
const PORT = parseInt(process.env.PORT || '3000', 10);
const PUBLIC_URL = process.env.PUBLIC_URL || `http://localhost:${PORT}`;
const ED25519_SECRET_KEY = process.env.ED25519_SECRET_KEY;

function parsePositiveInteger(envVar: string | undefined, fallback: number, name: string): number {
  if (!envVar) {
    return fallback;
  }

  const parsed = Number.parseInt(envVar, 10);
  if (Number.isNaN(parsed) || parsed <= 0) {
    console.warn(
      `⚠️  ${name} is invalid (${envVar}). Falling back to ${fallback}.`
    );
    return fallback;
  }

  return parsed;
}

const DEFAULT_BUNDLE_MAX_ENTRIES = 32;
const DEFAULT_BUNDLE_MAX_TOTAL_BYTES = 128 * 1024 * 1024; // 128 MiB
const DEFAULT_BUNDLE_TTL_MS = 24 * 60 * 60 * 1000; // 24 hours retention for review
const DEFAULT_BUNDLE_CLEANUP_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes
const DEFAULT_DOWNLOAD_URL_TTL_MS = 15 * 60 * 1000; // 15 minutes
const DEFAULT_BUNDLE_STORAGE_DIR = resolve(process.cwd(), '.data', 'bundles');
const __dirname = dirname(fileURLToPath(import.meta.url));
const OPENAI_APPS_CHALLENGE_TOKEN = '-YF4xJ6u9SsnqG5v338Ccoy2vYPnNNoaWUNEFJrYksA';
const RESPONSE_CACHE_TTL_MS = 60 * 1000;
const RESPONSE_CACHE_MAX_ENTRIES = 32;
const DEFAULT_INPUT_TEXT_MAX_CHARS = 256000;
const DEFAULT_INPUT_TEXT_MAX_CHARS_WITH_SOURCE = 64000;
const DEFAULT_CORS_ALLOW_ORIGINS = ['https://chatgpt.com', 'https://chat.openai.com'];

const DOWNLOAD_URL_SIGNING_SECRET = process.env.DOWNLOAD_URL_SIGNING_SECRET;

type CachedToolResponse = {
  createdAt: number;
  response: {
    content: Array<{ type: 'text'; text: string }>;
    structuredContent: {
      summary: string;
      car: {
        id: string;
        car_id: string;
        valid: boolean;
        signer: string;
        hash: string;
        download_url: string;
      };
      meta: {
        bytes_processed: number;
        runtime_ms: number;
      };
    };
  };
};

const responseCache = new Map<string, CachedToolResponse>();

function getCachedResponse(key: string) {
  const cached = responseCache.get(key);
  if (!cached) {
    return undefined;
  }
  if (Date.now() - cached.createdAt > RESPONSE_CACHE_TTL_MS) {
    responseCache.delete(key);
    return undefined;
  }
  return cached.response;
}

function setCachedResponse(key: string, response: CachedToolResponse['response']) {
  responseCache.set(key, { createdAt: Date.now(), response });
  if (responseCache.size > RESPONSE_CACHE_MAX_ENTRIES) {
    // Remove the oldest entry to bound memory.
    const oldest = Array.from(responseCache.entries()).sort(
      (a, b) => a[1].createdAt - b[1].createdAt
    )[0]?.[0];
    if (oldest) {
      responseCache.delete(oldest);
    }
  }
}

const BUNDLE_MAX_ENTRIES = parsePositiveInteger(
  process.env.BUNDLE_STORAGE_MAX_ENTRIES,
  DEFAULT_BUNDLE_MAX_ENTRIES,
  'BUNDLE_STORAGE_MAX_ENTRIES'
);

const BUNDLE_MAX_TOTAL_BYTES = parsePositiveInteger(
  process.env.BUNDLE_STORAGE_MAX_TOTAL_BYTES,
  DEFAULT_BUNDLE_MAX_TOTAL_BYTES,
  'BUNDLE_STORAGE_MAX_TOTAL_BYTES'
);

const BUNDLE_TTL_MS = parsePositiveInteger(
  process.env.BUNDLE_STORAGE_TTL_MS,
  DEFAULT_BUNDLE_TTL_MS,
  'BUNDLE_STORAGE_TTL_MS'
);

const DOWNLOAD_URL_TTL_MS = parsePositiveInteger(
  process.env.DOWNLOAD_URL_TTL_MS,
  DEFAULT_DOWNLOAD_URL_TTL_MS,
  'DOWNLOAD_URL_TTL_MS'
);

const INPUT_TEXT_MAX_CHARS = parsePositiveInteger(
  process.env.INPUT_TEXT_MAX_CHARS,
  DEFAULT_INPUT_TEXT_MAX_CHARS,
  'INPUT_TEXT_MAX_CHARS'
);

const INPUT_TEXT_MAX_CHARS_WITH_SOURCE = parsePositiveInteger(
  process.env.INPUT_TEXT_MAX_CHARS_WITH_SOURCE,
  DEFAULT_INPUT_TEXT_MAX_CHARS_WITH_SOURCE,
  'INPUT_TEXT_MAX_CHARS_WITH_SOURCE'
);

const CORS_ALLOW_ORIGINS = new Set(
  (process.env.CORS_ALLOW_ORIGINS
    ? process.env.CORS_ALLOW_ORIGINS.split(',')
    : DEFAULT_CORS_ALLOW_ORIGINS
  )
    .map((origin) => origin.trim())
    .filter(Boolean)
);

const BUNDLE_STORAGE_DIR = process.env.BUNDLE_STORAGE_DIR
  ? resolve(process.env.BUNDLE_STORAGE_DIR)
  : DEFAULT_BUNDLE_STORAGE_DIR;

const BUNDLE_CLEANUP_INTERVAL_MS = Math.min(
  parsePositiveInteger(
    process.env.BUNDLE_STORAGE_CLEANUP_INTERVAL_MS,
    DEFAULT_BUNDLE_CLEANUP_INTERVAL_MS,
    'BUNDLE_STORAGE_CLEANUP_INTERVAL_MS'
  ),
  BUNDLE_TTL_MS
);

// Validate production environment
if (!process.env.PUBLIC_URL || PUBLIC_URL.includes('localhost')) {
  console.warn('⚠️  WARNING: PUBLIC_URL is not set to production domain!');
  console.warn('⚠️  Download URLs will use localhost and will not work in production.');
}

if (!DOWNLOAD_URL_SIGNING_SECRET && process.env.VITEST_WORKER_ID === undefined) {
  throw new Error('DOWNLOAD_URL_SIGNING_SECRET is required to issue signed download URLs.');
}

function buildDownloadSignature(id: string, expiresAtSeconds: number): string | undefined {
  if (!DOWNLOAD_URL_SIGNING_SECRET) {
    return undefined;
  }
  return createHmac('sha256', DOWNLOAD_URL_SIGNING_SECRET)
    .update(`${id}.${expiresAtSeconds}`)
    .digest('hex');
}

function isValidDownloadSignature(id: string, expParam: unknown, sigParam: unknown): boolean {
  if (!DOWNLOAD_URL_SIGNING_SECRET) {
    return true;
  }
  if (typeof expParam !== 'string' || typeof sigParam !== 'string') {
    return false;
  }
  const expiresAtSeconds = Number.parseInt(expParam, 10);
  if (!Number.isFinite(expiresAtSeconds)) {
    return false;
  }
  if (expiresAtSeconds < Math.floor(Date.now() / 1000)) {
    return false;
  }
  const expected = buildDownloadSignature(id, expiresAtSeconds);
  if (!expected) {
    return false;
  }
  const expectedBuffer = Buffer.from(expected, 'utf8');
  const providedBuffer = Buffer.from(sigParam, 'utf8');
  if (expectedBuffer.length !== providedBuffer.length) {
    return false;
  }
  return timingSafeEqual(expectedBuffer, providedBuffer);
}

function createDownloadUrl(id: string): string {
  if (!DOWNLOAD_URL_SIGNING_SECRET) {
    throw new Error('DOWNLOAD_URL_SIGNING_SECRET is required to issue download URLs.');
  }
  const ttlSeconds = Math.max(1, Math.floor(DOWNLOAD_URL_TTL_MS / 1000));
  const expiresAtSeconds = Math.floor(Date.now() / 1000) + ttlSeconds;
  const signature = buildDownloadSignature(id, expiresAtSeconds);
  return `${PUBLIC_URL}/download/${id}?exp=${expiresAtSeconds}&sig=${signature}`;
}

function extractSummarizeInput(payload: unknown): { text: string; includeSource: boolean } | undefined {
  if (!payload || typeof payload !== 'object') {
    return undefined;
  }
  if (Array.isArray(payload)) {
    for (const entry of payload) {
      const found = extractSummarizeInput(entry);
      if (found) {
        return found;
      }
    }
    return undefined;
  }
  const params = (payload as { params?: any }).params;
  if (!params || typeof params !== 'object') {
    return undefined;
  }
  const toolName = params.name ?? params.tool?.name ?? params.tool_name;
  if (toolName !== 'summarize_content') {
    return undefined;
  }
  const args =
    (params.arguments && typeof params.arguments === 'object' ? params.arguments : undefined) ??
    (params.tool?.arguments && typeof params.tool.arguments === 'object' ? params.tool.arguments : undefined);
  if (!args || typeof args !== 'object') {
    return undefined;
  }
  if (typeof args.text !== 'string') {
    return undefined;
  }
  return {
    text: args.text,
    includeSource: Boolean(args.include_source)
  };
}

// ============================================================================
// Initialize MCP Server
// ============================================================================

const server = new McpServer({
  name: 'verifiable-summary',
  version: '0.1.0'
});

const bundleStorage = new LimitedBundleStorage({
  maxEntries: BUNDLE_MAX_ENTRIES,
  maxTotalBytes: BUNDLE_MAX_TOTAL_BYTES,
  ttlMs: BUNDLE_TTL_MS,
  directory: BUNDLE_STORAGE_DIR
});

console.log(
  `Bundle storage configured with maxEntries=${BUNDLE_MAX_ENTRIES}, maxTotalBytes=${BUNDLE_MAX_TOTAL_BYTES}, ttlMs=${BUNDLE_TTL_MS}, directory=${BUNDLE_STORAGE_DIR}`
);

// Clean up expired bundles on an interval to keep memory bounded
const cleanupTimer = setInterval(() => {
  const removed = bundleStorage.cleanupExpired();
  if (removed > 0) {
    console.log(`Cleaned up ${removed} expired bundle${removed === 1 ? '' : 's'} from storage.`);
  }
}, BUNDLE_CLEANUP_INTERVAL_MS);

if (typeof cleanupTimer.unref === 'function') {
  cleanupTimer.unref();
}

// ============================================================================
// Register UI Resource (Skybridge Widget)
// ============================================================================

// Embed the logo as a data URL so the widget stays self contained in ChatGPT.
function loadLogoDataUrl(): string | undefined {
  const logoPath = resolve(__dirname, '../assets/logo.png');
  try {
    const buffer = readFileSync(logoPath);
    if (buffer.length === 0) {
      return undefined;
    }
    return `data:image/png;base64,${buffer.toString('base64')}`;
  } catch (error) {
    console.warn(
      `⚠️  Logo not found at ${logoPath}. Continuing without embedded logo.`
    );
    return undefined;
  }
}

const LOGO_DATA_URL = loadLogoDataUrl();
const widgetLogoElement = LOGO_DATA_URL
  ? `<img src="${LOGO_DATA_URL}" alt="Intelexta fingerprint logo" class="logo-img" />`
  : '';
const logoShellAttributes = LOGO_DATA_URL ? '' : 'style="display: none;"';

// Load widget HTML (will be built by the web project)
const widgetHtml = String.raw`<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <style>
    :root {
      /* Light Theme / Brand Variables */
      --bg: #f8fafc;            /* Your requested background */
      --panel: #ffffff;
      --text-main: #0f172a;     /* Dark Slate for headings */
      --text-body: #334155;     /* Softer slate for body */
      --text-muted: #64748b;
      --border: #e2e8f0;
      
      /* Brand Colors */
      --brand-green: #10b981;   /* Matching the "Validate" buttons */
      --brand-green-soft: #d1fae5;
      --brand-purple: #4f46e5;  /* Matching "Upgrade to Pro" */
      --brand-purple-hover: #4338ca;
      
      /* Status Colors */
      --success: #059669;
      --warning: #d97706;
      --warning-bg: #fef3c7;
      
      --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
      --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
      
      font-family: 'DM Sans', 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    }

    * { box-sizing: border-box; }

    body {
      margin: 0;
      padding: 16px;
      min-height: 100vh;
      background-color: var(--bg);
      color: var(--text-body);
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 14px;
    }

    #root {
      width: 100%;
      max-width: 720px;
    }

    /* Main Card */
    .summary-card {
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 24px;
      box-shadow: var(--shadow-md);
    }

    .header {
      display: flex;
      align-items: flex-start;
      justify-content: space-between;
      gap: 16px;
      margin-bottom: 24px;
      border-bottom: 1px solid var(--border);
      padding-bottom: 20px;
    }

    .brand {
      display: flex;
      align-items: center;
      gap: 16px;
    }

    /* Fixed Logo Styling */
    .logo-shell {
      width: 48px;
      height: 48px;
      background: #ffffff;
      border-radius: 10px;
      border: 1px solid var(--border);
      display: flex;
      align-items: center;
      justify-content: center;
      overflow: hidden;
      flex-shrink: 0;
    }

    .logo-img {
      width: 100%;
      height: 100%;
      object-fit: contain; /* Prevents stretching */
      padding: 4px;        /* Adds breathing room */
    }

    .title-group h3 {
      margin: 0;
      font-size: 18px;
      font-weight: 700;
      color: var(--text-main);
      letter-spacing: -0.01em;
    }

    .eyebrow {
      font-size: 11px;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-muted);
      font-weight: 600;
      margin-bottom: 2px;
    }

    /* Badges */
    .badge {
      padding: 6px 12px;
      border-radius: 20px;
      font-weight: 600;
      font-size: 12px;
      display: inline-flex;
      align-items: center;
      gap: 6px;
    }

    .badge::before {
      content: '';
      width: 6px;
      height: 6px;
      border-radius: 50%;
    }

    .badge.verified {
      background: var(--brand-green-soft);
      color: var(--success);
      border: 1px solid rgba(16, 185, 129, 0.2);
    }
    .badge.verified::before { background: var(--success); }

    .badge.unsigned {
      background: var(--warning-bg);
      color: var(--warning);
      border: 1px solid rgba(217, 119, 6, 0.2);
    }
    .badge.unsigned::before { background: var(--warning); }

    /* Summary Content */
    .summary-section {
      margin-bottom: 24px;
    }

    .summary-content {
      line-height: 1.6;
      color: var(--text-main);
      font-size: 15px;
    }
    
    .summary-content.clamped {
      max-height: 160px;
      overflow: hidden;
      mask-image: linear-gradient(to bottom, black 60%, transparent 100%);
      -webkit-mask-image: linear-gradient(to bottom, black 60%, transparent 100%);
    }
    
    .summary-content.expanded {
      max-height: none;
    }

    .read-more-btn {
      background: none;
      border: none;
      color: var(--brand-purple);
      font-weight: 600;
      font-size: 13px;
      padding: 8px 0;
      cursor: pointer;
      margin-top: 4px;
    }
    
    .read-more-btn:hover { text-decoration: underline; }

    /* Metadata Grid */
    .meta-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 12px;
      margin-bottom: 24px;
    }

    .info-card {
      background: var(--bg);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 10px 12px;
    }

    .label {
      font-size: 11px;
      text-transform: uppercase;
      color: var(--text-muted);
      letter-spacing: 0.02em;
      margin-bottom: 4px;
      font-weight: 600;
    }

    .value {
      font-family: 'JetBrains Mono', monospace;
      font-size: 12px;
      color: var(--text-main);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    
    .value.clickable { cursor: pointer; transition: color 0.2s; }
    .value.clickable:hover { color: var(--brand-purple); }

    /* Actions */
    .actions {
      display: flex;
      flex-wrap: wrap;
      gap: 12px;
      padding-top: 20px;
      border-top: 1px solid var(--border);
    }

    .btn {
      padding: 10px 18px;
      border-radius: 6px;
      font-weight: 600;
      font-size: 14px;
      cursor: pointer;
      transition: all 0.15s ease;
      display: inline-flex;
      align-items: center;
      justify-content: center;
    }

    .btn:disabled { opacity: 0.5; cursor: not-allowed; }

    .btn-primary {
      background: var(--brand-purple);
      color: white;
      border: 1px solid transparent;
      box-shadow: 0 1px 2px rgba(0,0,0,0.1);
    }
    
    .btn-primary:not(:disabled):hover {
      background: var(--brand-purple-hover);
      transform: translateY(-1px);
    }

    .btn-secondary {
      background: white;
      color: var(--text-body);
      border: 1px solid var(--border);
    }
    
    .btn-secondary:hover {
      background: var(--bg);
      border-color: #cbd5e1;
    }

    /* Loading State */
    .loading {
      text-align: center;
      padding: 40px;
      color: var(--text-muted);
    }
    
    .spinner {
      width: 24px;
      height: 24px;
      border: 3px solid var(--border);
      border-top-color: var(--brand-purple);
      border-radius: 50%;
      animation: spin 1s linear infinite;
      margin: 0 auto 12px;
    }
    
    @keyframes spin { to { transform: rotate(360deg); } }

  </style>
</head>
<body>
  <div id="root">
    
    <div class="loading" data-loading>
      <div class="spinner"></div>
      <div data-loading-text>Generating summary...</div>
    </div>

    <div class="summary-card" data-summary-card hidden>
      <div class="header">
        <div class="brand">
          <div class="logo-shell" ${logoShellAttributes}>
            ${widgetLogoElement}
          </div>
          <div class="title-group">
            <div class="eyebrow">Intelexta</div>
            <h3>Verifiable Summary</h3>
          </div>
        </div>
        <span class="badge" data-badge>Processing...</span>
      </div>

      <div class="summary-section">
        <div class="summary-content" data-summary-content>
          <p class="summary-text" data-summary-text></p>
        </div>
        <button class="read-more-btn" data-read-more hidden type="button">Read more</button>
      </div>

      <div class="meta-grid">
        <div class="info-card">
          <div class="label">Signer Key</div>
          <div class="value clickable" data-signer-value title="Click to copy"></div>
        </div>
        <div class="info-card">
          <div class="label">Receipt Hash</div>
          <div class="value clickable" data-tree-hash title="Click to copy"></div>
        </div>
        <div class="info-card">
          <div class="label">Processing Stats</div>
          <div class="value" data-processed></div>
        </div>
      </div>

      <div class="actions">
        <button class="btn btn-primary" type="button" data-download>
          Download Proof Bundle
        </button>
        <button class="btn btn-secondary" type="button" data-verify>Verify Receipt</button>
        <button class="btn btn-secondary" type="button" data-learn-more>What is a CAR?</button>
      </div>
    </div>

  </div>
  <script>
    (function() {
      const root = document.getElementById('root');
      if (!root) return;

      const summaryCard = root.querySelector('[data-summary-card]');
      const loadingEl = root.querySelector('[data-loading]');
      const loadingTextEl = loadingEl?.querySelector('[data-loading-text]');
      const defaultLoadingText = 'Generating summary...';
      const badgeEl = root.querySelector('[data-badge]');
      const summaryContentEl = root.querySelector('[data-summary-content]');
      const summaryEl = root.querySelector('[data-summary-text]');
      const readMoreBtn = root.querySelector('[data-read-more]');
      const signerEl = root.querySelector('[data-signer-value]');
      const treeHashEl = root.querySelector('[data-tree-hash]');
      const processedEl = root.querySelector('[data-processed]');
      const downloadButton = root.querySelector('[data-download]');
      const learnMoreBtn = root.querySelector('[data-learn-more]');
      const verifyBtn = root.querySelector('[data-verify]');

      const subscribe = (callback) => {
        window.addEventListener('openai:set_globals', callback);
        return () => window.removeEventListener('openai:set_globals', callback);
      };

      const getSnapshot = () => {
        const toolOutput = window.openai?.toolOutput;
        if (!toolOutput || typeof toolOutput !== 'object') return undefined;

        // Handle structured output
        if (toolOutput.structuredContent && typeof toolOutput.structuredContent === 'object') {
          return toolOutput.structuredContent;
        }
        
        // Handle direct output
        if ('summary' in toolOutput) return toolOutput;
        
        return undefined;
      };

      let currentOutput = getSnapshot();

      const setSummaryContent = (text) => {
        if (!summaryEl) return;
        summaryEl.textContent = text ?? '';
        summaryEl.style.whiteSpace = 'pre-wrap';
      };

      const render = () => {
        const toolOutput = getSnapshot();

        if (!summaryCard || !loadingEl) return;

        // 1. Loading State
        if (!toolOutput) {
          summaryCard.hidden = true;
          loadingEl.hidden = false;
          if (loadingTextEl) loadingTextEl.textContent = defaultLoadingText;
          return;
        }

        const summary = toolOutput.summary ?? '';
        const car = toolOutput.car;
        const meta = toolOutput.meta;

        // 2. Partial Data State
        if (!car || !meta) {
          summaryCard.hidden = true;
          loadingEl.hidden = false;
          if (loadingTextEl) loadingTextEl.textContent = 'Finalizing verification proof...';
          return;
        }

        // 3. Success State
        loadingEl.hidden = true;
        summaryCard.hidden = false;

        setSummaryContent(summary);

        // Handle Read More Toggle
        const isTruncated = summary.length > 250;
        if (isTruncated && readMoreBtn && summaryContentEl) {
          summaryContentEl.classList.add('clamped');
          readMoreBtn.hidden = false;
          readMoreBtn.textContent = 'Read more';
          readMoreBtn.onclick = () => {
            const isClamped = summaryContentEl.classList.contains('clamped');
            if (isClamped) {
              summaryContentEl.classList.remove('clamped');
              summaryContentEl.classList.add('expanded');
              readMoreBtn.textContent = 'Collapse';
            } else {
              summaryContentEl.classList.remove('expanded');
              summaryContentEl.classList.add('clamped');
              readMoreBtn.textContent = 'Read more';
            }
          };
        } else if (readMoreBtn) {
          readMoreBtn.hidden = true;
        }

        // Update Badge & Signer
        if (car.valid) {
          badgeEl.textContent = 'Signed (Ed25519)';
          badgeEl.className = 'badge verified';
          
          signerEl.textContent = car.signer.slice(0, 16) + '...';
          signerEl.onclick = () => copyToClipboard(car.signer, signerEl);
        } else {
          badgeEl.textContent = 'Unsigned';
          badgeEl.className = 'badge unsigned';
          signerEl.textContent = 'No signature';
          signerEl.onclick = null;
        }

        // Update Hash
        const shortHash = car.hash.slice(0, 16) + '...';
        treeHashEl.textContent = shortHash;
        treeHashEl.onclick = () => copyToClipboard(car.hash, treeHashEl);

        // Update Stats
        processedEl.textContent = meta.runtime_ms + 'ms / ' + formatBytes(meta.bytes_processed);

        // Buttons
        downloadButton.disabled = !car.download_url;
        downloadButton.onclick = car.download_url
          ? () => window.openai?.openExternal?.({ href: car.download_url })
          : null;

        if (verifyBtn) verifyBtn.onclick = () => window.openai?.openExternal?.({ href: 'https://verify.intelexta.com' });
        if (learnMoreBtn) learnMoreBtn.onclick = () => window.openai?.openExternal?.({ href: 'https://intelexta.com' });
      };

      // Helpers
      function copyToClipboard(text, element) {
        if (navigator.clipboard) {
          navigator.clipboard.writeText(text).then(() => {
            const oldText = element.textContent;
            element.textContent = 'Copied!';
            setTimeout(() => element.textContent = oldText, 1500);
          });
        }
      }

      function formatBytes(bytes) {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
      }

      subscribe(() => {
        const newOutput = getSnapshot();
        if (newOutput !== currentOutput) {
          currentOutput = newOutput;
          render();
        }
      });

      render();
    })();
  </script>
</body>
</html>
`;

const WIDGET_RESOURCE_URI = 'ui://widget/verifiable-summary';

server.registerResource(
  'verifiable-summary-widget',
  WIDGET_RESOURCE_URI,
  {
    title: 'Verifiable Summary Widget',
    description: 'Renders verifiable summary results in ChatGPT',
    mimeType: 'text/html+skybridge'
  },
  async () => ({
    contents: [
      {
        uri: WIDGET_RESOURCE_URI,
        text: widgetHtml,
        mimeType: 'text/html+skybridge',
        _meta: {
          'openai/widgetDomain': 'https://chatgpt.com',
          'openai/widgetPrefersBorder': true,
          'openai/widgetCSP': {
            connect_domains: ['https://chatgpt.com'],
            resource_domains: ['https://*.oaistatic.com']
          }
        }
      }
    ]
  })
);

// ============================================================================
// Register Tool: summarize_content
// ============================================================================

const inputSchema = z.object({
  text: z.string(),
  style: z.enum(['tl;dr', 'bullets', 'outline']).default('tl;dr'),
  include_source: z.boolean().default(false)
});

server.registerTool(
  'summarize_content',
  {
    description:
      'Create a concise summary and a downloadable proof bundle (ZIP) with a CAR receipt and optional Ed25519 signature. Call once per user request.',
    inputSchema: inputSchema.shape,
    annotations: {
      title: 'Generate Verifiable Summary',
      readOnlyHint: false,
      destructiveHint: false,
      openWorldHint: false,
      idempotentHint: false
    },
    _meta: {
      'openai/outputTemplate': WIDGET_RESOURCE_URI,
      'openai/toolInvocation/invoking': 'Generating signed summary...',
      'openai/toolInvocation/invoked': 'Summary generated.'
    },
    // @ts-expect-error OpenAI security metadata not in SDK types
    securitySchemes: [{ type: 'noauth' }]
  },
  async (params) => {
    const startTime = Date.now();
    const input = inputSchema.parse(params);
    if (input.text.length > INPUT_TEXT_MAX_CHARS) {
      throw new Error(`Payload too large: text exceeds ${INPUT_TEXT_MAX_CHARS} characters.`);
    }
    if (input.include_source && input.text.length > INPUT_TEXT_MAX_CHARS_WITH_SOURCE) {
      throw new Error(
        `Payload too large: text exceeds ${INPUT_TEXT_MAX_CHARS_WITH_SOURCE} characters when include_source=true.`
      );
    }
    const requestHash = createHash('sha256')
      .update(input.text)
      .update(input.style)
      .update(input.include_source ? 'include' : 'no-include')
      .update(ED25519_SECRET_KEY ? 'signed' : 'unsigned')
      .digest('hex');

    const cached = getCachedResponse(requestHash);
    if (cached) {
      console.log(`[cache] Reusing bundle for request hash ${requestHash}`);
      return cached;
    }

    try {
      // 1. Prepare content
      const source = { url: 'inline://text', content: input.text };

      if (!source.content.trim()) {
        throw new Error('No content provided');
      }

      console.log(`Processing ${source.content.length} bytes from ${source.url}`);

      // 2. Summarize
      const { summary, usage } = await summarize(source.content, input.style);
      console.log(`Summary generated (${summary.length} chars)`);

      // 3. Generate proof bundle
      const { bundle: artifacts, isSigned } = await generateProofBundle(
        source,
        summary,
        'ChatGPT',
        ED25519_SECRET_KEY,
        {
          includeSource: input.include_source,
          usage: usage
            ? {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens
              }
            : undefined
        }
      );
      console.log('Proof bundle generated');

      // 4. Create ZIP
      const zip = new JSZip();
      for (const [path, content] of Object.entries(artifacts)) {
        zip.file(path, content);
      }
      const zipBuffer = await zip.generateAsync({ type: 'nodebuffer' });
      console.log(`ZIP created (${zipBuffer.length} bytes)`);

      // 5. Store and generate download URL
      const id = randomUUID();
      bundleStorage.store(id, zipBuffer);
      const downloadUrl = createDownloadUrl(id);

      const stats = bundleStorage.getStats();
      console.log(`[cache] Stored ZIP bundle: ${id} (${(zipBuffer.length / 1024 / 1024).toFixed(2)}MB)`);
      console.log(`[cache] Cache stats: ${stats.totalEntries} bundles, ${(stats.totalBytes / 1024 / 1024).toFixed(2)}MB total`);

      // 6. Parse car.json for metadata
      const carData = JSON.parse(artifacts['car.json']);
      const signer = isSigned && carData.signer_public_key ? carData.signer_public_key : 'unsigned';
      const badgeStatus = isSigned ? 'Signed (ed25519)' : 'Unsigned - no cryptographic proof';

      const runtime = Date.now() - startTime;
      console.log(`Total runtime: ${runtime}ms`);

      // FIX IS HERE: Added explicit type annotation : CachedToolResponse['response']
      const response: CachedToolResponse['response'] = {
        content: [{
          type: 'text',
          text: `Verifiable summary generated.\n\nCAR ID: ${carData.id}\nSigner: ${signer}\nStatus: ${badgeStatus}\nRuntime: ${runtime}ms`
        }],
        structuredContent: {
          summary,
          car: {
            id,
            car_id: carData.id,
            valid: isSigned,
            signer,
            hash: carData.id.replace('car:', ''),
            download_url: downloadUrl
          },
          meta: {
            bytes_processed: Buffer.byteLength(source.content, 'utf-8'),
            runtime_ms: runtime
          }
        }
      };
      
      setCachedResponse(requestHash, response);
      return response;
    } catch (error) {
      console.error('Error in summarize_content:', error);

      return {
        content: [{
          type: 'text',
          text: `Error: ${error instanceof Error ? error.message : 'Unknown error'}`
        }],
        isError: true
      };
    }
  }
);

// ============================================================================
// Express Server for HTTP Endpoints
// ============================================================================

const app = express();

// Apply security headers
app.use(helmet({
  contentSecurityPolicy: false, // Disabled for MCP compatibility
  crossOriginEmbedderPolicy: false,
}));

// Apply rate limiting to prevent DoS attacks
const limiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 100, // Limit each IP to 100 requests per window
  standardHeaders: true,
  legacyHeaders: false,
  message: 'Too many requests from this IP, please try again later.',
});

// Apply rate limiter to all routes
app.use(limiter);

// Stricter rate limiting for MCP endpoint (expensive operations)
const mcpLimiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 20, // Limit to 20 MCP requests per window
  standardHeaders: true,
  legacyHeaders: false,
  message: 'Too many MCP requests, please try again later.',
});

// Health check
app.get('/health', (_req, res) => {
  res.json({ status: 'ok', timestamp: new Date().toISOString() });
});

// Domain verification for OpenAI Apps
app.get('/.well-known/openai-apps-challenge', (_req, res) => {
  res.setHeader('Content-Type', 'text/plain');
  res.send(OPENAI_APPS_CHALLENGE_TOKEN);
});

// Download endpoint
app.get('/download/:id', (req, res) => {
  const origin = req.get('origin');
  if (origin && CORS_ALLOW_ORIGINS.has(origin)) {
    res.setHeader('Access-Control-Allow-Origin', origin);
    res.setHeader('Access-Control-Allow-Methods', 'GET');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
    res.setHeader('Vary', 'Origin');
  }
  res.setHeader('Cache-Control', 'no-store');
  res.setHeader('X-Content-Type-Options', 'nosniff');

  if (!isValidDownloadSignature(req.params.id, req.query.exp, req.query.sig)) {
    res.setHeader('Content-Type', 'text/plain; charset=utf-8');
    return res
      .status(410)
      .send('Download link expired or invalid. Please regenerate the verifiable summary to get a new link.');
  }

  const bundle = bundleStorage.getStream(req.params.id);

  if (!bundle) {
    console.warn(`Download requested for missing or expired bundle: ${req.params.id}`);
    return res.status(404).json({ error: 'Bundle not found or expired' });
  }

  console.log(`[download] Serving ZIP bundle: ${req.params.id} (${(bundle.size / 1024 / 1024).toFixed(2)}MB)`);

  res.setHeader('Content-Type', 'application/zip');
  res.setHeader('Content-Disposition', 'attachment; filename="verifiable.car.zip"');
  res.setHeader('Content-Length', bundle.size.toString());

  pipeline(bundle.stream, res)
    .then(() => {
      console.log(
        `Streamed bundle ${req.params.id} (${bundle.size} bytes, created at ${new Date(bundle.createdAt).toISOString()}).`
      );
    })
    .catch((error) => {
      console.error(`Failed to stream bundle ${req.params.id}:`, error);
      if (!res.headersSent) {
        res.status(500).json({ error: 'Failed to stream bundle' });
      } else {
        res.end();
      }
    });
});

// MCP endpoint wired through the official streamable HTTP transport
app.post('/mcp', mcpLimiter, express.json({ limit: '10mb' }), async (req, res) => {
  const summarizeInput = extractSummarizeInput(req.body);
  if (summarizeInput) {
    const textLength = summarizeInput.text.length;
    if (textLength > INPUT_TEXT_MAX_CHARS) {
      return res
        .status(413)
        .json({ error: `Payload too large: text exceeds ${INPUT_TEXT_MAX_CHARS} characters.` });
    }
    if (summarizeInput.includeSource && textLength > INPUT_TEXT_MAX_CHARS_WITH_SOURCE) {
      return res.status(413).json({
        error: `Payload too large: text exceeds ${INPUT_TEXT_MAX_CHARS_WITH_SOURCE} characters when include_source=true.`
      });
    }
  }
  const transport = new StreamableHTTPServerTransport({
    sessionIdGenerator: undefined,
    enableJsonResponse: true
  });

  res.on('close', () => {
    transport.close();
  });

  try {
    await server.connect(transport);
    await transport.handleRequest(req, res, req.body);
  } catch (error) {
    console.error('Error handling MCP request:', error);
    if (!res.headersSent) {
      res.status(500).json({ error: 'Failed to handle MCP request' });
    }
  }
});

// Start server
if (process.env.VITEST_WORKER_ID === undefined) {
  app.listen(PORT, () => {
    console.log(`
╔═══════════════════════════════════════════════════════════╗
║  Verifiable Summary MCP Server                            ║
╠═══════════════════════════════════════════════════════════╣
║  URL: ${PUBLIC_URL.padEnd(49)} ║
║  Port: ${PORT.toString().padEnd(48)} ║
║  Signing: ${(ED25519_SECRET_KEY ? 'Enabled' : 'Disabled (unsigned mode)').padEnd(44)} ║
╚═══════════════════════════════════════════════════════════╝

Tools:
  • summarize_content - Generate verifiable summaries with proof

Endpoints:
  • GET /health - Health check
  • GET /download/:id - Download CAR bundles
  • POST /mcp - MCP protocol endpoint

Next steps:
  1. Expose this server with ngrok: ngrok http ${PORT}
  2. Add MCP connector in ChatGPT Developer Mode
  3. Invoke "summarize_content" tool
`);
  });
}
