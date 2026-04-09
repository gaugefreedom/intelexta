import { Coins, Cpu, Gauge, Leaf, Calendar, Layers } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { Car } from '../types/car';
import { formatDate, formatNumber } from '../utils/textHelpers';

interface WorkflowOverviewCardProps {
  car: Car;
}

const WorkflowOverviewCard = ({ car }: WorkflowOverviewCardProps) => {
  const { t, i18n } = useTranslation();
  const { run, proof, budgets, sgrade } = car;

  const runKindLabel = run.kind.charAt(0).toUpperCase() + run.kind.slice(1);
  const proofKindLabel = proof.match_kind.charAt(0).toUpperCase() + proof.match_kind.slice(1);

  const checkpointCount = proof.process?.sequential_checkpoints?.length || 0;

  const formatUsd = (usd: number) => {
    if (usd === 0) {
      return { label: '$0.00' };
    }
    if (usd > 0 && usd < 0.01) {
      return { label: '< $0.01', tooltip: `$${usd.toString()}` };
    }
    return { label: `$${usd.toFixed(2)}` };
  };

  const formatNatureCost = (natureCostKwh: number) => {
    if (natureCostKwh >= 1) {
      return { value: natureCostKwh.toFixed(2), unit: 'kWh' };
    }
    const wh = natureCostKwh * 1000;
    if (wh >= 1) {
      return { value: wh.toFixed(2), unit: 'Wh' };
    }
    const mwh = natureCostKwh * 1_000_000;
    return { value: mwh.toFixed(2), unit: 'mWh' };
  };

  const usdDisplay = formatUsd(budgets.usd);
  const natureCostDisplay = formatNatureCost(budgets.nature_cost);

  return (
    <div className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      <header className="mb-6">
        <p className="text-xs uppercase tracking-[0.3em] font-bold text-slate-400">{t('overview_label')}</p>
        <h2 className="text-xl font-bold text-slate-900 mt-1">{t('overview_title')}</h2>
        <p className="mt-2 text-sm text-slate-500">{t('overview_description')}</p>
      </header>

      <dl className="space-y-3">
        {/* Workflow Name */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <Layers className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div>
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">{t('overview_workflow_name')}</dt>
            <dd className="text-sm font-medium text-slate-700">{run.name}</dd>
          </div>
        </div>

        {/* Created At */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <Calendar className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div>
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">{t('overview_created')}</dt>
            <dd className="text-sm font-medium text-slate-700">{formatDate(car.created_at, i18n.language)}</dd>
          </div>
        </div>

        {/* Run & Proof Mode */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <Cpu className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div>
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">{t('overview_run_proof_mode')}</dt>
            <dd className="text-sm font-medium text-slate-700">
              {runKindLabel} · {proofKindLabel}
            </dd>
          </div>
        </div>

        {/* Model & Steps */}
        <div className="rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <div className="flex items-center gap-2 text-[10px] uppercase font-bold tracking-wide text-slate-400">
            {t('overview_model_steps')}
          </div>
          <div className="mt-2 space-y-1">
            <div className="text-sm text-slate-700">
              <span className="font-medium">{run.model}</span>
            </div>
            <div className="text-xs text-slate-500">
              {t(run.steps.length === 1 ? 'overview_steps_one' : 'overview_steps_other', { count: run.steps.length })}
              {checkpointCount > 0 && ` · ${t(checkpointCount === 1 ? 'overview_checkpoints_one' : 'overview_checkpoints_other', { count: checkpointCount })}`}
            </div>
          </div>
        </div>

        {/* Budgets (if non-zero) */}
        {(budgets.tokens > 0 || budgets.usd > 0 || budgets.nature_cost > 0) && (
          <div className="rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
            <div className="flex items-center gap-2 text-[10px] uppercase font-bold tracking-wide text-slate-400 mb-2">
              <Coins className="h-3.5 w-3.5" />
              {t('overview_budgets')}
            </div>
            <dl className="space-y-1">
              {budgets.tokens > 0 && (
                <div className="flex justify-between text-sm">
                  <dt className="text-slate-500">{t('overview_tokens')}</dt>
                  <dd className="font-medium text-slate-700">{formatNumber(budgets.tokens)}</dd>
                </div>
              )}
              {budgets.usd >= 0 && (
                <div className="flex justify-between text-sm">
                  <dt className="text-slate-500">{t('overview_usd')}</dt>
                  <dd className="font-medium text-slate-700" title={usdDisplay.tooltip}>
                    {usdDisplay.label}
                  </dd>
                </div>
              )}
              {budgets.nature_cost > 0 && (
                <div className="flex justify-between text-sm">
                  <dt className="text-slate-500 flex items-center gap-1">
                    <Leaf className="h-3 w-3 text-emerald-600" />
                    {t('overview_nature_cost')}
                  </dt>
                  <dd className="font-medium text-slate-700">
                    {natureCostDisplay.value} {natureCostDisplay.unit}
                  </dd>
                </div>
              )}
            </dl>
          </div>
        )}

        {/* Stewardship Grade */}
        <div className="flex items-start gap-3 rounded-lg border border-slate-100 bg-slate-50 px-4 py-3">
          <Gauge className="mt-0.5 h-4 w-4 flex-shrink-0 text-emerald-600" aria-hidden />
          <div className="flex-1">
            <dt className="text-[10px] uppercase font-bold tracking-wide text-slate-400">{t('overview_stewardship_score')}</dt>
            <dd className="mt-1 flex items-center gap-3">
              <span className="text-2xl font-bold text-slate-900">{sgrade.score}</span>
              <span className="text-sm text-slate-400">/ 100</span>
            </dd>
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
