import {
  BadgeCheck,
  CalendarClock,
  FileBadge2,
  Fingerprint,
  ShieldCheck,
  Workflow,
  Copy
} from 'lucide-react';
import type { VerificationReport } from '../types/verifier';

interface MetadataCardProps {
  report?: VerificationReport | null;
}

const statusStyles = {
  verified: {
    label: 'Verified',
    className: 'border-emerald-200 bg-emerald-100 text-emerald-700'
  },
  failed: {
    label: 'Verification failed',
    className: 'border-rose-200 bg-rose-100 text-rose-700'
  }
} as const;

const formatDate = (value: string) => {
  if (!value) return 'Unknown timestamp';
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString();
};

const truncateHash = (hash: string, prefixLen = 12, suffixLen = 8) => {
  if (!hash || hash.length <= prefixLen + suffixLen + 3) return hash;
  return `${hash.slice(0, prefixLen)}...${hash.slice(-suffixLen)}`;
};

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text).catch(console.error);
};

const MetadataCard = ({ report }: MetadataCardProps) => {
  if (!report) {
    return (
      <section className="rounded-xl border border-slate-200 bg-white p-6 text-sm text-slate-500 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Verification Summary</h2>
        <p className="mt-2 text-slate-500">
          Drop a CAR archive to inspect signer, model details, and verification status.
        </p>
      </section>
    );
  }

  const statusStyle = statusStyles[report.status] ?? statusStyles.failed;
  const summary = report.summary;

  // Extract workflow name from model (e.g., "workflow:llm question")
  const modelName = report.model.name.startsWith('workflow:')
    ? report.model.name.replace('workflow:', '')
    : report.model.name;

  // Truncate version hash
  const modelVersion = report.model.version ? truncateHash(report.model.version, 8, 8) : '';
  const modelLabel = [modelName, modelVersion].filter(Boolean).join(' · ') || 'Unknown model';

  const signerKey = report.signer?.public_key ?? '';
  const signerLabel = signerKey ? truncateHash(signerKey, 16, 8) : 'Unsigned';

  const numericMetrics = [
    {
      label: 'Checkpoints',
      value: `${summary.checkpoints_verified}/${summary.checkpoints_total} verified`
    },
    {
      label: 'Provenance',
      value: `${summary.provenance_verified}/${summary.provenance_total} verified`
    },
    {
      label: 'Attachments',
      value: `${summary.attachments_verified}/${summary.attachments_total} verified`
    }
  ];

  const booleanChecks = [
    { label: 'Hash chain integrity', passed: summary.hash_chain_valid },
    { label: 'Signature validation', passed: summary.signatures_valid },
    { label: 'Content integrity', passed: summary.content_integrity_valid }
  ];

  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <header className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-lg font-semibold text-slate-900">Verification Summary</h2>
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
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">Run ID</dt>
            <dd className="text-sm font-medium text-slate-700">{report.run_id || 'Unknown run'}</dd>
          </div>
        </div>

        {/* CAR ID */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3 group cursor-pointer hover:border-emerald-200 transition-colors"
             onClick={() => copyToClipboard(report.car_id)}
             title="Click to copy CAR ID">
          <FileBadge2 className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div className="flex-1 min-w-0">
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400 flex items-center gap-2">
              CAR ID <Copy size={10} className="opacity-0 group-hover:opacity-100 transition-opacity" />
            </dt>
            <dd className="text-sm font-medium text-slate-700 font-mono truncate hover:text-emerald-700 transition-colors">
              {truncateHash(report.car_id, 16, 12)}
            </dd>
          </div>
        </div>

        {/* Signer */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3 group cursor-pointer hover:border-emerald-200 transition-colors"
             onClick={signerKey ? () => copyToClipboard(signerKey) : undefined}
             title={signerKey ? "Click to copy Public Key" : "Unsigned"}>
          <Fingerprint className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div className="flex-1 min-w-0">
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400 flex items-center gap-2">
              Signer <Copy size={10} className="opacity-0 group-hover:opacity-100 transition-opacity" />
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
              Workflow Model <Copy size={10} className="opacity-0 group-hover:opacity-100 transition-opacity" />
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
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">Timestamp</dt>
            <dd className="text-sm font-medium text-slate-700">{formatDate(report.created_at)}</dd>
          </div>
        </div>
      </dl>

      <div className="mt-6 space-y-6 pt-6 border-t border-slate-100">
        <div>
          <h3 className="text-xs font-bold uppercase tracking-wide text-slate-400 mb-3">Metrics</h3>
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
          <h3 className="text-xs font-bold uppercase tracking-wide text-slate-400 mb-3">Integrity Checks</h3>
          <ul className="space-y-2" role="list">
            {booleanChecks.map((item) => (
              <li
                key={item.label}
                className="flex items-center justify-between rounded-md border border-slate-100 bg-white px-3 py-2 text-sm"
              >
                <span className="text-slate-600">{item.label}</span>
                <span className={item.passed ? 'text-emerald-600 font-medium' : 'text-rose-600 font-medium'}>
                  {item.passed ? 'Passed' : 'Failed'}
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