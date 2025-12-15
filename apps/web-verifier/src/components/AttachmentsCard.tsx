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

  // Determine badge color based on claim type (Light Theme)
  const getBadgeColor = () => {
    switch (attachment.claimType) {
      case 'input':
        return 'bg-blue-50 text-blue-700 border-blue-200';
      case 'output':
        return 'bg-emerald-50 text-emerald-700 border-emerald-200';
      case 'config':
        return 'bg-purple-50 text-purple-700 border-purple-200';
      default:
        return 'bg-slate-100 text-slate-600 border-slate-200';
    }
  };

  return (
    <div className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      {/* Header */}
      <div className="mb-4 flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            {attachment.claimType && (
              <span
                className={`inline-flex items-center rounded-full px-2 py-0.5 text-[10px] uppercase font-bold tracking-wide border ${getBadgeColor()}`}
              >
                {attachment.claimType}
              </span>
            )}
            <h4 className="text-sm font-bold text-slate-900 font-mono break-all">
              {attachment.fileName}
            </h4>
          </div>
          <p className="mt-1 text-xs text-slate-500 font-medium">
            {formatFileSize(attachment.size)}
            {attachment.kind === 'text' && ' · Text file'}
            {attachment.kind === 'binary' && ' · Binary file'}
          </p>
        </div>
      </div>

      {/* Content Preview */}
      {attachment.kind === 'text' && attachment.preview && (
        <div className="rounded-lg border border-slate-100 bg-slate-50 p-4">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2 text-[10px] uppercase font-bold tracking-wide text-slate-400">
              <FileText className="h-3.5 w-3.5 text-emerald-600" />
              Content Preview
            </div>
            {hasLongerContent && (
              <button
                onClick={() => setIsExpanded(!isExpanded)}
                className="flex items-center gap-1 text-xs font-medium text-emerald-600 hover:text-emerald-700 transition-colors"
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

          <div className="overflow-auto rounded-md bg-white border border-slate-200 p-3 max-h-[500px] shadow-inner">
            <pre className="text-xs leading-relaxed text-slate-600 whitespace-pre-wrap font-mono">
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
        <div className="rounded-lg border border-slate-100 bg-slate-50 p-4">
          <p className="text-sm text-slate-500 flex items-center gap-2">
            <Paperclip className="h-4 w-4 text-slate-400" />
            Binary attachment (no inline preview available)
          </p>
        </div>
      )}

      {/* Hash Reference */}
      {attachment.hashHex && (
        <div className="mt-3 rounded-lg border border-slate-100 bg-slate-50 px-3 py-2">
          <p className="text-[10px] uppercase font-bold tracking-wide text-slate-400 mb-1">Content Hash</p>
          <code className="text-xs text-slate-600 font-mono break-all">
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
      <div className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
        <header className="mb-4">
          <p className="text-xs uppercase tracking-[0.3em] font-bold text-slate-400">Attachments</p>
          <h3 className="text-xl font-bold text-slate-900 mt-1">Content Files</h3>
        </header>
        <p className="text-sm text-slate-500">
          This receipt does not include any content files. It only records workflow metadata and content hashes.
        </p>
      </div>
    );
  }

  // Separate attachments by type
  const textAttachments = attachments.filter((a) => a.kind === 'text');

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
    <div className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <header className="mb-6">
        <p className="text-xs uppercase tracking-[0.3em] font-bold text-slate-400">Attachments</p>
        <h3 className="text-xl font-bold text-slate-900 mt-1">Content Files</h3>
        <p className="mt-1 text-sm text-slate-500">
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
            ? 'border-amber-200 bg-amber-50'
            : 'border-slate-200 bg-slate-50'
        }`}>
          {showWarning ? (
            <p className="text-sm text-amber-800">
              <strong className="font-semibold">Note:</strong> This receipt records {provenance.length} provenance claim
              {provenance.length !== 1 ? 's' : ''}. {missingExternal} of them refer to content stored externally or not exported. The receipt remains consistent, but only the bundled files can be inspected here.
            </p>
          ) : (
            <p className="text-sm text-slate-600">
              <strong className="font-semibold text-slate-900">Provenance:</strong> {provenance.length} claim{provenance.length !== 1 ? 's' : ''} — {externalWithFile} bundled file{externalWithFile !== 1 ? 's' : ''} can be inspected
              {inlineClaims > 0 && `, ${inlineClaims} tracked by metadata only`}.
            </p>
          )}
        </div>
      )}
    </div>
  );
};

export default AttachmentsCard;