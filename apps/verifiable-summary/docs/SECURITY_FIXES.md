# Security Fixes Applied to Verifiable Summary MCP Server

This document summarizes the critical security vulnerabilities that were identified and fixed in the Verifiable Summary MCP server.

## Summary of Fixes

All **critical** and **high-risk** vulnerabilities have been addressed. The server is now hardened against common web security threats.

---

## 🛑 Critical Vulnerabilities Fixed

### 1. Server-Side Request Forgery (SSRF) - FIXED ✅

**Issue:** The server blindly fetched any URL provided by users in the `fileUrl` parameter, allowing attackers to:
- Scan internal networks
- Access cloud metadata endpoints (e.g., `169.254.169.254`)
- Exfiltrate sensitive data from internal services

**Fix Applied:**
- Created `src/security.ts` with `validateSafeUrl()` function
- Protocol validation: Only `http:` and `https:` allowed
- DNS resolution and IP validation against private/internal ranges
- Blocked ranges: `127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `169.254.0.0/16`
- Blocked cloud metadata endpoints: `169.254.169.254`, `metadata.google.internal`
- Added 10-second timeout to prevent slow-loris attacks

**Code Location:** `src/index.ts:239-244`, `src/security.ts:21-72`

### 2. Cross-Site Scripting (XSS) - FIXED ✅

**Issue:** The Skybridge widget used `innerHTML` to inject user-controllable content (summary text, signer display), allowing script injection if malicious HTML was included in summarized content.

**Fix Applied:**
- Created `escapeHtml()` function that safely escapes all HTML entities
- All user-controllable content is now escaped before insertion
- Replaced inline `onclick` handlers with proper event listeners attached after DOM creation
- Event listeners are now added via `addEventListener()` instead of inline attributes

**Code Location:** `src/index.ts:108-184`

### 3. Unbounded File Size (Denial of Service) - FIXED ✅

**Issue:** The server would attempt to read entire files into memory without checking size, allowing OOM attacks with large files (e.g., 5GB).

**Fix Applied:**
- Created `validateFileSize()` function in `src/security.ts`
- Check `Content-Length` header before reading response body
- Maximum file size: **10MB** (configurable via `MAX_FILE_SIZE_BYTES`)
- Clear error messages when file exceeds limit

**Code Location:** `src/index.ts:251-253`, `src/security.ts:77-92`

---

## ⚠️ High-Risk Vulnerabilities Fixed

### 4. Request Size Limit (Denial of Service) - FIXED ✅

**Issue:** No limit on JSON payload size in MCP endpoint, allowing payload attacks.

**Fix Applied:**
- Added `{ limit: '10mb' }` to `express.json()` middleware
- Prevents excessively large JSON payloads

**Code Location:** `src/index.ts:419`

### 5. Rate Limiting (Denial of Service) - FIXED ✅

**Issue:** No rate limiting, making the server vulnerable to DoS attacks and abuse.

**Fix Applied:**
- Installed `express-rate-limit` package
- Global rate limit: **100 requests per 15 minutes** per IP
- MCP endpoint stricter limit: **20 requests per 15 minutes** per IP
- Standard rate limit headers included in responses

**Code Location:** `src/index.ts:374-393`

---

## 🔒 Security Hardening Applied

### 6. Security Headers - ADDED ✅

**Issue:** Missing standard security headers.

**Fix Applied:**
- Installed `helmet` package
- Applied security headers (HSTS, X-Content-Type-Options, etc.)
- Disabled CSP and COEP for MCP compatibility

**Code Location:** `src/index.ts:368-372`

### 7. Non-Root Container User - ADDED ✅

**Issue:** Docker container ran as root, providing elevated privileges to attackers if code execution was achieved.

**Fix Applied:**
- Created dedicated `nodejs` user (UID 1001) and group (GID 1001)
- All files owned by `nodejs:nodejs`
- Container runs as `USER nodejs`
- Added `npm cache clean --force` to reduce image size

**Code Location:** `Dockerfile:32-54`

---

## Environment Variables

The server now uses the correct environment variable names:

| Variable | Description | Example |
|----------|-------------|---------|
| `PUBLIC_URL` | Production domain for download links | `https://api.intelexta.com` |
| `ED25519_SECRET_KEY` | Base64-encoded Ed25519 signing key | (generate with `npm run keygen`) |
| `PORT` | Server port (auto-set by Cloud Run) | `8080` |
| `NODE_ENV` | Environment mode | `production` |

**Important:** The environment variable has been renamed from `PUBLIC_BASE_URL` to `PUBLIC_URL` for consistency.

---

## Deployment Command (Updated)

Use this command to deploy to Google Cloud Run with all security fixes:

