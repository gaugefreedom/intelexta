The directory guidelines explicitly prohibit selling **digital products/services/subscriptions** inside an app (including freemium upsells), so treat this MCP app as **free** and don’t add “upgrade to Pro” language in the ChatGPT surface. ([OpenAI Developers][1])

Here’s a cleaned “agent prompt” you can paste (it includes your intent, plus a couple corrections):

Goal: Make apps/verifiable-summary submission-ready for the ChatGPT directory and close Codex security blockers.

Do:

Remove remote file support entirely

Delete mode, fileUrl from the tool schema.

Remove fetchRemoteFile, validateRemoteFileUrl, and any SSRF-related utilities.

Tool input becomes: { text: string, style?: "tl;dr"|"bullets"|"outline", include_source?: boolean }.

Privacy default

include_source=false by default.

When include_source=false, do not embed full input text in the CAR bundle.

Store only: input_sha256, bytes, and a short preview (e.g. first 200 chars) inside attachments/metadata.

(If any size-limited reads remain) Fix size-limit enforcement

Ensure any “read with limit” uses streaming enforcement always (never response.text()).

Widget resource metadata

In registerResource, ensure the returned contents item includes:

mimeType: "text/html+skybridge"

_meta.openai/widgetDomain = "https://chatgpt.com"

_meta.openai/widgetPrefersBorder = true

_meta.openai/widgetCSP = { connect_domains:["https://chatgpt.com"], resource_domains:["https://*.oaistatic.com"] }

Tool descriptor metadata

In registerTool:

add _meta["openai/toolInvocation/invoking"] and _meta["openai/toolInvocation/invoked"]

set annotations: { readOnlyHint:true, idempotentHint:true, openWorldHint:false, destructiveHint:false }

declare securitySchemes: [{ type: "noauth" }]

UI copy

Change badge text from “Verified” → “Signed” when Ed25519 signature present.

Durable downloads

Replace in-memory TTL storage with durable storage (object storage or persisted disk on Cloud Run volume).

Ensure downloads don’t break on restarts; set a retention window suitable for review.

Add Cache-Control: no-store to /download/:id.

