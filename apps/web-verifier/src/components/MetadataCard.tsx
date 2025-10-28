import {
  BadgeCheck,
  CalendarClock,
  FileBadge2,
  Fingerprint,
  ShieldCheck,
  Workflow
} from 'lucide-react';
import type { VerificationReport } from '../types/verifier';

interface MetadataCardProps {
  report?: VerificationReport | null;
}

const statusStyles = {
  verified: {
    label: 'Verified',
    className: 'border-emerald-500/40 bg-emerald-500/10 text-emerald-200'
  },
  failed: {
    label: 'Verification failed',
    className: 'border-rose-500/40 bg-rose-500/10 text-rose-200'
  }
} as const;

const formatDate = (value: string) => {
  if (!value) return 'Unknown timestamp';
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString();
};

const MetadataCard = ({ report }: MetadataCardProps) => {
  if (!report) {
    return (
      <section className="rounded-xl border border-slate-800 bg-slate-900/70 p-6 text-sm text-slate-300">
        <h2 className="text-lg font-semibold text-slate-100">Verification Summary</h2>
        <p className="mt-2 text-slate-400">
          Drop a CAR archive to inspect signer, model details, and verification status.
        </p>
      </section>
    );
  }

  const statusStyle = statusStyles[report.status] ?? statusStyles.failed;
  const summary = report.summary;
  const modelLabel = [report.model.name, report.model.version].filter(Boolean).join(' · ') || 'Unknown model';
  const signerLabel = report.signer?.public_key ?? 'Signer not provided';

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
    <section className="rounded-2xl border border-slate-800 bg-slate-900/70 p-6 shadow-inner shadow-slate-950/30">
      <header className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-lg font-semibold text-slate-100">Verification Summary</h2>
          <span
            className={`inline-flex items-center gap-2 rounded-full border px-3 py-1 text-xs font-semibold uppercase tracking-wide ${statusStyle.className}`}
          >
            <BadgeCheck className="h-4 w-4" aria-hidden />
            {statusStyle.label}
          </span>
        </div>
        {report.error && (
          <p className="text-sm text-rose-200">
            {report.error}
          </p>
        )}
      </header>

      <dl className="mt-6 space-y-4">
        <div className="flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
          <Workflow className="mt-0.5 h-5 w-5 flex-shrink-0 text-brand-300" aria-hidden />
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate-400">Run ID</dt>
            <dd className="text-sm font-medium text-slate-100">{report.run_id || 'Unknown run'}</dd>
          </div>
        </div>

        <div className="flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
          <FileBadge2 className="mt-0.5 h-5 w-5 flex-shrink-0 text-brand-300" aria-hidden />
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate-400">CAR ID</dt>
            <dd className="text-sm font-medium text-slate-100">{report.car_id || 'Unknown CAR'}</dd>
          </div>
        </div>

        <div className="flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
          <Fingerprint className="mt-0.5 h-5 w-5 flex-shrink-0 text-brand-300" aria-hidden />
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate-400">Signer</dt>
            <dd className="text-sm font-medium text-slate-100 break-all">{signerLabel}</dd>
          </div>
        </div>

        <div className="flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
          <ShieldCheck className="mt-0.5 h-5 w-5 flex-shrink-0 text-brand-300" aria-hidden />
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate-400">Model</dt>
            <dd className="text-sm font-medium text-slate-100">{modelLabel}</dd>
          </div>
        </div>

        <div className="flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
          <CalendarClock className="mt-0.5 h-5 w-5 flex-shrink-0 text-brand-300" aria-hidden />
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate-400">Timestamp</dt>
            <dd className="text-sm font-medium text-slate-100">{formatDate(report.created_at)}</dd>
          </div>
        </div>
      </dl>

      <div className="mt-6 space-y-4">
        <div>
          <h3 className="text-sm font-semibold uppercase tracking-wide text-slate-300">Totals</h3>
          <ul className="mt-3 space-y-2" role="list">
            {numericMetrics.map((metric) => (
              <li
                key={metric.label}
                className="flex items-center justify-between rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-2 text-sm text-slate-200"
              >
                <span>{metric.label}</span>
                <span className="font-medium text-slate-100">{metric.value}</span>
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h3 className="text-sm font-semibold uppercase tracking-wide text-slate-300">Checks</h3>
          <ul className="mt-3 space-y-2" role="list">
            {booleanChecks.map((item) => (
              <li
                key={item.label}
                className="flex items-center justify-between rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-2 text-sm"
              >
                <span className="text-slate-200">{item.label}</span>
                <span className={item.passed ? 'text-emerald-300' : 'text-rose-300'}>
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
