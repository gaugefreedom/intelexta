import {
  BadgeCheck,
  CalendarClock,
  FileBadge2,
  Fingerprint,
  ShieldCheck,
  Workflow,
  Copy
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { VerificationReport } from '../types/verifier';

interface MetadataCardProps {
  report?: VerificationReport | null;
}

const truncateHash = (hash: string, prefixLen = 12, suffixLen = 8) => {
  if (!hash || hash.length <= prefixLen + suffixLen + 3) return hash;
  return `${hash.slice(0, prefixLen)}...${hash.slice(-suffixLen)}`;
};

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text).catch(console.error);
};

const MetadataCard = ({ report }: MetadataCardProps) => {
  const { t, i18n } = useTranslation();

  const formatDate = (value: string) => {
    if (!value) return t('metadata_unknown_timestamp');
    const parsed = new Date(value);
    if (Number.isNaN(parsed.getTime())) return value;
    return parsed.toLocaleString(i18n.language);
  };

  if (!report) {
    return (
      <section className="rounded-xl border border-slate-200 bg-white p-6 text-sm text-slate-500 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">{t('metadata_title')}</h2>
        <p className="mt-2 text-slate-500">{t('metadata_empty_body')}</p>
      </section>
    );
  }

  const statusStyles = {
    verified: {
      label: t('metadata_status_verified'),
      className: 'border-emerald-200 bg-emerald-100 text-emerald-700'
    },
    failed: {
      label: t('metadata_status_failed'),
      className: 'border-rose-200 bg-rose-100 text-rose-700'
    }
  } as const;

  const statusStyle = statusStyles[report.status] ?? statusStyles.failed;
  const summary = report.summary;

  const modelName = report.model.name.startsWith('workflow:')
    ? report.model.name.replace('workflow:', '')
    : report.model.name;

  const modelVersion = report.model.version ? truncateHash(report.model.version, 8, 8) : '';
  const modelLabel = [modelName, modelVersion].filter(Boolean).join(' · ') || 'Unknown model';

  const signerKey = report.signer?.public_key ?? '';
  const signerLabel = signerKey ? truncateHash(signerKey, 16, 8) : t('metadata_signer_unsigned');

  const numericMetrics = [
    {
      label: t('metadata_checkpoints'),
      value: t('metadata_verified_count', { verified: summary.checkpoints_verified, total: summary.checkpoints_total })
    },
    {
      label: t('metadata_provenance'),
      value: t('metadata_verified_count', { verified: summary.provenance_verified, total: summary.provenance_total })
    },
    {
      label: t('metadata_attachments'),
      value: t('metadata_verified_count', { verified: summary.attachments_verified, total: summary.attachments_total })
    }
  ];

  const booleanChecks = [
    { label: t('metadata_hash_chain'), passed: summary.hash_chain_valid },
    { label: t('metadata_signatures'), passed: summary.signatures_valid },
    { label: t('metadata_content_integrity'), passed: summary.content_integrity_valid }
  ];

  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <header className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-lg font-semibold text-slate-900">{t('metadata_title')}</h2>
          <span
            className={`inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs font-bold uppercase tracking-wide ${statusStyle.className}`}
          >
            <BadgeCheck className="h-3.5 w-3.5" aria-hidden />
            {statusStyle.label}
          </span>
        </div>
        {report.error && (
          <div className="rounded-md bg-rose-50 border border-rose-100 p-3 text-sm text-rose-700">
            {report.error}
          </div>
        )}
      </header>

      <dl className="mt-6 space-y-3">
        {/* Run ID */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <Workflow className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div>
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">{t('metadata_run_id')}</dt>
            <dd className="text-sm font-medium text-slate-700">{report.run_id || t('metadata_unknown_run')}</dd>
          </div>
        </div>

        {/* CAR ID */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3 group cursor-pointer hover:border-emerald-200 transition-colors"
             onClick={() => copyToClipboard(report.car_id)}
             title={t('metadata_car_id_copy_title')}>
          <FileBadge2 className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div className="flex-1 min-w-0">
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400 flex items-center gap-2">
              {t('metadata_car_id')} <Copy size={10} className="opacity-0 group-hover:opacity-100 transition-opacity" />
            </dt>
            <dd className="text-sm font-medium text-slate-700 font-mono truncate hover:text-emerald-700 transition-colors">
              {truncateHash(report.car_id, 16, 12)}
            </dd>
          </div>
        </div>

        {/* Signer */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3 group cursor-pointer hover:border-emerald-200 transition-colors"
             onClick={signerKey ? () => copyToClipboard(signerKey) : undefined}
             title={signerKey ? t('metadata_signer_copy_title') : t('metadata_signer_unsigned')}>
          <Fingerprint className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div className="flex-1 min-w-0">
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400 flex items-center gap-2">
              {t('metadata_signer')} <Copy size={10} className="opacity-0 group-hover:opacity-100 transition-opacity" />
            </dt>
            <dd className={`text-sm font-medium font-mono truncate ${signerKey ? 'text-slate-700 hover:text-emerald-700' : 'text-slate-400 italic'}`}>
              {signerLabel}
            </dd>
          </div>
        </div>

        {/* Workflow Model */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3 group cursor-pointer hover:border-emerald-200 transition-colors"
             onClick={() => copyToClipboard(`${report.model.name} · ${report.model.version}`)}>
          <ShieldCheck className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div className="flex-1 min-w-0">
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400 flex items-center gap-2">
              {t('metadata_workflow_model')} <Copy size={10} className="opacity-0 group-hover:opacity-100 transition-opacity" />
            </dt>
            <dd className="text-sm font-medium text-slate-700 hover:text-emerald-700 truncate">
              {modelLabel}
            </dd>
          </div>
        </div>

        {/* Timestamp */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <CalendarClock className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div>
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">{t('metadata_timestamp')}</dt>
            <dd className="text-sm font-medium text-slate-700">{formatDate(report.created_at)}</dd>
          </div>
        </div>
      </dl>

      <div className="mt-6 space-y-6 pt-6 border-t border-slate-100">
        <div>
          <h3 className="text-xs font-bold uppercase tracking-wide text-slate-400 mb-3">{t('metadata_metrics_title')}</h3>
          <ul className="space-y-2" role="list">
            {numericMetrics.map((metric) => (
              <li
                key={metric.label}
                className="flex items-center justify-between rounded-md border border-slate-100 bg-white px-3 py-2 text-sm text-slate-600"
              >
                <span>{metric.label}</span>
                <span className="font-semibold text-slate-900">{metric.value}</span>
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h3 className="text-xs font-bold uppercase tracking-wide text-slate-400 mb-3">{t('metadata_integrity_title')}</h3>
          <ul className="space-y-2" role="list">
            {booleanChecks.map((item) => (
              <li
                key={item.label}
                className="flex items-center justify-between rounded-md border border-slate-100 bg-white px-3 py-2 text-sm"
              >
                <span className="text-slate-600">{item.label}</span>
                <span className={item.passed ? 'text-emerald-600 font-medium' : 'text-rose-600 font-medium'}>
                  {item.passed ? t('metadata_passed') : t('metadata_failed')}
                </span>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </section>
  );
};

export default MetadataCard;
