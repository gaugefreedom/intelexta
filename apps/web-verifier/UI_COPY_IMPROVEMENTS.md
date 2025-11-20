# Web Verifier UI Copy Improvements

## Overview

This document specifies wording and microcopy improvements for the Intelexta Web Verifier to better communicate:
- Intelexta is a **proof layer for AI-assisted work**
- A **CAR** is a **signed receipt of a run**, not necessarily a full transcript
- **Verified** means **cryptographic validity**, not document correctness

**Implementation Note**: These changes focus on copy, labels, tooltips, and small layout tweaks. No data model or verification logic changes.

---

## Part 1: Hero / Header Area

### Current State
**File**: `src/components/Verifier.tsx` lines 254-260

**Current**:
```tsx
<h1>Workflow Proof Verifier</h1>
<p>Validate signed workflow archives directly in your browser. Upload a CAR bundle exported from Intelexta or drop a JSON transcript to preview steps, prompts, and outputs.</p>
```

### Required Changes

**Title**: Keep as-is
```tsx
<h1>Workflow Proof Verifier</h1>
```

**Subtitle**: Update to emphasize receipt verification
```tsx
<p className="text-base text-slate-300 sm:text-lg">
  Verify signed receipts (CARs) for AI-assisted workflows. Files are processed in your browser and never uploaded.
</p>
```

---

## Part 2: Dropzone Area

### Current State
**File**: `src/components/Verifier.tsx` lines 273-280

**Current**:
```tsx
<p>Drag & drop a .car.json or .car.zip file here</p>
<p>Supports Intelexta signed CAR archives and JSON transcripts. Files stay in the browser and are never uploaded.</p>
```

### Required Changes

**Drag prompt**: Update to use "receipt" terminology
```tsx
<p className="text-lg font-medium text-slate-100">
  {isDragActive
    ? 'Release to verify your receipt'
    : 'Drag & drop a .car.json or .car.zip receipt here'}
</p>
```

**Helper text**: Simplify (browser privacy already in main subtitle)
```tsx
<p className="mt-2 max-w-md text-sm text-slate-400">
  Supports signed CAR receipts and JSON-only formats.
</p>
```

---

## Part 3: Status Banner (Success State)

### Current State
**File**: `src/components/Verifier.tsx` lines 232-250 (statusMessage)

### Required Changes

Add a **prominent success callout** above the tabs when `status === 'success'`.

**Location**: After `StatusBanner`, before view mode toggle (around line 288)

**New Component**:
```tsx
{result && status === 'success' && (
  <div className="rounded-lg border border-emerald-500/40 bg-emerald-500/10 p-6">
    <div className="flex items-start gap-3">
      <CheckCircle2 className="h-6 w-6 flex-shrink-0 text-emerald-400 mt-0.5" />
      <div>
        <h3 className="text-lg font-semibold text-emerald-100 mb-2">Receipt verified</h3>
        <p className="text-sm text-emerald-200 leading-relaxed">
          This Content-Addressable Receipt (CAR) is cryptographically valid. Hash chains and signatures are consistent.
          {result.summary && result.summary.provenance_total > result.summary.provenance_verified && (
            <span className="block mt-2">
              Some referenced content is not included in this bundle and can only be verified by hash.
            </span>
          )}
        </p>
      </div>
    </div>
  </div>
)}
```

---

## Part 4: Last File Status

### Current State
**File**: `src/components/Verifier.tsx` lines 281-285

**Current**:
```tsx
<p className="mt-4 rounded-full border border-slate-700 bg-slate-800/80 px-5 py-1 text-xs uppercase tracking-wide text-slate-300">
  Last file: {droppedFileName}
</p>
```

### Required Changes

Make it a **compact status line** with verification result:
```tsx
{droppedFileName && result && (
  <p className="mt-4 rounded-full border border-slate-700 bg-slate-800/80 px-5 py-1 text-xs text-slate-300 flex items-center gap-2">
    <span className="font-medium">Last receipt:</span>
    <span>{droppedFileName}</span>
    <span className="text-slate-500">·</span>
    <span>{validation.kind === 'json' ? 'JSON receipt-only' : 'ZIP bundle'}</span>
    <span className="text-slate-500">·</span>
    <span className={status === 'success' ? 'text-emerald-400' : 'text-rose-400'}>
      {status === 'success' ? 'Verified' : 'Failed'}
    </span>
  </p>
)}
```

