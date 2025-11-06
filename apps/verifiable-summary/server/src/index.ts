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
import { randomUUID } from 'node:crypto';
import { pipeline } from 'node:stream/promises';
import { config } from 'dotenv';

import { generateProofBundle } from './provenance.js';
import { summarize } from './summarizer.js';
import { fetchRemoteFile, validateRemoteFileUrl } from './urlValidation.js';
import { LimitedBundleStorage } from './storage.js';

// Load environment variables
config();

// Configuration
const PORT = parseInt(process.env.PORT || '3000', 10);
const PUBLIC_URL = process.env.PUBLIC_URL || `http://localhost:${PORT}`;
const ED25519_SECRET_KEY = process.env.ED25519_SECRET_KEY;

const DEFAULT_REMOTE_FILE_MAX_BYTES = 2 * 1024 * 1024; // 2 MiB
const REMOTE_FILE_MAX_BYTES = (() => {
  const configured = process.env.REMOTE_FILE_MAX_BYTES;
  if (!configured) {
    return DEFAULT_REMOTE_FILE_MAX_BYTES;
  }

  const parsed = Number.parseInt(configured, 10);
  if (Number.isNaN(parsed) || parsed <= 0) {
    console.warn(
      `⚠️  REMOTE_FILE_MAX_BYTES is invalid (${configured}). Falling back to ${DEFAULT_REMOTE_FILE_MAX_BYTES}.`
    );
    return DEFAULT_REMOTE_FILE_MAX_BYTES;
  }

  return parsed;
})();

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
const DEFAULT_BUNDLE_TTL_MS = 60 * 60 * 1000; // 1 hour
const DEFAULT_BUNDLE_CLEANUP_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

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
  ttlMs: BUNDLE_TTL_MS
});

console.log(
  `Bundle storage configured with maxEntries=${BUNDLE_MAX_ENTRIES}, maxTotalBytes=${BUNDLE_MAX_TOTAL_BYTES}, ttlMs=${BUNDLE_TTL_MS}`
);

function createOversizeError(limit: number): Error {
  return new Error(`Remote file is too large. Maximum allowed size is ${limit} bytes.`);
}

async function readResponseBodyWithLimit(response: Response, limit: number): Promise<string> {
  const headerValue = response.headers.get('content-length');
  const parsedLength = headerValue ? Number.parseInt(headerValue, 10) : undefined;

  if (Number.isFinite(parsedLength) && typeof parsedLength === 'number') {
    if (parsedLength > limit) {
      if (response.body) {
        await response.body.cancel('Exceeded size limit');
      }
      throw createOversizeError(limit);
    }

    const text = await response.text();
    const byteLength = Buffer.byteLength(text, 'utf-8');
    if (byteLength > limit) {
      throw createOversizeError(limit);
    }
    return text;
  }

  if (!response.body) {
    throw new Error('Remote response is missing a readable body.');
  }

  const body = response.body;
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let received = 0;
  let result = '';
  let done = false;

  while (!done) {
    const { value, done: streamDone } = await reader.read();
    done = streamDone;

    if (done) {
      result += decoder.decode();
      break;
    }

    if (value) {
      received += value.byteLength;
      if (received > limit) {
        reader.releaseLock();
        await body.cancel('Exceeded size limit');
        throw createOversizeError(limit);
      }

      result += decoder.decode(value, { stream: true });
    }
  }

  return result;
}

export const internal = {
  readResponseBodyWithLimit,
  createOversizeError,
  REMOTE_FILE_MAX_BYTES
};

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

