import { useState } from 'react';
import { ChevronDown, ChevronUp, Code, FileText } from 'lucide-react';
import type { Car, RunStep } from '../types/car';
import { truncateText, truncateJson, formatTokens } from '../utils/textHelpers';

interface WorkflowStepsCardProps {
  car: Car;
}

interface StepItemProps {
  step: RunStep;
  index: number;
}

const StepItem = ({ step, index }: StepItemProps) => {
  const [isConfigExpanded, setIsConfigExpanded] = useState(false);

  // Truncate prompt for preview
  const promptPreview = step.prompt ? truncateText(step.prompt, 240) : null;
  const hasLongerPrompt = step.prompt && step.prompt.length > 240;

  // Truncate config JSON
  const configPreview = step.configJson ? truncateJson(step.configJson, 160) : null;
  const hasFullConfig = step.configJson && step.configJson.length > 160;

  return (
    <div className="rounded-xl border border-slate-800/70 bg-slate-950/70 p-5">
      {/* Step Header */}
      <header className="mb-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h4 className="text-lg font-semibold text-slate-100">
              Step {index} – {step.stepType || 'llm'}
            </h4>
            <p className="text-sm text-slate-400">{step.checkpointType}</p>
          </div>
          <span className="rounded-full border border-slate-700 bg-slate-800/80 px-3 py-1 text-xs font-medium text-slate-300">
            {step.proofMode}
          </span>
        </div>
      </header>

      {/* Step Details */}
      <dl className="space-y-3">
        {/* Model */}
        {step.model && (
          <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
            <dt className="text-xs uppercase tracking-wide text-slate-400">Model</dt>
            <dd className="mt-1 text-sm font-medium text-slate-100">{step.model}</dd>
          </div>
        )}

        {/* Proof Mode & Epsilon */}
        <div className="grid grid-cols-2 gap-3">
          <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
            <dt className="text-xs uppercase tracking-wide text-slate-400">Proof Mode</dt>
            <dd className="mt-1 text-sm font-medium text-slate-100">{step.proofMode}</dd>
          </div>
          {step.epsilon !== undefined && (
            <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
              <dt className="text-xs uppercase tracking-wide text-slate-400">Epsilon</dt>
              <dd className="mt-1 text-sm font-medium text-slate-100">{step.epsilon}</dd>
            </div>
          )}
          {step.epsilon === undefined && (
            <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
              <dt className="text-xs uppercase tracking-wide text-slate-400">Token Budget</dt>
              <dd className="mt-1 text-sm font-medium text-slate-100">{formatTokens(step.tokenBudget)}</dd>
            </div>
          )}
        </div>

        {/* Prompt Preview */}
        {promptPreview && (
          <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
            <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-slate-400 mb-2">
              <FileText className="h-3 w-3" />
              Prompt
            </div>
            <dd className="text-sm text-slate-200 whitespace-pre-wrap leading-relaxed">
              {promptPreview}
            </dd>
            {hasLongerPrompt && (
              <p className="mt-2 text-xs text-slate-400 italic">
                (Prompt truncated for preview)
              </p>
            )}
          </div>
        )}

        {/* Config JSON */}
        {configPreview && (
          <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-slate-400">
                <Code className="h-3 w-3" />
                Configuration
              </div>
              {hasFullConfig && (
                <button
                  onClick={() => setIsConfigExpanded(!isConfigExpanded)}
                  className="flex items-center gap-1 text-xs text-brand-400 hover:text-brand-300 transition-colors"
                  aria-expanded={isConfigExpanded}
                >
                  {isConfigExpanded ? (
                    <>
                      <ChevronUp className="h-3 w-3" />
                      Collapse
                    </>
                  ) : (
                    <>
                      <ChevronDown className="h-3 w-3" />
                      Expand
                    </>
                  )}
                </button>
              )}
            </div>
            <dd className="overflow-auto rounded-md bg-slate-950/80 p-3">
              <pre className="text-xs leading-relaxed text-slate-200 font-mono">
                {isConfigExpanded ? step.configJson : configPreview}
              </pre>
            </dd>
          </div>
        )}
      </dl>
    </div>
  );
};

const WorkflowStepsCard = ({ car }: WorkflowStepsCardProps) => {
  const steps = car.run.steps;

  if (steps.length === 0) {
    return (
      <div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-6">
        <h3 className="text-lg font-semibold text-slate-100">Workflow Steps</h3>
        <p className="mt-2 text-sm text-slate-400">No steps found in this workflow.</p>
      </div>
    );
  }

  return (
    <div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-6 shadow-lg shadow-slate-950/40">
      <header className="mb-6">
        <p className="text-xs uppercase tracking-[0.3em] text-brand-300">Steps</p>
        <h3 className="text-2xl font-semibold text-slate-50">Workflow Steps</h3>
        <p className="mt-1 text-sm text-slate-400">
          {steps.length} step{steps.length !== 1 ? 's' : ''} configured
        </p>
      </header>

      <div className="space-y-4">
        {steps.map((step, index) => (
          <StepItem key={step.id} step={step} index={index} />
        ))}
      </div>
    </div>
  );
};

export default WorkflowStepsCard;
