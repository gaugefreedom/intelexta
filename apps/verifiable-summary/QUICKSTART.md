# Quick Start Guide - Verifiable Summary

Get up and running with the Verifiable Summary MCP server in 5 minutes.

## Prerequisites

- Node.js 20+ installed
- ngrok account (free tier works)
- ChatGPT Plus subscription (for Developer Mode)

## Step 1: Install Dependencies

```bash
cd apps/verifiable-summary/server
npm install
```

## Step 2: Start the Server

```bash
npm run dev
```

You should see:

```
╔═══════════════════════════════════════════════════════════╗
║  Verifiable Summary MCP Server                            ║
╠═══════════════════════════════════════════════════════════╣
║  URL: http://localhost:3000                               ║
║  Port: 3000                                               ║
║  Signing: Enabled                                         ║
╚═══════════════════════════════════════════════════════════╝

Tools:
  • summarize_content - Generate verifiable summaries with proof
```

## Step 3: Expose with ngrok

In a new terminal:

```bash
ngrok http 3000
```

Copy the **Forwarding URL** (e.g., `https://abcd-1234.ngrok-free.app`)

## Step 4: Add Connector to ChatGPT

1. Go to https://chatgpt.com
2. Click your profile → **Settings**
3. Navigate to **Developer Mode**
4. Click **Add Connector**
5. Paste your ngrok URL
6. Click **Save**

The connector should show as "Active" with a green dot.

## Step 5: Test the Tool

In ChatGPT, type:

```
Summarize this in bullets: Artificial intelligence is revolutionizing healthcare.
Machine learning models can now diagnose diseases with accuracy comparable to expert
physicians. AI-powered drug discovery is accelerating the development of new treatments.
Personalized medicine is becoming a reality through AI analysis of genetic data.
```

You should see:

1. ChatGPT invokes the `summarize_content` tool
2. A widget appears showing:
   - ✓ Verified badge
   - Your summary in bullet points
   - Cryptographic verification info
   - Download button
3. Click "Download CAR Bundle" to get the proof package

## Step 6: Verify the Bundle

Download the ZIP and verify it:

```bash
unzip verifiable.car.zip
ls -l
# Should show: summary.md, sources.jsonl, transcript.json, manifest.json, receipts/

# Verify hashes
sha256sum summary.md sources.jsonl transcript.json
cat manifest.json | jq '.files'
```

## Troubleshooting

### "Tool not found"
- Check ngrok is running
- Verify connector URL in ChatGPT settings
- Restart ChatGPT session

### "Failed to generate summary"
- Check server logs (`npm run dev` terminal)
- Verify `.env` file exists with ED25519_SECRET_KEY
- Try with shorter text first

### "Bundle not found"
- Bundles expire after 1 hour
- Re-run the tool to generate a new one

### Widget not rendering
- Check browser console for errors
- Verify ngrok URL is accessible
- Try disconnecting and reconnecting the connector

## What's Next?

### Try Different Styles

```
Summarize this as tl;dr: [text]
Summarize this in bullets: [text]
Summarize this as outline: [text]
```

### Try File URLs

```
Summarize this file: https://example.com/article.txt
```

### Enable OpenAI Summarization

Add to `.env`:
```
OPENAI_API_KEY=sk-...
```

Restart the server. Now summaries will use GPT-4o-mini for higher quality.

### Verify Cryptographic Proof

Use the Intelexta web verifier (if running):

```bash
# In another terminal
cd apps/web-verifier
npm run dev
```

Then upload your downloaded `.car.zip` to http://localhost:5173

## Advanced Usage

### Custom Port

```bash
PORT=8080 npm run dev
```

Don't forget to update ngrok and ChatGPT connector URL!

### Production Mode

```bash
npm run build
npm start
```

### Generate New Keypair

```bash
npm run keygen
```

Copy the secret key to `.env`

### Check Health

```bash
curl http://localhost:3000/health
```

## Support

- **README**: See `apps/verifiable-summary/README.md` for full docs
- **Implementation**: See `IMPLEMENTATION.md` for technical details
- **Issues**: https://github.com/gaugefreedom/intelexta/issues

## Success Checklist

- [ ] Server running on localhost:3000
- [ ] ngrok forwarding to server
- [ ] Connector added in ChatGPT
- [ ] Tool invoked successfully
- [ ] Widget rendered correctly
- [ ] Bundle downloaded
- [ ] Hashes verified manually

🎉 **Congratulations!** You now have a working verifiable summary system with cryptographic proofs!
