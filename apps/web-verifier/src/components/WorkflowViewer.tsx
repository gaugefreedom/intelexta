import { AlertCircle, CheckCircle2, Circle, FileText, Paperclip } from 'lucide-react';
import type { VerificationReport, WorkflowStep } from '../types/verifier';

interface WorkflowViewerProps {
  report?: VerificationReport | null;
}

const statusAppearance = {
  passed: {
    badge: 'border-emerald-500/40 bg-emerald-500/10 text-emerald-200',
    icon: <CheckCircle2 className="h-5 w-5 text-emerald-400" aria-hidden />
  },
  failed: {
    badge: 'border-rose-500/40 bg-rose-500/10 text-rose-200',
    icon: <AlertCircle className="h-5 w-5 text-rose-400" aria-hidden />
  },
  skipped: {
    badge: 'border-slate-700/60 bg-slate-800/80 text-slate-200',
    icon: <Circle className="h-4 w-4 text-slate-400" aria-hidden />
  }
} as const;

const deriveDetails = (step: WorkflowStep) => {
  const details = step.details ?? [];
  const attachmentDetails = details.filter((detail) =>
    detail.label.toLowerCase().includes('attachment')
  );
  const contentDetails = details.filter(
    (detail) => !detail.label.toLowerCase().includes('attachment')
  );
  return { attachmentDetails, contentDetails };
};

const WorkflowViewer = ({ report }: WorkflowViewerProps) => {
  const steps = report?.workflow?.steps ?? [];

  if (!steps.length) {
    return (
      <section
        aria-labelledby="workflow-viewer-heading"
        className="rounded-xl border border-slate-800 bg-slate-900/70 p-6 text-sm text-slate-300"
      >
        <h2 id="workflow-viewer-heading" className="text-lg font-semibold text-slate-100">
          Workflow Timeline
        </h2>
        <p className="mt-2 text-slate-400">
          No workflow steps were returned by the verifier. Upload a CAR archive to review prompts,
          model outputs, and attachment checks.
        </p>
      </section>
    );
  }

  return (
    <section
      aria-labelledby="workflow-viewer-heading"
      className="rounded-2xl border border-slate-800 bg-slate-900/70 p-6 shadow-lg shadow-slate-950/40"
    >
      <header className="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.3em] text-brand-300">Workflow</p>
          <h2 id="workflow-viewer-heading" className="text-2xl font-semibold text-slate-50">
            Verification Timeline
          </h2>
        </div>
        <p className="text-sm text-slate-400" aria-live="polite">
          {steps.length} step{steps.length === 1 ? '' : 's'} verified
        </p>
      </header>

      <ol role="list" className="mt-8 space-y-8 border-l border-slate-800 pl-6">
        {steps.map((step, index) => {
          const appearance = statusAppearance[step.status] ?? statusAppearance.skipped;
          const { attachmentDetails, contentDetails } = deriveDetails(step);

          return (
            <li key={`${step.key}-${index}`} className="relative">
              <span className="absolute -left-3 flex h-6 w-6 items-center justify-center rounded-full bg-slate-950">
                {appearance.icon}
              </span>

              <article className="flex flex-col gap-4 rounded-xl border border-slate-800/70 bg-slate-950/70 p-5">
                <header className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                  <h3 className="text-lg font-semibold text-slate-100">
                    Step {index + 1}: {step.label}
                  </h3>
                  <span
                    className={`inline-flex items-center rounded-full border px-3 py-1 text-xs font-semibold uppercase tracking-wide ${appearance.badge}`}
                  >
                    {step.status}
                  </span>
                </header>

                {step.error && (
                  <div
                    role="alert"
                    className="flex items-start gap-2 rounded-lg border border-rose-500/40 bg-rose-500/10 p-3 text-sm text-rose-100"
                  >
                    <AlertCircle className="mt-0.5 h-4 w-4 flex-shrink-0" aria-hidden />
                    <p>{step.error}</p>
                  </div>
                )}

                <div className="grid gap-4 md:grid-cols-2">
                  <section aria-label="Step content" className="space-y-3">
                    <div className="flex items-center gap-2 text-sm font-semibold text-slate-200">
                      <FileText className="h-4 w-4 text-brand-300" aria-hidden />
                      <span>Content</span>
                    </div>
                    {contentDetails.length ? (
                      contentDetails.map((detail) => (
                        <div
                          key={`${detail.label}-${detail.value}`}
                          className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3 text-sm text-slate-200"
                        >
                          <p className="text-xs uppercase tracking-wide text-slate-400">{detail.label}</p>
                          <p className="mt-1 whitespace-pre-wrap text-slate-100">{detail.value}</p>
                        </div>
                      ))
                    ) : (
                      <p className="text-sm text-slate-400">
                        No content was provided for this step.
                      </p>
                    )}
                  </section>

                  <section aria-label="Step attachments" className="space-y-3">
                    <div className="flex items-center gap-2 text-sm font-semibold text-slate-200">
                      <Paperclip className="h-4 w-4 text-brand-300" aria-hidden />
                      <span>Attachments</span>
                    </div>
                    {attachmentDetails.length ? (
                      <ul className="space-y-2" role="list">
                        {attachmentDetails.map((detail) => (
                          <li
                            key={`${detail.label}-${detail.value}`}
                            className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3 text-sm text-slate-200"
                          >
                            <p className="text-xs uppercase tracking-wide text-slate-400">{detail.label}</p>
                            <p className="mt-1 text-slate-100">{detail.value}</p>
                          </li>
                        ))}
                      </ul>
                    ) : (
                      <p className="text-sm text-slate-400">
                        No attachments were referenced for this step.
                      </p>
                    )}
                  </section>
                </div>
              </article>
            </li>
          );
        })}
      </ol>
    </section>
  );
};

export default WorkflowViewer;
