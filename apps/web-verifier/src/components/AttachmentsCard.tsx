import { useState } from 'react';
import { ChevronDown, ChevronUp, FileText, Paperclip } from 'lucide-react';
import type { AttachmentPreview, ProvenanceClaim } from '../types/car';
import { formatFileSize } from '../utils/zipParser';

interface AttachmentsCardProps {
  attachments: AttachmentPreview[];
  provenance?: ProvenanceClaim[];
}

interface AttachmentItemProps {
  attachment: AttachmentPreview;
}

const AttachmentItem = ({ attachment }: AttachmentItemProps) => {
  const [isExpanded, setIsExpanded] = useState(false);

  const hasLongerContent =
    attachment.fullText && attachment.preview && attachment.fullText.length > attachment.preview.length;

  // Determine badge color based on claim type
  const getBadgeColor = () => {
    switch (attachment.claimType) {
      case 'input':
        return 'bg-blue-500/10 text-blue-300 border-blue-500/30';
      case 'output':
        return 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30';
      case 'config':
        return 'bg-purple-500/10 text-purple-300 border-purple-500/30';
      default:
        return 'bg-slate-500/10 text-slate-300 border-slate-500/30';
    }
  };

  return (
    <div className="rounded-xl border border-slate-800/70 bg-slate-950/70 p-5">
      {/* Header */}
      <div className="mb-4 flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            {attachment.claimType && (
              <span
                className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium border ${getBadgeColor()}`}
              >
                {attachment.claimType}
              </span>
            )}
            <h4 className="text-sm font-medium text-slate-200 font-mono break-all">
              {attachment.fileName}
            </h4>
          </div>
          <p className="mt-1 text-xs text-slate-400">
            {formatFileSize(attachment.size)}
            {attachment.kind === 'text' && ' · Text file'}
            {attachment.kind === 'binary' && ' · Binary file'}
          </p>
        </div>
      </div>

      {/* Content Preview */}
      {attachment.kind === 'text' && attachment.preview && (
        <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-4">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-slate-400">
              <FileText className="h-3 w-3" />
              Content Preview
            </div>
            {hasLongerContent && (
              <button
                onClick={() => setIsExpanded(!isExpanded)}
                className="flex items-center gap-1 text-xs text-brand-400 hover:text-brand-300 transition-colors"
                aria-expanded={isExpanded}
              >
                {isExpanded ? (
                  <>
                    <ChevronUp className="h-3 w-3" />
                    Collapse
                  </>
                ) : (
                  <>
                    <ChevronDown className="h-3 w-3" />
                    View Full
                  </>
                )}
              </button>
            )}
          </div>

          <div className="overflow-auto rounded-md bg-slate-950/80 p-3 max-h-[500px]">
            <pre className="text-xs leading-relaxed text-slate-200 whitespace-pre-wrap font-mono">
              {isExpanded ? attachment.fullText : attachment.preview}
            </pre>
          </div>

          {!isExpanded && hasLongerContent && (
            <p className="mt-2 text-xs text-slate-400 italic">
              Preview truncated. Click "View Full" to see complete content.
            </p>
          )}
        </div>
      )}

      {/* Binary File Notice */}
      {attachment.kind === 'binary' && (
        <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-4">
          <p className="text-sm text-slate-400">
            <Paperclip className="inline h-4 w-4 mr-2" />
            Binary attachment (no inline preview available)
          </p>
        </div>
      )}

      {/* Hash Reference */}
      {attachment.hashHex && (
        <div className="mt-3 rounded-lg border border-slate-800/60 bg-slate-900/60 px-3 py-2">
          <p className="text-xs text-slate-500">Content Hash</p>
          <code className="text-xs text-slate-300 font-mono break-all">
            sha256:{attachment.hashHex}
          </code>
        </div>
      )}
    </div>
  );
};

const AttachmentsCard = ({ attachments, provenance }: AttachmentsCardProps) => {
  if (!attachments || attachments.length === 0) {
    return (
      <div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-6 shadow-lg shadow-slate-950/40">
        <header className="mb-4">
          <p className="text-xs uppercase tracking-[0.3em] text-brand-300">Attachments</p>
          <h3 className="text-2xl font-semibold text-slate-50">Content Files</h3>
        </header>
        <p className="text-sm text-slate-400">
          No attachments found in this CAR bundle. Attachments are only available in <code className="text-slate-300">.car.zip</code> bundles.
        </p>
      </div>
    );
  }

  // Separate attachments by type
  const textAttachments = attachments.filter((a) => a.kind === 'text');
  const binaryAttachments = attachments.filter((a) => a.kind === 'binary');

  // Compute provenance statistics
  const INLINE_TYPES = new Set(['config', 'policy', 'run_spec']);

  let inlineClaims = 0;
  let externalClaims = 0;
  let externalWithFile = 0;
  let missingExternal = 0;

  if (provenance) {
    // Build set of attachment hashes (without .txt extension)
    const attachmentHashes = new Set(
      attachments.map((a) => a.fileName.replace(/\.(txt|md|json)$/i, ''))
    );

    for (const claim of provenance) {
      const hash = claim.sha256.replace(/^sha256:/, '');

      if (INLINE_TYPES.has(claim.claim_type)) {
        // Config/policy claims are embedded in CAR metadata, no file expected
        inlineClaims++;
      } else {
        // External claims (input, output, attachment) should have files
        externalClaims++;
        if (attachmentHashes.has(hash)) {
          externalWithFile++;
        } else {
          missingExternal++;
        }
      }
    }
  }

  const showWarning = missingExternal > 0;

  return (
    <div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-6 shadow-lg shadow-slate-950/40">
      <header className="mb-6">
        <p className="text-xs uppercase tracking-[0.3em] text-brand-300">Attachments</p>
        <h3 className="text-2xl font-semibold text-slate-50">Content Files</h3>
        <p className="mt-1 text-sm text-slate-400">
          {attachments.length} file{attachments.length !== 1 ? 's' : ''} extracted from bundle
          {textAttachments.length > 0 && ` · ${textAttachments.length} with preview`}
        </p>
      </header>

      <div className="space-y-4">
        {attachments.map((attachment) => (
          <AttachmentItem key={attachment.fileName} attachment={attachment} />
        ))}
      </div>

      {/* Provenance Summary */}
      {provenance && provenance.length > 0 && (
        <div className={`mt-6 rounded-lg border p-4 ${
          showWarning
            ? 'border-amber-500/40 bg-amber-500/10'
            : 'border-slate-700/60 bg-slate-800/40'
        }`}>
          {showWarning ? (
            <p className="text-sm text-amber-200">
              <strong>Note:</strong> This CAR references {provenance.length} provenance claim
              {provenance.length !== 1 ? 's' : ''}. {missingExternal} non-config claim
              {missingExternal !== 1 ? 's do' : ' does'} not have matching attachment file
              {missingExternal !== 1 ? 's' : ''} in this bundle. Some content may be stored externally or was not exported.
            </p>
          ) : (
            <p className="text-sm text-slate-300">
              <strong>Provenance:</strong> {provenance.length} claim{provenance.length !== 1 ? 's' : ''} — {externalWithFile} external attachment{externalWithFile !== 1 ? 's' : ''} present
              {inlineClaims > 0 && `, ${inlineClaims} embedded in CAR metadata`}.
            </p>
          )}
        </div>
      )}
    </div>
  );
};

export default AttachmentsCard;
