import { useTranslation } from 'react-i18next';
import type { Car, AttachmentPreview } from '../types/car';
import WorkflowOverviewCard from './WorkflowOverviewCard';
import WorkflowStepsCard from './WorkflowStepsCard';
import AttachmentsCard from './AttachmentsCard';

interface ContentViewProps {
  car: Car | null;
  attachments?: AttachmentPreview[];
}

const ContentView = ({ car, attachments }: ContentViewProps) => {
  const { t } = useTranslation();

  if (!car) {
    return (
      <section className="rounded-xl border border-slate-800 bg-slate-900/70 p-6 text-sm text-slate-300">
        <h2 className="text-lg font-semibold text-slate-100">{t('content_visualizer_title')}</h2>
        <p className="mt-2 text-slate-400">{t('content_visualizer_empty')}</p>
      </section>
    );
  }

  return (
    <div className="space-y-6">
      <WorkflowOverviewCard car={car} />
      <WorkflowStepsCard car={car} />
      <AttachmentsCard attachments={attachments || []} provenance={car.provenance} />
    </div>
  );
};

export default ContentView;