---

## Part 5: Verification Tab - WorkflowViewer

### Current State
**File**: `src/components/WorkflowViewer.tsx`

### Required Changes

#### Step Labels (lines ~40-80)

**Current step names**:
- File Integrity
- Hash Chain
- Signatures
- Content Integrity

**Update to**:
```tsx
const steps = [
  {
    key: 'file',
    label: 'Hash chain integrity',
    description: 'Checks that the sequence of checkpoints forms an unbroken hash chain.',
    ...
  },
  {
    key: 'signatures',
    label: 'Signature validation',
    description: 'Verifies Ed25519 signatures for this run and its checkpoints.',
    ...
  },
  {
    key: 'provenance',
    label: 'Provenance references',
    description: 'Verifies content hashes for provenance claims.',
    ...
  },
  {
    key: 'attachments',
    label: 'Attachment integrity',
    description: 'Checks integrity of bundled content files.',
    ...
  }
];
```

#### Step Details - Provenance

When `provenance_verified < provenance_total`, show status as `Info` (blue) instead of error:

```tsx
// In step detail rendering
{step.key === 'provenance' && summary.provenance_total > 0 && summary.provenance_verified < summary.provenance_total && (
  <p className="text-sm text-blue-200 mt-2">
    Provenance claims: {summary.provenance_verified} / {summary.provenance_total} re-checked locally (claims reference content not included in this bundle).
  </p>
)}
```

#### Step Details - Attachments

For JSON-only receipts:
```tsx
{step.key === 'attachments' && summary.attachments_total === 0 && (
  <p className="text-sm text-slate-400 mt-2">
    Attachment files: 0 / 0 (this receipt does not bundle content files).
  </p>
)}
```

---

## Part 6: Verification Tab - MetadataCard

### Current State
**File**: `src/components/MetadataCard.tsx`

### Required Changes

**Card Title** (line ~20):
```tsx
<h2 className="text-lg font-semibold text-slate-100">Verification Summary</h2>
```

**Status Section** - Add explicit "Status" field at top:
```tsx
<div className="space-y-3">
  {/* Status */}
  <div className="rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
    <dt className="text-xs uppercase tracking-wide text-slate-400">Status</dt>
    <dd className={`mt-1 text-sm font-medium ${
      report.status === 'verified' ? 'text-emerald-400' : 'text-rose-400'
    }`}>
      {report.status === 'verified' ? 'Cryptographically valid' : 'Verification failed'}
    </dd>
  </div>

  {/* Existing fields: CAR ID, Run ID, Signer, Timestamp */}
  ...
</div>
```

**Coverage Section** - Add after existing fields:
```tsx
{report.summary && (
  <div className="mt-6">
    <h3 className="text-sm font-semibold text-slate-200 mb-3">Coverage</h3>
    <div className="space-y-2 text-sm">
      <div className="flex justify-between">
        <span className="text-slate-400">Checkpoints:</span>
        <span className="text-slate-200 font-mono">
          {report.summary.checkpoints_verified} / {report.summary.checkpoints_total}
        </span>
      </div>
      <div className="flex justify-between">
        <span className="text-slate-400">Provenance with local artifacts:</span>
        <span className="text-slate-200 font-mono">
          {report.summary.provenance_verified} / {report.summary.provenance_total}
        </span>
      </div>
      <div className="flex justify-between">
        <span className="text-slate-400">Attachments in this bundle:</span>
        <span className="text-slate-200 font-mono">
          {report.summary.attachments_total}
        </span>
      </div>
    </div>

    {/* Partial coverage explanations */}
    {report.summary.provenance_total > 0 && report.summary.provenance_verified < report.summary.provenance_total && (
      <p className="mt-3 text-xs text-blue-200 leading-relaxed">
        Some provenance claims refer to external content that is not bundled here. The receipt still tracks them by hash, but their raw content cannot be re-checked locally.
      </p>
    )}

    {report.summary.attachments_total === 0 && (
      <p className="mt-3 text-xs text-slate-400 leading-relaxed">
        This JSON receipt does not embed content files. It verifies workflow structure and content hashes, not the raw text.
      </p>
    )}
  </div>
)}
```

