# Intelexta Local Usage Guide

Complete guide for building, running, and testing Intelexta components locally.

## Table of Contents

1. [Project Structure](#project-structure)
2. [Prerequisites](#prerequisites)
3. [Building Components](#building-components)
4. [CLI Verifier](#cli-verifier)
5. [Web Verifier](#web-verifier)
6. [Verifiable Summary Server](#verifiable-summary-server)
7. [Intelexta Desktop](#intelexta-desktop)
8. [Testing](#testing)
9. [Troubleshooting](#troubleshooting)

## Project Structure

```
intelexta/
├── src-tauri/                          # Main Tauri desktop app
│   ├── crates/
│   │   └── intelexta-verify/           # CLI verifier crate
│   │       └── src/main.rs
│   └── src/
│       ├── car.rs                      # CAR data structures
│       └── orchestrator.rs             # Workflow orchestration
├── target/release/                     # ⚠️ CORRECT binary location
│   └── intelexta-verify               # ← Use this CLI verifier
├── apps/
│   ├── web-verifier/                   # Browser-based verifier
│   │   ├── wasm-verify/                # Rust → WASM
│   │   │   ├── src/lib.rs
│   │   │   └── pkg/                    # Build artifacts (not served)
│   │   ├── public/pkg/                 # ← Served WASM location
│   │   └── src/
│   │       └── wasm/loader.ts          # WASM loader
│   └── verifiable-summary/
│       └── server/                     # MCP server for GPT
│           ├── src/
│           │   ├── provenance.ts       # CAR generation
│           │   └── index.ts            # MCP server
│           └── dist/                   # Compiled JS
└── scripts/
    └── build-wasm.sh                   # Copies WASM to public/pkg/
```

## Prerequisites

### Required Software

```bash
# Node.js 18+
node --version

# Rust (latest stable)
rustc --version
cargo --version

# wasm-pack
wasm-pack --version
# If not installed:
cargo install wasm-pack

# Tauri CLI (optional, for desktop app)
cargo install tauri-cli
```

### Install Dependencies

```bash
# Clone the repository
cd ~/Documents/codes/gaugefreedom/intelexta

# Install Node dependencies for all workspaces
npm install
```

## Building Components

### Build Everything

```bash
# From project root
npm run build
```

### Build Individual Components

```bash
# Build CLI verifier
cargo build --release --bin intelexta-verify

# Build web verifier WASM (and copy to public/)
cd apps/web-verifier
npm run build:wasm

# Build verifiable-summary server
cd apps/verifiable-summary/server
npm run build

# Build Intelexta Desktop
cargo tauri build
```

## CLI Verifier

### Location

⚠️ **Important**: The CLI verifier is located at:
```
/home/marcelo/Documents/codes/gaugefreedom/intelexta/target/release/intelexta-verify
```

**NOT** at `src-tauri/target/release/intelexta-verify` (that's an older build artifact).

### Build

```bash
cd ~/Documents/codes/gaugefreedom/intelexta
cargo build --release --bin intelexta-verify
```

### Usage

```bash
# Verify a CAR file
./target/release/intelexta-verify ~/Documents/teste/verifiable.car.zip

# Verify with JSON output
./target/release/intelexta-verify ~/Documents/teste/verifiable.car.zip --format json

# Verify a standalone car.json
./target/release/intelexta-verify ~/Documents/teste/car.json
```

### Expected Output

**Valid CAR**:
```
Intelexta CAR Verification
==================================================

CAR ID: car:f0585ecd9387166fd8b183ef9ec26129b960888bec96aa902f247454daa3da38

  ✓ File Integrity
  ✓ Hash Chain (1/1 checkpoints)
  ✓ Signatures (1 checkpoints)
  ✓ Content Integrity (3/3 provenance claims)

--------------------------------------------------
✓ VERIFIED: This CAR is cryptographically valid and has not been tampered with.
```

**Tampered CAR**:
```
  ✓ File Integrity
  ✓ Hash Chain (1/1 checkpoints)
  ✗ Signatures (1 checkpoints)
  ✗ Content Integrity (0/0 provenance claims)

--------------------------------------------------
✗ FAILED: Verification failed.
Error: Top-level body signature verification failed: Top-level body signature verification failed
```

### Create an Alias (Optional)

```bash
# Add to ~/.bashrc or ~/.zshrc
alias intelexta-verify='~/Documents/codes/gaugefreedom/intelexta/target/release/intelexta-verify'

# Then use:
intelexta-verify ~/Documents/teste/verifiable.car.zip
```

## Web Verifier

### Build WASM

```bash
cd ~/Documents/codes/gaugefreedom/intelexta/apps/web-verifier

# Build and copy WASM to public/pkg/
npm run build:wasm

# Or manually:
cd wasm-verify
wasm-pack build --target web
cp -r pkg ../public/
```

### Run Development Server

```bash
cd ~/Documents/codes/gaugefreedom/intelexta/apps/web-verifier
npm run dev

# Opens at http://localhost:5173
```

### Clear WASM Cache

If verification results seem outdated:

1. **Hard refresh**: Ctrl+Shift+R (Cmd+Shift+R on Mac)
2. **Incognito window**: Open http://localhost:5173 in incognito
3. **Clear site data**: DevTools → Application → Clear site data
4. **Rebuild WASM**: `npm run build:wasm` then restart dev server

### Production Build

```bash
npm run build
npm run preview  # Test production build locally
```

## Verifiable Summary Server

### Build

```bash
cd ~/Documents/codes/gaugefreedom/intelexta/apps/verifiable-summary/server
npm run build
```

### Run Locally

```bash
npm run dev
# Server starts on http://localhost:3000
```

### Test

```bash
npm test  # Run unit tests
```

### Use with ChatGPT

1. Start server: `npm run dev`
2. In ChatGPT GPT configuration, set MCP server URL to `http://localhost:3000`
3. Restart GPT conversation
4. Ask: "Create a verifiable summary of https://example.com/article"
5. Download the generated `.car.zip` file

### Generate CAR Programmatically

```javascript
import { generateProofBundle, generateKeypair } from './dist/provenance.js';
import fs from 'fs';

// Generate keypair (do this once)
const { publicKey, secretKey } = generateKeypair();
console.log('Store this securely:', secretKey);

// Generate CAR bundle
const { bundle, isSigned } = await generateProofBundle(
  {
    url: 'https://example.com/article',
    content: 'Article content here...'
  },
  'Summary of the article',
  'gpt-4o-mini',
  secretKey  // Optional: omit for unsigned bundle
);

// Save files
fs.writeFileSync('car.json', bundle['car.json']);
// bundle also contains attachment files
```

## Intelexta Desktop

### Build

```bash
cd ~/Documents/codes/gaugefreedom/intelexta
cargo tauri build
```

### Run Development

```bash
cargo tauri dev
```

### Generate CAR from Desktop

1. Launch Intelexta Desktop
2. Create a workflow (e.g., summarization task)
3. Run the workflow
4. Export CAR: File → Export → CAR Archive
5. Save to `~/Documents/teste/`

**Note**: Desktop now generates CARs with dual signatures (body + checkpoint) for tamper detection.

## Testing

### Run All Tests

```bash
# From project root
npm test
```

### Test Individual Components

```bash
# Verifiable-summary server tests
cd apps/verifiable-summary/server
npm test

# Web verifier tests
cd apps/web-verifier
npm test
```

### Manual Verification Testing

```bash
# 1. Generate a CAR
cd apps/verifiable-summary/server
npm run dev
# Use GPT to generate, or run programmatically

# 2. Verify original
./target/release/intelexta-verify ~/Documents/teste/verifiable.car.zip
# Should PASS

# 3. Tamper with it
unzip verifiable.car.zip -d test/
node -e "
const fs = require('fs');
const car = JSON.parse(fs.readFileSync('test/car.json'));
car.created_at = '1970-01-01T00:00:00Z';
fs.writeFileSync('test/car.json', JSON.stringify(car, null, 2));
"

# 4. Verify tampered
./target/release/intelexta-verify test/car.json
# Should FAIL with "Top-level body signature verification failed"
```

## Troubleshooting

### CLI Verifier Not Found

```bash
# ❌ Wrong location
./src-tauri/target/release/intelexta-verify

# ✅ Correct location
./target/release/intelexta-verify
```

If not found, rebuild:
```bash
cargo build --release --bin intelexta-verify
```

### Web Verifier Showing Old Results

**Symptoms**: Tampered CAR passes verification in browser but fails in CLI.

**Solution**:
```bash
# 1. Rebuild WASM
cd apps/web-verifier
npm run build:wasm

# 2. Restart dev server
npm run dev

# 3. Hard refresh browser (Ctrl+Shift+R)
# Or use incognito window
```

### Verifiable-Summary Not Generating New Signatures

**Symptoms**: Generated CARs have single signature like `["ed25519:..."]` instead of dual signatures.

**Solution**:
```bash
# 1. Rebuild
cd apps/verifiable-summary/server
npm run build

# 2. Restart server
npm run dev

# 3. Restart GPT conversation (important!)
```

### Tests Failing

```bash
# 1. Clean and rebuild
cd apps/verifiable-summary/server
rm -rf node_modules dist
npm install
npm run build

# 2. Run tests
npm test
```

### WASM Build Errors

```bash
# Ensure wasm-pack is installed
cargo install wasm-pack

# Rebuild WASM
cd apps/web-verifier/wasm-verify
wasm-pack build --target web

# Copy to public/
cp -r pkg ../public/
```

## Intelexta Desktop Dual Signatures ✅

**Status**: ✅ COMPLETED (2025-10-29)

**Files Updated**:
- `src-tauri/src/car.rs` (CAR generation logic at lines 467-480)

**Changes Made**:
1. Generate body signature (covers entire CAR with ID, without signatures)
2. Generate checkpoint signature (covers chain hashes)
3. Store both in signatures array: `["ed25519-body:...", "ed25519-checkpoint:..."]`

All CARs exported from Intelexta Desktop now have full tamper protection on top-level fields.

## Binary Locations Reference

| Component | Build Command | Binary Location |
|-----------|--------------|-----------------|
| CLI Verifier | `cargo build --release --bin intelexta-verify` | `target/release/intelexta-verify` |
| WASM Verifier | `npm run build:wasm` (in web-verifier) | `apps/web-verifier/public/pkg/*.wasm` |
| Verifiable-Summary | `npm run build` (in server/) | `apps/verifiable-summary/server/dist/` |
| Intelexta Desktop | `cargo tauri build` | `src-tauri/target/release/intelexta` |

## Quick Reference Commands

```bash
# Build everything
npm run build

# Build CLI verifier
cargo build --release --bin intelexta-verify

# Verify a CAR
./target/release/intelexta-verify ~/Documents/teste/file.car.zip

# Build web verifier WASM
cd apps/web-verifier && npm run build:wasm

# Run web verifier
cd apps/web-verifier && npm run dev

# Run verifiable-summary server
cd apps/verifiable-summary/server && npm run dev

# Run tests
npm test
```

## Next Steps

1. **Test the security fix**: Generate CARs from desktop, tamper with them, verify they fail
2. **Deploy verifiable-summary server**: See `DEPLOYMENT_SUMMARY.md`
3. **Deploy web verifier**: See `DEPLOYMENT_SUMMARY.md`
4. **Create GitHub release**: Tag and publish binaries

## Support

- **Issues**: https://github.com/gaugefreedom/intelexta/issues
- **Security**: See `SECURITY_ISSUE_TOP_LEVEL_SIGNATURE.md`
- **CAR Format**: See `apps/verifiable-summary/CAR_PROFILES.md`
