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
    <div className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <header className="mb-6">
        <p className="text-xs uppercase tracking-[0.3em] font-bold text-slate-400">Workflow</p>
        <h2 className="text-xl font-bold text-slate-900 mt-1">Overview</h2>
        <p className="mt-2 text-sm text-slate-500">
          This section shows what this run did: workflow name, models, budgets, and stewardship score recorded in the receipt.
        </p>
      </header>

      <dl className="space-y-3">
        {/* Workflow Name */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <Layers className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div>
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">Workflow Name</dt>
            <dd className="text-sm font-medium text-slate-700">{run.name}</dd>
          </div>
        </div>

        {/* Created At */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <Calendar className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div>
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">Created</dt>
            <dd className="text-sm font-medium text-slate-700">{formatDate(car.created_at)}</dd>
          </div>
        </div>

        {/* Run & Proof Mode */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <Cpu className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div>
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">Run & Proof Mode</dt>
            <dd className="text-sm font-medium text-slate-700">
              {runKindLabel} · {proofKindLabel}
            </dd>
          </div>
        </div>

        {/* Model & Steps */}
        <div className="rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <div className="flex items-center gap-2 text-[10px] uppercase font-bold tracking-wide text-slate-400">
            Model & Steps
          </div>
          <div className="mt-2 space-y-1">
            <div className="text-sm text-slate-700">
              <span className="font-medium">{run.model}</span>
            </div>
            <div className="text-xs text-slate-500">
              {run.steps.length} step{run.steps.length !== 1 ? 's' : ''}
              {checkpointCount > 0 && ` · ${checkpointCount} checkpoint${checkpointCount !== 1 ? 's' : ''}`}
            </div>
          </div>
        </div>

        {/* Budgets (if non-zero) */}
        {(budgets.tokens > 0 || budgets.usd > 0 || budgets.nature_cost > 0) && (
          <div className="rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
            <div className="flex items-center gap-2 text-[10px] uppercase font-bold tracking-wide text-slate-400 mb-2">
              <Coins className="h-3.5 w-3.5" />
              Budgets
            </div>
            <dl className="space-y-1">
              {budgets.tokens > 0 && (
                <div className="flex justify-between text-sm">
                  <dt className="text-slate-500">Tokens</dt>
                  <dd className="font-medium text-slate-700">{formatNumber(budgets.tokens)}</dd>
                </div>
              )}
              {budgets.usd > 0 && (
                <div className="flex justify-between text-sm">
                  <dt className="text-slate-500">USD</dt>
                  <dd className="font-medium text-slate-700">${budgets.usd.toFixed(2)}</dd>
                </div>
              )}
              {budgets.nature_cost > 0 && (
                <div className="flex justify-between text-sm">
                  <dt className="text-slate-500 flex items-center gap-1">
                    <Leaf className="h-3 w-3 text-emerald-600" />
                    Nature Cost
                  </dt>
                  <dd className="font-medium text-slate-700">{budgets.nature_cost.toFixed(2)}</dd>
                </div>
              )}
            </dl>
          </div>
        )}

        {/* Stewardship Grade */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <Gauge className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div className="flex-1">
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">Stewardship Score</dt>
            <dd className="mt-1 flex items-center gap-3">
              <span className="text-2xl font-bold text-slate-900">{sgrade.score}</span>
              <span className="text-sm text-slate-400">/ 100</span>
            </dd>
            {/* Score bar */}
            <div className="mt-2 h-1.5 w-full rounded-full bg-slate-200 overflow-hidden">
              <div
                className="h-full rounded-full bg-emerald-500 transition-all"
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