**Raw Output** - Make collapsible (add state):
```tsx
const [rawExpanded, setRawExpanded] = useState(false);

// In render
<div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-5">
  <button
    onClick={() => setRawExpanded(!rawExpanded)}
    className="flex w-full items-center justify-between text-left"
  >
    <h2 className="text-lg font-semibold text-slate-100">Developer details (raw JSON)</h2>
    {rawExpanded ? (
      <ChevronUp className="h-5 w-5 text-slate-400" />
    ) : (
      <ChevronDown className="h-5 w-5 text-slate-400" />
    )}
  </button>

  {rawExpanded && (
    <>
      <p className="mb-4 text-sm text-slate-400 mt-2">
        Review the normalized JSON payload returned from the verifier.
      </p>
      <pre className="max-h-[420px] overflow-auto rounded-lg bg-slate-950/80 p-4 text-xs leading-relaxed text-slate-200">
        {rawJson || defaultJsonPlaceholder}
      </pre>
    </>
  )}
</div>
```

---

## Part 7: Visualize Content Tab - WorkflowOverviewCard

### Current State
**File**: `src/components/WorkflowOverviewCard.tsx`

### Required Changes

**Add description** before fields (line ~25):
```tsx
<header className="mb-6">
  <p className="text-xs uppercase tracking-[0.3em] text-brand-300">Workflow</p>
  <h3 className="text-2xl font-semibold text-slate-50">Overview</h3>
  <p className="mt-2 text-sm text-slate-400">
    This section shows what this run did: workflow name, models, budgets, and stewardship score recorded in the receipt.
  </p>
</header>
```

**Budgets Section** - Show "—" for missing values:
```tsx
{/* Budgets (if non-zero) */}
{(budgets.tokens > 0 || budgets.usd > 0 || budgets.nature_cost > 0) && (
  <div className="rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
    <dt className="text-xs uppercase tracking-wide text-slate-400">Budgets</dt>
    <dd className="mt-2 space-y-1 text-sm text-slate-100">
      <div className="flex justify-between">
        <span className="text-slate-400">Tokens:</span>
        <span className="font-mono">{budgets.tokens > 0 ? formatNumber(budgets.tokens) : '—'}</span>
      </div>
      <div className="flex justify-between">
        <span className="text-slate-400">USD:</span>
        <span className="font-mono">{budgets.usd > 0 ? `$${formatNumber(budgets.usd)}` : '—'}</span>
      </div>
      <div className="flex justify-between">
        <span className="text-slate-400">Nature Cost:</span>
        <span className="font-mono">{budgets.nature_cost > 0 ? formatNumber(budgets.nature_cost) : '—'}</span>
      </div>
    </dd>
  </div>
)}
```

**Stewardship Score** - Add helper text:
```tsx
<div className="rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
  <dt className="text-xs uppercase tracking-wide text-slate-400">Stewardship Score</dt>
  <dd className="mt-2">
    <div className="flex items-center justify-between mb-2">
      <span className="text-2xl font-semibold text-slate-100">{score}</span>
      <span className="text-sm text-slate-400">/100</span>
    </div>
    <div className="h-2 rounded-full bg-slate-800 overflow-hidden">
      <div
        className="h-full bg-gradient-to-r from-brand-400 to-brand-500 transition-all"
        style={{ width: `${score}%` }}
      />
    </div>
    <p className="mt-2 text-xs text-slate-400">
      A higher score reflects more transparent and policy-aligned use of models and resources.
    </p>
  </dd>
</div>
```

---

## Part 8: Visualize Content Tab - WorkflowStepsCard

### Current State
**File**: `src/components/WorkflowStepsCard.tsx`

