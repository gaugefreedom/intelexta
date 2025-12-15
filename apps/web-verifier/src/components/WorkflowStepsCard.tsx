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
    <div className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
      {/* Step Header */}
      <header className="mb-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h4 className="text-lg font-bold text-slate-900">
              Step {index} – {step.stepType || 'llm'}
            </h4>
            <p className="text-xs font-medium text-slate-500 uppercase tracking-wide mt-1">
              {step.checkpointType}
            </p>
          </div>
          <span className="rounded-full border border-slate-200 bg-slate-100 px-3 py-1 text-xs font-bold text-slate-600 uppercase tracking-wide">
            {step.proofMode}
          </span>
        </div>
      </header>

      {/* Step Details */}
      <dl className="space-y-3">
        {/* Model */}
        {step.model && (
          <div className="rounded-lg border border-slate-100 bg-slate-50 p-3">
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">Model</dt>
            <dd className="mt-1 text-sm font-medium text-slate-700">{step.model}</dd>
          </div>
        )}

        {/* Proof Mode & Epsilon */}
        <div className="grid grid-cols-2 gap-3">
          <div className="rounded-lg border border-slate-100 bg-slate-50 p-3">
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">Proof Mode</dt>
            <dd className="mt-1 text-sm font-medium text-slate-700">{step.proofMode}</dd>
          </div>
          {step.epsilon !== undefined && (
            <div className="rounded-lg border border-slate-100 bg-slate-50 p-3">
              <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">Epsilon</dt>
              <dd className="mt-1 text-sm font-medium text-slate-700">{step.epsilon}</dd>
            </div>
          )}
          {step.epsilon === undefined && (
            <div className="rounded-lg border border-slate-100 bg-slate-50 p-3">
              <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">Token Budget</dt>
              <dd className="mt-1 text-sm font-medium text-slate-700">{formatTokens(step.tokenBudget)}</dd>
            </div>
          )}
        </div>

        {/* Prompt Preview */}
        {promptPreview && (
          <div className="rounded-lg border border-slate-100 bg-slate-50 p-3">
            <div className="flex items-center gap-2 text-[10px] uppercase font-bold tracking-wide text-slate-400 mb-2">
              <FileText className="h-3 w-3 text-emerald-600" />
              Prompt
            </div>
            <dd className="text-sm text-slate-600 whitespace-pre-wrap leading-relaxed">
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
          <div className="rounded-lg border border-slate-100 bg-slate-50 p-3">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2 text-[10px] uppercase font-bold tracking-wide text-slate-400">
                <Code className="h-3 w-3 text-emerald-600" />
                Configuration
              </div>
              {hasFullConfig && (
                <button
                  onClick={() => setIsConfigExpanded(!isConfigExpanded)}
                  className="flex items-center gap-1 text-xs font-medium text-emerald-600 hover:text-emerald-700 transition-colors"
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
            <dd className="overflow-auto rounded-md bg-white border border-slate-200 p-3 shadow-inner">
              <pre className="text-xs leading-relaxed text-slate-600 font-mono">
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
      <div className="rounded-xl border border-slate-200 bg-white p-6 text-center">
        <h3 className="text-lg font-semibold text-slate-900">Workflow Steps</h3>
        <p className="mt-2 text-sm text-slate-500">No steps found in this workflow.</p>
      </div>
    );
  }

  return (
    <div className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <header className="mb-6">
        <p className="text-xs uppercase tracking-[0.3em] font-bold text-slate-400">Steps</p>
        <h3 className="text-xl font-bold text-slate-900 mt-1">Workflow Steps</h3>
        <p className="mt-1 text-sm text-slate-500">
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