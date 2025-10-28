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
import JSZip from 'jszip';
import { randomUUID } from 'node:crypto';
import { config } from 'dotenv';

import { generateProofBundle } from './provenance.js';
import { summarize } from './summarizer.js';

// Load environment variables
config();

// Configuration
const PORT = parseInt(process.env.PORT || '3000', 10);
const PUBLIC_BASE_URL = process.env.PUBLIC_BASE_URL || `http://localhost:${PORT}`;
const ED25519_SECRET_KEY = process.env.ED25519_SECRET_KEY;

// ============================================================================
// Initialize MCP Server
// ============================================================================

const server = new McpServer({
  name: 'verifiable-summary',
  version: '0.1.0'
});

// In-memory store for ZIP bundles (TODO: add TTL cleanup)
const zipStore = new Map<string, { buffer: Buffer; createdAt: number }>();

// Clean up old ZIPs every hour
setInterval(() => {
  const now = Date.now();
  const ONE_HOUR = 3600000;

  for (const [id, { createdAt }] of zipStore.entries()) {
    if (now - createdAt > ONE_HOUR) {
      zipStore.delete(id);
      console.log(`Cleaned up expired ZIP: ${id}`);
    }
  }
}, 3600000);

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
  <div id="root"><div class="loading">Loading...</div></div>
  <script>
    (function() {
      const subscribe = (callback) => {
        window.addEventListener('openai:set_globals', callback);
        return () => window.removeEventListener('openai:set_globals', callback);
      };

      const getSnapshot = () => window.openai?.toolOutput;

      let currentOutput = getSnapshot();

      const render = () => {
        const toolOutput = getSnapshot();
        if (!toolOutput) {
          document.getElementById('root').innerHTML = '<div class="loading">Generating summary...</div>';
          return;
        }

        const { summary, car, meta } = toolOutput;

        const badgeClass = car.valid ? 'verified' : 'unsigned';
        const badgeLabel = car.valid ? '✓ Verified' : '⚠ Unsigned';
        const signerDisplay = car.valid ? `${car.signer.slice(0, 24)}...` : 'unsigned';
        const signerTitle = car.valid ? 'Click to copy signer' : 'Unsigned bundle - no signer';
        const signerOnclick = car.valid ? `onclick="navigator.clipboard.writeText('${car.signer}')"` : '';

        document.getElementById('root').innerHTML = \`
          <div class="summary-card">
            <div class="header">
              <h3>Verifiable Summary</h3>
              <span class="badge \${badgeClass}">
                \${badgeLabel}
              </span>
            </div>

            <div class="summary-content">
              <p>\${summary.replace(/\\n/g, '<br>')}</p>
            </div>

            <div class="verification-info">
              <div class="info-row">
                <span class="label">Signer:</span>
                <code class="value" \${signerOnclick} title="\${signerTitle}">
                  \${signerDisplay}
                </code>
              </div>
              <div class="info-row">
                <span class="label">Tree Hash:</span>
                <code class="value" title="\${car.hash}">\${car.hash.slice(0, 20)}...</code>
              </div>
              <div class="info-row">
                <span class="label">Processed:</span>
                <span class="value">\${meta.bytes_processed.toLocaleString()} bytes in \${meta.runtime_ms}ms</span>
              </div>
            </div>

            <div class="actions">
              <button class="btn btn-primary" onclick="window.openai.openExternal({ href: '\${car.download_url}' })">
                Download CAR Bundle
              </button>
            </div>
          </div>
        \`;
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
        console.log(`Fetching content from: ${input.fileUrl}`);
        const response = await fetch(input.fileUrl);
        if (!response.ok) {
          throw new Error(`Failed to fetch file: ${response.status} ${response.statusText}`);
        }
        source = {
          url: input.fileUrl,
          content: await response.text()
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
      zipStore.set(id, { buffer: zipBuffer, createdAt: Date.now() });
      const downloadUrl = `${PUBLIC_BASE_URL}/download/${id}`;

      // 6. Parse manifest for metadata
      const manifest = JSON.parse(artifacts['manifest.json']);
      const receipt = JSON.parse(artifacts['receipts/ed25519.json']);
      const signer = isSigned && receipt.publicKey ? receipt.publicKey : 'unsigned';
      const badgeStatus = isSigned ? 'Signed (ed25519)' : 'Unsigned - no cryptographic proof';

      const runtime = Date.now() - startTime;
      console.log(`Total runtime: ${runtime}ms`);

      return {
        content: [{
          type: 'text',
          text: `Verifiable summary generated.\n\nTree Hash: ${manifest.treeHash}\nSigner: ${signer}\nReceipt: ${badgeStatus}\nRuntime: ${runtime}ms`
        }],
        structuredContent: {
          summary,
          car: {
            id,
            valid: isSigned,
            signer,
            hash: manifest.treeHash,
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

// Health check
app.get('/health', (_req, res) => {
  res.json({ status: 'ok', timestamp: new Date().toISOString() });
});

// Download endpoint
app.get('/download/:id', (req, res) => {
  const entry = zipStore.get(req.params.id);

  if (!entry) {
    return res.status(404).json({ error: 'Bundle not found or expired' });
  }

  res.setHeader('Content-Type', 'application/zip');
  res.setHeader('Content-Disposition', 'attachment; filename="verifiable.car.zip"');
  res.send(entry.buffer);
});

// MCP endpoint wired through the official streamable HTTP transport
app.post('/mcp', express.json(), async (req, res) => {
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
app.listen(PORT, () => {
  console.log(`
╔═══════════════════════════════════════════════════════════╗
║  Verifiable Summary MCP Server                            ║
╠═══════════════════════════════════════════════════════════╣
║  URL: ${PUBLIC_BASE_URL.padEnd(49)} ║
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
