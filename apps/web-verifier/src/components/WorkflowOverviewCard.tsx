import { Coins, Cpu, Gauge, Leaf, Calendar, Layers } from 'lucide-react';
import type { Car } from '../types/car';
import { formatDate, formatNumber } from '../utils/textHelpers';

interface WorkflowOverviewCardProps {
  car: Car;
}

const WorkflowOverviewCard = ({ car }: WorkflowOverviewCardProps) => {
  const { run, proof, budgets, sgrade } = car;

  // Format run kind and proof match_kind
  const runKindLabel = run.kind.charAt(0).toUpperCase() + run.kind.slice(1);
  const proofKindLabel = proof.match_kind.charAt(0).toUpperCase() + proof.match_kind.slice(1);

  // Count checkpoints if available
  const checkpointCount = proof.process?.sequential_checkpoints?.length || 0;

  return (
    <div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-6 shadow-lg shadow-slate-950/40">
      <header className="mb-6">
        <p className="text-xs uppercase tracking-[0.3em] text-brand-300">Workflow</p>
        <h2 className="text-2xl font-semibold text-slate-50">Overview</h2>
        <p className="mt-2 text-sm text-slate-400">
          This section shows what this run did: workflow name, models, budgets, and stewardship score recorded in the receipt.
        </p>
      </header>

      <dl className="space-y-4">
        {/* Workflow Name */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
          <Layers className="mt-0.5 h-5 w-5 flex-shrink-0 text-brand-300" aria-hidden />
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate-400">Workflow Name</dt>
            <dd className="text-sm font-medium text-slate-100">{run.name}</dd>
          </div>
        </div>

        {/* Created At */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
          <Calendar className="mt-0.5 h-5 w-5 flex-shrink-0 text-brand-300" aria-hidden />
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate-400">Created</dt>
            <dd className="text-sm font-medium text-slate-100">{formatDate(car.created_at)}</dd>
          </div>
        </div>

        {/* Run & Proof Mode */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
          <Cpu className="mt-0.5 h-5 w-5 flex-shrink-0 text-brand-300" aria-hidden />
          <div>
            <dt className="text-xs uppercase tracking-wide text-slate-400">Run & Proof Mode</dt>
            <dd className="text-sm font-medium text-slate-100">
              {runKindLabel} · {proofKindLabel}
            </dd>
          </div>
        </div>

        {/* Model & Steps */}
        <div className="rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
          <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-slate-400">
            Model & Steps
          </div>
          <div className="mt-2 space-y-1">
            <div className="text-sm text-slate-100">
              <span className="font-medium">{run.model}</span>
            </div>
            <div className="text-sm text-slate-300">
              {run.steps.length} step{run.steps.length !== 1 ? 's' : ''}
              {checkpointCount > 0 && ` · ${checkpointCount} checkpoint${checkpointCount !== 1 ? 's' : ''}`}
            </div>
          </div>
        </div>

        {/* Budgets (if non-zero) */}
        {(budgets.tokens > 0 || budgets.usd > 0 || budgets.nature_cost > 0) && (
          <div className="rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
            <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-slate-400 mb-2">
              <Coins className="h-4 w-4" />
              Budgets
            </div>
            <dl className="space-y-1">
              {budgets.tokens > 0 && (
                <div className="flex justify-between text-sm">
                  <dt className="text-slate-300">Tokens</dt>
                  <dd className="font-medium text-slate-100">{formatNumber(budgets.tokens)}</dd>
                </div>
              )}
              {budgets.usd > 0 && (
                <div className="flex justify-between text-sm">
                  <dt className="text-slate-300">USD</dt>
                  <dd className="font-medium text-slate-100">${budgets.usd.toFixed(2)}</dd>
                </div>
              )}
              {budgets.nature_cost > 0 && (
                <div className="flex justify-between text-sm">
                  <dt className="text-slate-300 flex items-center gap-1">
                    <Leaf className="h-3 w-3" />
                    Nature Cost
                  </dt>
                  <dd className="font-medium text-slate-100">{budgets.nature_cost.toFixed(2)}</dd>
                </div>
              )}
            </dl>
          </div>
        )}

        {/* Stewardship Grade */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/60 px-4 py-3">
          <Gauge className="mt-0.5 h-5 w-5 flex-shrink-0 text-brand-300" aria-hidden />
          <div className="flex-1">
            <dt className="text-xs uppercase tracking-wide text-slate-400">Stewardship Score</dt>
            <dd className="mt-1 flex items-center gap-3">
              <span className="text-2xl font-bold text-slate-100">{sgrade.score}</span>
              <span className="text-sm text-slate-400">/ 100</span>
            </dd>
            {/* Score bar */}
            <div className="mt-2 h-2 w-full rounded-full bg-slate-800">
              <div
                className="h-full rounded-full bg-gradient-to-r from-brand-500 to-emerald-500 transition-all"
                style={{ width: `${sgrade.score}%` }}
              />
            </div>
          </div>
        </div>
      </dl>
    </div>
  );
};

export default WorkflowOverviewCard;