// Load widget HTML (will be built by the web project)
// For now, use a placeholder
const widgetHtml = `
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; padding: 20px; }
    .summary-card { max-width: 600px; margin: 0 auto; }
    .header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
    .badge { padding: 4px 12px; border-radius: 12px; font-size: 14px; font-weight: 500; }
    .badge.verified { background: #10b981; color: white; }
    .badge.unsigned { background: #f59e0b; color: #1f2937; }
    .summary-content { padding: 16px; background: #f9fafb; border-radius: 8px; margin-bottom: 16px; line-height: 1.6; }
    .verification-info { margin-bottom: 16px; }
    .info-row { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #e5e7eb; }
    .label { font-weight: 500; color: #6b7280; }
    .value { font-family: monospace; font-size: 14px; cursor: pointer; }
    .value:hover { color: #3b82f6; }
    .actions { display: flex; gap: 12px; }
    .btn { padding: 10px 20px; border: none; border-radius: 6px; font-weight: 500; cursor: pointer; }
    .btn-primary { background: #3b82f6; color: white; }
    .btn-primary:hover { background: #2563eb; }
    .btn-secondary { background: #e5e7eb; color: #374151; }
    .btn-secondary:hover { background: #d1d5db; }
    .loading { text-align: center; padding: 40px; color: #6b7280; }
  </style>
</head>
<body>
  <div id="root">
    <div class="summary-card" data-summary-card hidden>
      <div class="header">
        <h3>Verifiable Summary</h3>
        <span class="badge" data-badge></span>
      </div>

      <div class="summary-content">
        <p class="summary-text" data-summary-text></p>
      </div>

      <div class="verification-info">
        <div class="info-row">
          <span class="label">Signer:</span>
          <code class="value" data-signer-value title=""></code>
        </div>
        <div class="info-row">
          <span class="label">Tree Hash:</span>
          <code class="value" data-tree-hash title=""></code>
        </div>
        <div class="info-row">
          <span class="label">Processed:</span>
          <span class="value" data-processed></span>
        </div>
      </div>

      <div class="actions">
        <button class="btn btn-primary" type="button" data-download>
          Download CAR Bundle
        </button>
      </div>
    </div>
    <div class="loading" data-loading>Generating summary...</div>
  </div>
  <script>
    (function() {
      const root = document.getElementById('root');
      if (!root) {
        return;
      }

      const summaryCard = root.querySelector('[data-summary-card]');
      const loadingEl = root.querySelector('[data-loading]');
      const defaultLoadingText = loadingEl?.textContent ?? '';
      const badgeEl = root.querySelector('[data-badge]');
      const summaryEl = root.querySelector('[data-summary-text]');
      const signerEl = root.querySelector('[data-signer-value]');
      const treeHashEl = root.querySelector('[data-tree-hash]');
      const processedEl = root.querySelector('[data-processed]');
      const downloadButton = root.querySelector('[data-download]');

      const subscribe = (callback) => {
        window.addEventListener('openai:set_globals', callback);
        return () => window.removeEventListener('openai:set_globals', callback);
      };

      const getSnapshot = () => {
        const toolOutput = window.openai?.toolOutput;
        if (!toolOutput || typeof toolOutput !== 'object') {
          return undefined;
        }

        const structured =
          toolOutput && typeof toolOutput === 'object'
            ? toolOutput.structuredContent
            : undefined;

        if (structured && typeof structured === 'object') {
          return structured;
        }

        if (
          'summary' in toolOutput ||
          'car' in toolOutput ||
          'meta' in toolOutput
        ) {
          return toolOutput;
        }

        console.warn('Unsupported toolOutput shape', toolOutput);
        return undefined;
      };

      let currentOutput = getSnapshot();

      const setSummaryContent = (text) => {
        if (!summaryEl) {
          return;
        }

        summaryEl.textContent = text ?? '';
        const sanitized = summaryEl.textContent || '';
        const fragments = sanitized.split('\n');
        const nodes = [];

        for (let i = 0; i < fragments.length; i += 1) {
          if (i > 0) {
            nodes.push(document.createElement('br'));
          }
          nodes.push(document.createTextNode(fragments[i]));
        }

        summaryEl.replaceChildren(...nodes);
      };

      const render = () => {
        const toolOutput = getSnapshot();

        if (!summaryCard || !loadingEl || !badgeEl || !signerEl || !treeHashEl || !processedEl || !downloadButton) {
          return;
        }

        if (!toolOutput) {
          summaryCard.hidden = true;
          loadingEl.hidden = false;
          loadingEl.textContent = defaultLoadingText;
          return;
        }

        loadingEl.textContent = defaultLoadingText;
        loadingEl.hidden = true;
        summaryCard.hidden = false;

        const summary = toolOutput.summary ?? '';
        const car = toolOutput.car;
        const meta = toolOutput.meta;

        if (!car || !meta) {
          summaryCard.hidden = true;
          loadingEl.hidden = false;
          loadingEl.textContent = 'Verification data unavailable yet. Please try again shortly.';
          return;
        }

        setSummaryContent(summary);

        if (car.valid) {
          badgeEl.textContent = '✓ Verified';
          badgeEl.classList.add('verified');
          badgeEl.classList.remove('unsigned');
          signerEl.textContent = car.signer.slice(0, 24) + '...';
          signerEl.title = 'Click to copy signer';
          signerEl.onclick = () => {
            if (navigator.clipboard?.writeText) {
              navigator.clipboard.writeText(car.signer);
            }
          };
        } else {
          badgeEl.textContent = '⚠ Unsigned';
          badgeEl.classList.add('unsigned');
          badgeEl.classList.remove('verified');
          signerEl.textContent = 'unsigned';
          signerEl.title = 'Unsigned bundle - no signer';
          signerEl.onclick = null;
        }

        treeHashEl.textContent = car.hash.slice(0, 20) + '...';
        treeHashEl.title = car.hash;
        processedEl.textContent =
          meta.bytes_processed.toLocaleString() + ' bytes in ' + meta.runtime_ms + 'ms';

        downloadButton.disabled = !car.download_url;
        downloadButton.onclick = car.download_url
          ? () => window.openai?.openExternal?.({ href: car.download_url })
          : null;
      };

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
        text: widgetHtml
      }
    ]
  })
);

// ============================================================================
// Register Tool: summarize_content
// ============================================================================

const inputSchema = z.object({
  mode: z.enum(['text', 'file']).default('text'),
  text: z.string().optional(),
  fileUrl: z.string().url().optional(),
  style: z.enum(['tl;dr', 'bullets', 'outline']).default('tl;dr')
});

server.registerTool(
  'summarize_content',
  {
    description: 'Generates a verifiable summary with cryptographic proof bundle',
    inputSchema: inputSchema.shape,
    annotations: {
      title: 'Summarize with Verification',
      openai: {
        outputTemplate: WIDGET_RESOURCE_URI,
        widgetAccessible: true,
        toolInvocation: {
          invoking: 'Generating verifiable summary...',
          invoked: 'Summary complete with cryptographic proof.'
        }
      }
    },
    _meta: {
      'openai/outputTemplate': WIDGET_RESOURCE_URI
    }
  },
  async (params) => {
    const startTime = Date.now();
    const input = inputSchema.parse(params);

    try {
      // 1. Fetch content
      let source = { url: 'inline://text', content: input.text ?? '' };

      if (input.mode === 'file' && input.fileUrl) {
        const safeUrl = await validateRemoteFileUrl(input.fileUrl);
        console.log(`Fetching content from: ${safeUrl.toString()}`);
        const response = await fetchRemoteFile(safeUrl);
        if (!response.ok) {
          throw new Error(`Failed to fetch file: ${response.status} ${response.statusText}`);
        }
        const content = await readResponseBodyWithLimit(response, REMOTE_FILE_MAX_BYTES);
        source = {
          url: safeUrl.toString(),
          content
        };
      }

      if (!source.content) {
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
        usage ? 'gpt-4o-mini' : 'local-summarizer',
        ED25519_SECRET_KEY
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
      const downloadUrl = `${PUBLIC_URL}/download/${id}`;

      const stats = bundleStorage.getStats();
      console.log(`[cache] Stored ZIP bundle: ${id} (${(zipBuffer.length / 1024 / 1024).toFixed(2)}MB)`);
      console.log(`[cache] Cache stats: ${stats.totalEntries} bundles, ${(stats.totalBytes / 1024 / 1024).toFixed(2)}MB total`);

      // 6. Parse car.json for metadata
      const carData = JSON.parse(artifacts['car.json']);
      const signer = isSigned && carData.signer_public_key ? carData.signer_public_key : 'unsigned';
      const badgeStatus = isSigned ? 'Signed (ed25519)' : 'Unsigned - no cryptographic proof';

      const runtime = Date.now() - startTime;
      console.log(`Total runtime: ${runtime}ms`);

      return {
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

// Download endpoint
app.get('/download/:id', (req, res) => {
  // Add CORS headers for cross-origin downloads
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

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
