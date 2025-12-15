import { AlertCircle, CheckCircle2, Circle, FileText, Paperclip } from 'lucide-react';
import type { VerificationReport, WorkflowStep } from '../types/verifier';

interface WorkflowViewerProps {
  report?: VerificationReport | null;
}

const statusAppearance = {
  passed: {
    badge: 'border-emerald-200 bg-emerald-50 text-emerald-700',
    icon: <CheckCircle2 className="h-5 w-5 text-emerald-500" aria-hidden />
  },
  failed: {
    badge: 'border-rose-200 bg-rose-50 text-rose-700',
    icon: <AlertCircle className="h-5 w-5 text-rose-500" aria-hidden />
  },
  skipped: {
    badge: 'border-slate-200 bg-slate-100 text-slate-500',
    icon: <Circle className="h-4 w-4 text-slate-300" aria-hidden />
  }
} as const;

const deriveDetails = (step: WorkflowStep) => {
  const details = step.details ?? [];
  const attachmentDetails = details.filter((detail) => detail.label.toLowerCase().includes('attachment'));
  const contentDetails = details.filter((detail) => !detail.label.toLowerCase().includes('attachment'));
  return { attachmentDetails, contentDetails };
};

const WorkflowViewer = ({ report }: WorkflowViewerProps) => {
  const steps = report?.workflow?.steps ?? [];

  if (!steps.length) {
    return (
      <section className="rounded-xl border border-slate-200 bg-white p-8 text-center shadow-sm">
        <p className="text-slate-500 text-sm">No workflow steps found in this receipt.</p>
      </section>
    );
  }

  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <header className="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:justify-between mb-8">
        <div>
          <h2 className="text-xl font-bold text-slate-900">Verification Timeline</h2>
          <p className="text-sm text-slate-500">Step-by-step cryptographic validation of the workflow.</p>
        </div>
        <span className="text-xs font-medium px-2 py-1 bg-slate-100 text-slate-600 rounded-md">
          {steps.length} Steps
        </span>
      </header>

      <div className="relative border-l-2 border-slate-100 ml-3 space-y-8 pb-2">
        {steps.map((step, index) => {
          const appearance = statusAppearance[step.status] ?? statusAppearance.skipped;
          const { attachmentDetails, contentDetails } = deriveDetails(step);

          return (
            <div key={`${step.key}-${index}`} className="relative pl-8">
              {/* Timeline Dot */}
              <span className="absolute -left-[11px] top-0 bg-white p-1">
                {appearance.icon}
              </span>

              <article className="flex flex-col gap-4 rounded-xl border border-slate-200 bg-slate-50/50 p-5">
                <header className="flex items-center justify-between">
                  <h3 className="font-bold text-slate-800 text-sm">
                    {index + 1}. {step.label}
                  </h3>
                  <span className={`inline-flex items-center rounded-full border px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide ${appearance.badge}`}>
                    {step.status}
                  </span>
                </header>

                {step.error && (
                  <div className="flex items-start gap-2 rounded-lg border border-rose-200 bg-rose-50 p-3 text-xs text-rose-700">
                    <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
                    <p>{step.error}</p>
                  </div>
                )}

                <div className="grid gap-4 md:grid-cols-2">
                  {/* Content Section */}
                  <section className="space-y-2">
                    <div className="flex items-center gap-2 text-xs font-bold text-slate-400 uppercase tracking-wider">
                      <FileText size={12} /> Content
                    </div>
                    {contentDetails.length ? (
                      contentDetails.map((detail, i) => (
                        <div key={i} className="rounded border border-slate-200 bg-white p-3 text-sm">
                          <p className="text-[10px] text-slate-400 font-bold mb-1">{detail.label}</p>
                          <p className="text-slate-700 whitespace-pre-wrap">{detail.value}</p>
                        </div>
                      ))
                    ) : <p className="text-xs text-slate-400 italic">No content data.</p>}
                  </section>

                  {/* Attachments Section */}
                  <section className="space-y-2">
                    <div className="flex items-center gap-2 text-xs font-bold text-slate-400 uppercase tracking-wider">
                      <Paperclip size={12} /> Attachments
                    </div>
                    {attachmentDetails.length ? (
                      <ul className="space-y-2">
                        {attachmentDetails.map((detail, i) => (
                          <li key={i} className="rounded border border-slate-200 bg-white p-3 text-sm">
                            <p className="text-[10px] text-slate-400 font-bold mb-1">{detail.label}</p>
                            <p className="text-slate-700">{detail.value}</p>
                          </li>
                        ))}
                      </ul>
                    ) : <p className="text-xs text-slate-400 italic">No attachments.</p>}
                  </section>
                </div>
              </article>
            </div>
          );
        })}
      </div>
    </section>
  );
};

export default WorkflowViewer;