### Required Changes

**Step Header** - Add human-readable summary (around line 35):
```tsx
<div className="mb-4 border-b border-slate-800/60 pb-3">
  <h4 className="text-base font-semibold text-slate-100">
    Step {step.orderIndex} – {step.checkpointType || 'Unknown'}
  </h4>
  <p className="text-sm text-slate-400 mt-1">
    Type: {step.proofMode || 'N/A'} · Model: {step.model || 'N/A'}
    {step.epsilon !== undefined && step.epsilon !== null && ` · ε = ${step.epsilon}`}
  </p>
</div>
```

**Prompt Section** - Add note when not included:
```tsx
{/* Prompt */}
{step.prompt && (
  <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
    <dt className="text-xs uppercase tracking-wide text-slate-400">Prompt</dt>
    <dd className="mt-1 text-sm text-slate-200 whitespace-pre-wrap font-mono">
      {truncateText(step.prompt, 240)}
    </dd>
  </div>
)}
{!step.prompt && (
  <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
    <p className="text-sm text-slate-400 italic">
      Prompt content not included in this receipt (tracked by hash only).
    </p>
  </div>
)}
```

---

## Part 9: Visualize Content Tab - AttachmentsCard

### Current State
**File**: `src/components/AttachmentsCard.tsx` (already has good provenance warning)

### Required Changes

**Empty State** - Update wording:
```tsx
if (!attachments || attachments.length === 0) {
  return (
    <div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-6 shadow-lg shadow-slate-950/40">
      <header className="mb-4">
        <p className="text-xs uppercase tracking-[0.3em] text-brand-300">Attachments</p>
        <h3 className="text-2xl font-semibold text-slate-50">Content Files</h3>
      </header>
      <p className="text-sm text-slate-400">
        This receipt does not include any content files. It only records workflow metadata and content hashes.
      </p>
    </div>
  );
}
```

**Provenance Note** - Update existing warning (lines 210-214):
```tsx
{showWarning ? (
  <p className="text-sm text-amber-200">
    <strong>Note:</strong> This receipt records {provenance.length} provenance claim
    {provenance.length !== 1 ? 's' : ''}. {missingExternal} of them refer to content stored externally or not exported. The receipt remains consistent, but only the bundled files can be inspected here.
  </p>
) : (
  <p className="text-sm text-slate-300">
    <strong>Provenance:</strong> {provenance.length} claim{provenance.length !== 1 ? 's' : ''} — {externalWithFile} bundled file{externalWithFile !== 1 ? 's' : ''} can be inspected
    {inlineClaims > 0 && `, ${inlineClaims} tracked by metadata only`}.
  </p>
)}
```

---

## Implementation Checklist

- [ ] Update header/hero area (Verifier.tsx)
- [ ] Add success callout component (Verifier.tsx)
- [ ] Update last file status line (Verifier.tsx)
- [ ] Update WorkflowViewer step labels and descriptions
- [ ] Add Coverage section to MetadataCard
- [ ] Make Raw Output collapsible in MetadataCard
- [ ] Add description to WorkflowOverviewCard
- [ ] Update budget display with "—" for missing values
- [ ] Add stewardship score helper text
- [ ] Add step headers to WorkflowStepsCard
- [ ] Add prompt not-included message
- [ ] Update AttachmentsCard empty state
- [ ] Update AttachmentsCard provenance note

---

## Testing Scenarios

1. **JSON-only receipt** (no attachments):
   - Verify "receipt does not bundle content files" messaging
   - Check that provenance shows as "tracked by hash only"

2. **ZIP bundle with all content**:
   - Verify success callout appears
   - Check coverage section shows correct counts

3. **ZIP bundle with missing content**:
   - Verify "content not included" warnings appear
   - Check provenance status shows as "Info" not error

4. **Failed verification**:
   - Verify error messaging is clear
   - Check that status shows "Verification failed"

---

## Notes

- All changes preserve existing functionality
- No changes to verification logic or data model
- Focus on clarity and user education
- Consistent use of "receipt" terminology
- Clear distinction between cryptographic validity and content correctness
