import type { Car, AttachmentPreview } from '../types/car';
import WorkflowOverviewCard from './WorkflowOverviewCard';
import WorkflowStepsCard from './WorkflowStepsCard';
import AttachmentsCard from './AttachmentsCard';

interface ContentViewProps {
  car: Car | null;
  attachments?: AttachmentPreview[];
}

const ContentView = ({ car, attachments }: ContentViewProps) => {
  if (!car) {
    return (
      <section className="rounded-xl border border-slate-800 bg-slate-900/70 p-6 text-sm text-slate-300">
        <h2 className="text-lg font-semibold text-slate-100">Content Visualizer</h2>
        <p className="mt-2 text-slate-400">
          Upload a CAR file to visualize workflow content, steps, and attachments.
        </p>
      </section>
    );
  }

  return (
    <div className="space-y-6">
      {/* Workflow Overview */}
      <WorkflowOverviewCard car={car} />

      {/* Workflow Steps */}
      <WorkflowStepsCard car={car} />

      {/* Attachments Card - show extracted files from ZIP */}
      <AttachmentsCard attachments={attachments || []} provenance={car.provenance} />
    </div>
  );
};

export default ContentView;
