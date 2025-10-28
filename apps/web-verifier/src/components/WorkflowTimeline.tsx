import { Fragment } from 'react';
import { CheckCircle2, Circle, Clock3 } from 'lucide-react';
import type { WorkflowStep } from '../wasm/loader';

interface WorkflowTimelineProps {
  steps: WorkflowStep[];
}

const statusIcon = (status: string) => {
  switch (status.toLowerCase()) {
    case 'succeeded':
    case 'success':
      return <CheckCircle2 className="h-5 w-5 text-emerald-400" aria-hidden />;
    case 'running':
    case 'pending':
      return <Clock3 className="h-5 w-5 text-amber-300" aria-hidden />;
    default:
      return <Circle className="h-4 w-4 text-slate-500" aria-hidden />;
  }
};

const WorkflowTimeline = ({ steps }: WorkflowTimelineProps) => {
  if (!steps?.length) {
    return (
      <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-6 text-sm text-slate-400">
        No workflow steps were returned by the verifier. Once a CAR archive is verified the
        timeline will display each action, prompt, and output.
      </div>
    );
  }

  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-6">
      <h2 className="text-lg font-semibold text-slate-100">Workflow Timeline</h2>
      <ol className="mt-6 space-y-6">
        {steps.map((step, index) => (
          <Fragment key={step.id ?? index}>
            <li className="relative pl-10">
              <span className="absolute left-0 top-1 flex h-8 w-8 items-center justify-center rounded-full bg-slate-950/70">
                {statusIcon(step.status ?? 'pending')}
              </span>
              <div className="flex flex-col gap-2">
                <div className="flex items-center gap-2">
                  <p className="text-base font-medium text-slate-100">
                    {step.label ?? `Step ${index + 1}`}
                  </p>
                  <span className="rounded-full border border-slate-700/60 bg-slate-800/80 px-2 py-0.5 text-xs uppercase tracking-wide text-slate-300">
                    {step.status ?? 'unknown'}
                  </span>
                </div>
                {(step.started_at || step.finished_at) && (
                  <p className="text-xs text-slate-400">
                    {step.started_at && <span>Started: {new Date(step.started_at).toLocaleString()}</span>}
                    {step.started_at && step.finished_at && <span className="mx-2">•</span>}
                    {step.finished_at && (
                      <span>Completed: {new Date(step.finished_at).toLocaleString()}</span>
                    )}
                  </p>
                )}
                {step.prompt && (
                  <div className="rounded-lg border border-slate-800/60 bg-slate-950/60 p-3 text-xs text-slate-300">
                    <p className="mb-1 font-semibold text-slate-200">Prompt</p>
                    <p className="whitespace-pre-wrap text-slate-300">{step.prompt}</p>
                  </div>
                )}
                {step.output && (
                  <div className="rounded-lg border border-slate-800/60 bg-slate-950/60 p-3 text-xs text-slate-300">
                    <p className="mb-1 font-semibold text-slate-200">Output</p>
                    <p className="whitespace-pre-wrap text-slate-300">{step.output}</p>
                  </div>
                )}
              </div>
            </li>
            {index < steps.length - 1 && (
              <div className="ml-[1.125rem] h-6 w-px bg-slate-800" aria-hidden />
            )}
          </Fragment>
        ))}
      </ol>
    </div>
  );
};

export default WorkflowTimeline;
