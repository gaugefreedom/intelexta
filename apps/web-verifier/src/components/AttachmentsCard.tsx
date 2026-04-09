import { useState } from 'react';
import { ChevronDown, ChevronUp, FileText, Paperclip } from 'lucide-react';
import { useTranslation } from 'react-i18next';
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
  const { t } = useTranslation();
  const [isExpanded, setIsExpanded] = useState(false);

  const hasLongerContent =
    attachment.fullText && attachment.preview && attachment.fullText.length > attachment.preview.length;

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
            {attachment.kind === 'text' && ` · ${t('attachments_text_file')}`}
            {attachment.kind === 'binary' && ` · ${t('attachments_binary_file')}`}
          </p>
        </div>
      </div>

      {/* Content Preview */}
      {attachment.kind === 'text' && attachment.preview && (
        <div className="rounded-lg border border-slate-100 bg-slate-50 p-4">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2 text-[10px] uppercase font-bold tracking-wide text-slate-400">
              <FileText className="h-3.5 w-3.5 text-emerald-600" />
              {t('attachments_content_preview')}
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
                    {t('attachments_collapse')}
                  </>
                ) : (
                  <>
                    <ChevronDown className="h-3 w-3" />
                    {t('attachments_view_full')}
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
              {t('attachments_preview_truncated')}
            </p>
          )}
        </div>
      )}

      {/* Binary File Notice */}
      {attachment.kind === 'binary' && (
        <div className="rounded-lg border border-slate-100 bg-slate-50 p-4">
          <p className="text-sm text-slate-500 flex items-center gap-2">
            <Paperclip className="h-4 w-4 text-slate-400" />
            {t('attachments_binary_notice')}
          </p>
        </div>
      )}

      {/* Hash Reference */}
      {attachment.hashHex && (
        <div className="mt-3 rounded-lg border border-slate-100 bg-slate-50 px-3 py-2">
          <p className="text-[10px] uppercase font-bold tracking-wide text-slate-400 mb-1">{t('attachments_content_hash')}</p>
          <code className="text-xs text-slate-600 font-mono break-all">
            sha256:{attachment.hashHex}
          </code>
        </div>
      )}
    </div>
  );
};

const AttachmentsCard = ({ attachments, provenance }: AttachmentsCardProps) => {
  const { t } = useTranslation();

  if (!attachments || attachments.length === 0) {
    return (
      <div className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
        <header className="mb-4">
          <p className="text-xs uppercase tracking-[0.3em] font-bold text-slate-400">{t('attachments_label')}</p>
          <h3 className="text-xl font-bold text-slate-900 mt-1">{t('attachments_title')}</h3>
        </header>
        <p className="text-sm text-slate-500">{t('attachments_empty_body')}</p>
      </div>
    );
  }

  const textAttachments = attachments.filter((a) => a.kind === 'text');

  const INLINE_TYPES = new Set(['config', 'policy', 'run_spec']);

  let inlineClaims = 0;
  let externalClaims = 0;
  let externalWithFile = 0;
  let missingExternal = 0;

  if (provenance) {
    const attachmentHashes = new Set(
      attachments.map((a) => a.fileName.replace(/\.(txt|md|json)$/i, ''))
    );

    for (const claim of provenance) {
      const hash = claim.sha256.replace(/^sha256:/, '');

      if (INLINE_TYPES.has(claim.claim_type)) {
        inlineClaims++;
      } else {
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

  const buildProvenanceText = () => {
    if (showWarning) {
      const key = provenance!.length === 1
        ? 'attachments_provenance_warning'
        : 'attachments_provenance_warning_plural';
      return t(key, { total: provenance!.length, missing: missingExternal });
    }

    const pluralTotal = provenance!.length !== 1;
    const pluralBundled = externalWithFile !== 1;
    let key: string;
    if (pluralTotal && pluralBundled) key = 'attachments_provenance_ok_plural_both';
    else if (pluralTotal) key = 'attachments_provenance_ok_plural_total';
    else if (pluralBundled) key = 'attachments_provenance_ok_plural_bundled';
    else key = 'attachments_provenance_ok';

    let text = t(key, { total: provenance!.length, bundled: externalWithFile });
    if (inlineClaims > 0) {
      text += t(inlineClaims === 1 ? 'attachments_provenance_inline_suffix_one' : 'attachments_provenance_inline_suffix_other', { count: inlineClaims });
    }
    return text;
  };

  return (
    <div className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <header className="mb-6">
        <p className="text-xs uppercase tracking-[0.3em] font-bold text-slate-400">{t('attachments_label')}</p>
        <h3 className="text-xl font-bold text-slate-900 mt-1">{t('attachments_title')}</h3>
        <p className="mt-1 text-sm text-slate-500">
          {t(attachments.length === 1 ? 'attachments_count_one' : 'attachments_count_other', { count: attachments.length })}
          {textAttachments.length > 0 && ` · ${t(textAttachments.length === 1 ? 'attachments_preview_count_one' : 'attachments_preview_count_other', { count: textAttachments.length })}`}
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
          <p
            className={`text-sm ${showWarning ? 'text-amber-800' : 'text-slate-600'}`}
            dangerouslySetInnerHTML={{ __html: buildProvenanceText() }}
          />
        </div>
      )}
    </div>
  );
};

export default AttachmentsCard;
