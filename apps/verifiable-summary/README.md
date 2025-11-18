# Verifiable Summary - OpenAI Apps SDK

MCP server that provides a `summarize_content` tool for ChatGPT, generating summaries with cryptographic proof bundles.

## Features

- 📝 **Multiple Summary Styles**: TL;DR, bullet points, or outlines
- 🔐 **Cryptographic Proofs**: Ed25519 signatures for verification
- 📦 **CAR Bundles**: Download complete proof packages as ZIP files
- 🎨 **Interactive Widget**: Skybridge UI rendered in ChatGPT
- 🚀 **Flexible Summarization**: Local extraction or OpenAI API

## Architecture

```
apps/verifiable-summary/
├── server/              # MCP server (TypeScript)
│   ├── src/
│   │   ├── index.ts          # MCP server + Express
│   │   ├── provenance.ts     # Cryptographic utilities
│   │   └── summarizer.ts     # Summarization logic
│   └── scripts/
│       └── generate-keypair.js
└── web/                 # React widget (TODO)
```

## Quick Start

### 1. Install Dependencies

```bash
cd apps/verifiable-summary/server
npm install
```

### 2. Generate Signing Key

```bash
npm run keygen
```

Copy the `ED25519_SECRET_KEY` to your `.env` file.

### 3. Configure Environment

```bash
cp .env.example .env
# Edit .env with your configuration
```

Required variables:
- `PORT` - Server port (default: 3000)
- `PUBLIC_URL` - Public URL for downloads
- `ED25519_SECRET_KEY` - Signing key from step 2
- `OPENAI_API_KEY` - (Optional) For cloud summarization

### 4. Start Development Server

```bash
npm run dev
```

### 5. Expose with ngrok

```bash
ngrok http 3000
```

### 6. Add to ChatGPT

1. Open ChatGPT Settings → Developer Mode
2. Click "Add Connector"
3. Enter your ngrok URL
4. Save

### 7. Test the Tool

In ChatGPT:
```
Summarize this: [paste your text]
```

The widget will appear with:
- Generated summary
- Verification status
- Cryptographic signatures
- Download button for CAR bundle

## Tool Parameters

### `summarize_content`

**Input**:
```typescript
{
  mode: "text" | "file",    // Content source
  text?: string,             // Direct text content
  fileUrl?: string,          // URL to fetch content
  style: "tl;dr" | "bullets" | "outline"  // Summary format
}
```

**Output**:
```typescript
{
  summary: string,           // Generated summary
  car: {
    id: string,             // Bundle ID
    valid: boolean,         // Verification status
    signer: string,         // Public key (base64)
    hash: string,           // Tree hash (hex)
    download_url: string    // ZIP download URL
  },
  meta: {
    bytes_processed: number,
    runtime_ms: number
  }
}
```

## CAR Bundle Structure

Downloaded ZIPs contain:

```
verifiable.car.zip
├── summary.md              # Generated summary
├── sources.jsonl           # Source metadata
├── transcript.json         # Workflow steps
├── manifest.json           # File hashes + tree hash
└── receipts/
    └── ed25519.json        # Cryptographic signature
```

### Verification

Bundles can be verified with the Intelexta web verifier:

```bash
# Upload to: http://localhost:5173 (if running web-verifier)
```

Or verify manually:
1. Recompute file hashes (SHA-256)
2. Verify tree hash = SHA256(sorted hashes joined by newline)
3. Verify signature using Ed25519 public key

## Summarization Strategies

### Local (Default)
- **Speed**: Instant
- **Quality**: Basic extraction
- **Cost**: Free

Uses simple text extraction:
- TL;DR: First 1-2 sentences
- Bullets: First sentence per paragraph
- Outline: Structured headings

### OpenAI API (Optional)
- **Speed**: 1-3 seconds
- **Quality**: High
- **Cost**: ~$0.01 per summary

Set `OPENAI_API_KEY` to enable.
Uses `gpt-4o-mini` for cost-effectiveness.

## Development

### Project Scripts

```bash
npm run dev       # Start with hot reload
npm run build     # Compile TypeScript
npm start         # Run production build
npm test          # Run tests (TODO)
npm run keygen    # Generate Ed25519 keypair
```

### Testing Locally

```bash
# Terminal 1: Start server
npm run dev

# Terminal 2: Test tool directly (TODO: add test script)
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"tool":"summarize_content","params":{"text":"Test content","style":"tl;dr"}}'
```

## Roadmap

### Phase 1: Core Server ✅
- [x] MCP server setup
- [x] Provenance utilities
- [x] Summarization (local + OpenAI)
- [x] ZIP bundle generation
- [x] Basic Skybridge widget

### Phase 2: Enhanced Widget
- [ ] Build React component
- [ ] Styled UI with Tailwind
- [ ] Re-verify button
- [ ] Copy-to-clipboard actions
- [ ] Error states

### Phase 3: Testing & Polish
- [ ] Unit tests for provenance
- [ ] Integration tests
- [ ] End-to-end ChatGPT tests
- [ ] Performance optimization
- [ ] Documentation

### Phase 4: Advanced Features
- [ ] OAuth2 authentication
- [ ] Batch summarization
- [ ] Custom models (local LLMs)
- [ ] Verification API endpoint
- [ ] Analytics dashboard

## Regression: Escaped Summary Content

To confirm that summaries render safely (and `<script>` tags do not execute inside the widget):

1. Start the MCP server locally:
   ```bash
   cd apps/verifiable-summary/server
   npm run dev
   ```
2. Visit the widget resource directly at [http://localhost:3000/widget/verifiable-summary](http://localhost:3000/widget/verifiable-summary).
3. Open your browser devtools console and run:
   ```js
   window.openai = {
     toolOutput: {
       summary: "<script>window.__xss_executed = true</script>\nSecond line",
       car: {
         valid: true,
         signer: 'abcdefghijklmnopqrstuvwxyz123456',
         hash: '0123456789abcdef0123456789abcdef01234567',
         download_url: 'https://example.com/fake.car'
       },
       meta: { bytes_processed: 1234, runtime_ms: 56 }
     },
     openExternal: ({ href }) => console.log('openExternal called with', href)
   };
   window.dispatchEvent(new Event('openai:set_globals'));

    // Optionally simulate the structuredContent payload used by the live tool.
    window.openai.toolOutput = {
      structuredContent: {
        summary: "<script>window.__xss_executed = true</script>\nSecond line",
        car: {
          valid: true,
          signer: 'abcdefghijklmnopqrstuvwxyz123456',
          hash: '0123456789abcdef0123456789abcdef01234567',
          download_url: 'https://example.com/fake.car'
        },
        meta: { bytes_processed: 1234, runtime_ms: 56 }
      }
    };
    window.dispatchEvent(new Event('openai:set_globals'));
   ```
4. Verify that no alert fires and `window.__xss_executed` remains `undefined`. The `<script>` tag should render as plain text with line breaks preserved.

This manual regression ensures summary HTML is escaped before rendering.

## Troubleshooting

### "Bundle not found"
- ZIPs expire after 1 hour
- Re-run the tool to generate a new bundle

### "Failed to fetch file"
- Ensure `fileUrl` is publicly accessible
- Check CORS settings if fetching from web

### "Unsigned bundle"
- Set `ED25519_SECRET_KEY` in `.env`
- Run `npm run keygen` to generate a key

### Widget not rendering
- Verify ngrok URL is correct
- Check browser console for errors
- Ensure MCP connector is active

## Contributing

This is part of the Intelexta monorepo. See main repository docs for contributing guidelines.

## License

MIT - See LICENSE in repository root.
