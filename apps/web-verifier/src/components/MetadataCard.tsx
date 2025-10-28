import { CalendarClock, Fingerprint, UserCheck, Workflow } from 'lucide-react';
import type { VerificationMetadata } from '../wasm/loader';

interface MetadataCardProps {
  metadata?: VerificationMetadata;
}

const Placeholder = () => (
  <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-6 text-sm text-slate-400">
    No metadata returned yet. Drop a CAR archive to inspect signer, model, and timestamps.
  </div>
);

const InfoRow = ({
  icon: Icon,
  label,
  value
}: {
  icon: typeof CalendarClock;
  label: string;
  value?: string;
}) => (
  <div className="flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
    <Icon className="mt-0.5 h-5 w-5 flex-shrink-0 text-brand-300" aria-hidden />
    <div className="flex flex-col">
      <span className="text-xs uppercase tracking-wide text-slate-400">{label}</span>
      <span className="text-sm font-medium text-slate-100">{value ?? '—'}</span>
    </div>
  </div>
);

const MetadataCard = ({ metadata }: MetadataCardProps) => {
  if (!metadata) {
    return <Placeholder />;
  }

  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-6">
      <h2 className="text-lg font-semibold text-slate-100">Run Metadata</h2>
      <div className="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-2">
        <InfoRow icon={Workflow} label="Run ID" value={metadata.runId} />
        <InfoRow icon={UserCheck} label="Model" value={metadata.model} />
        <InfoRow icon={Fingerprint} label="Signer" value={metadata.signer} />
        <InfoRow icon={CalendarClock} label="Created" value={metadata.createdAt} />
      </div>
      {metadata.dataset && (
        <p className="mt-4 text-sm text-slate-300">
          Dataset: <span className="font-medium text-slate-100">{metadata.dataset}</span>
        </p>
      )}
    </div>
  );
};

export default MetadataCard;