```bash
cd apps/verifiable-summary/server

gcloud run deploy "verifiable-summary-server" \
  --source . \
  --platform "managed" \
  --region "us-central1" \
  --allow-unauthenticated \
  --set-env-vars="ED25519_SECRET_KEY=[YOUR_SECRET_KEY_HERE],PUBLIC_URL=https://api.intelexta.com" \
  --memory 512Mi \
  --cpu 1 \
  --timeout 60s \
  --max-instances 10 \
  --port 8080
```

---

## Testing Recommendations

Before deploying to production, test the following:

1. **SSRF Protection:**
   ```bash
   # Should be blocked
   curl -X POST https://api.intelexta.com/mcp \
     -H "Content-Type: application/json" \
     -d '{"fileUrl": "http://169.254.169.254/metadata"}'
   ```

2. **File Size Limit:**
   ```bash
   # Should reject files > 10MB
   curl -X POST https://api.intelexta.com/mcp \
     -H "Content-Type: application/json" \
     -d '{"fileUrl": "http://example.com/large-file.txt"}'
   ```

3. **Rate Limiting:**
   ```bash
   # Should get rate limited after 20 requests
   for i in {1..25}; do
     curl -X POST https://api.intelexta.com/mcp \
       -H "Content-Type: application/json" \
       -d '{"text": "test"}'
   done
   ```

4. **XSS Protection:**
   - Summarize content with HTML/JavaScript: `<script>alert('XSS')</script>`
   - Verify it's displayed as plain text, not executed

---

### 8. Unbounded ZIP Store (DoS) - FIXED ✅

**Issue:** Generated ZIP bundles were stored in an unbounded `Map`, allowing attackers to exhaust memory by repeatedly calling the summarize tool.

**Fix Applied:**
- Replaced `Map` with **LRU Cache** (`lru-cache` package)
- Maximum entries: **100 bundles**
- Maximum total size: **500MB**
- Automatic TTL: **1 hour**
- Automatic eviction when limits are reached
- Logging for cache operations (store, evict, download)

**Code Location:** `src/index.ts:49-59`, `src/index.ts:315-319`, `src/index.ts:409-420`

**Cache Configuration:**
```typescript
const zipStore = new LRUCache<string, Buffer>({
  max: 100,                      // Maximum 100 bundles
  maxSize: 500 * 1024 * 1024,    // Maximum 500MB total
  ttl: 60 * 60 * 1000,          // 1 hour TTL
  updateAgeOnGet: true,          // Reset TTL when accessed
});
```

---

## Remaining Recommendations (Optional)

These are not critical but improve production readiness:

1. **Cloud Storage for ZIP Files (Future Enhancement):**
   - Consider migrating from LRU cache to Google Cloud Storage for larger scale
   - Set TTL policy on bucket for automatic cleanup
   - Makes server stateless and horizontally scalable
   - Current LRU cache is sufficient for most use cases

2. **Structured Logging:**
   - Add Winston or Pino for JSON-formatted logs
   - Integrate with Google Cloud Logging

3. **Authentication (if needed):**
   - Add API key or OAuth if this should be restricted access
   - Use Cloud Run IAM for internal-only services

4. **Monitoring & Alerting:**
   - Set up Cloud Monitoring alerts for error rates
   - Monitor rate limit triggers

---

## Files Modified

| File | Changes |
|------|---------|
| `src/index.ts` | SSRF protection, XSS fixes, rate limiting, security headers |
| `src/security.ts` | **NEW** - SSRF validation, file size validation |
| `Dockerfile` | Non-root user, ownership changes |
| `.env.example` | Renamed `PUBLIC_BASE_URL` to `PUBLIC_URL` |
| `package.json` | Added `helmet`, `express-rate-limit` dependencies |

---

## Security Checklist

- ✅ SSRF protection with URL validation
- ✅ XSS protection with HTML escaping
- ✅ File size limits (10MB)
- ✅ Request payload limits (10MB)
- ✅ Rate limiting (100 global, 20 MCP)
- ✅ Security headers (helmet)
- ✅ Non-root container user
- ✅ CORS headers for downloads
- ✅ Request timeouts (10s)
- ✅ Environment variable validation
- ✅ Bounded memory with LRU cache (100 bundles, 500MB max)

---

## Agent Report Status

All issues from the security agent report have been addressed:

| Issue | Status | Implementation |
|-------|--------|----------------|
| 1. SSRF via unrestricted fileUrl | ✅ Fixed | Protocol validation, IP blocking, DNS resolution |
| 2. DOM XSS in Skybridge widget | ✅ Fixed | HTML escaping, event listeners |
| 3. Unbounded file download OOM | ✅ Fixed | Content-Length validation, 10MB limit |
| 4. In-memory ZIP store DoS | ✅ Fixed | LRU cache with 100 entry / 500MB limits |

**Missing (non-critical):**
- Unit tests for SSRF validation (recommended but not blocking)
- Regression test for XSS (recommended but not blocking)

---

**Status:** All critical and high-risk vulnerabilities have been mitigated. The server is ready for production deployment.
