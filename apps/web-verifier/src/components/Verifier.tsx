import { useCallback, useEffect, useMemo, useState } from 'react';
import { type FileRejection, useDropzone } from 'react-dropzone';
import clsx from 'clsx';
import { AlertCircle, CheckCircle2, Loader2, UploadCloud } from 'lucide-react';
import { initVerifier, verifyCarBytes, verifyCarJson } from '../wasm/loader';
import type { VerificationReport } from '../types/verifier';
import type { Car, AttachmentPreview } from '../types/car';
import WorkflowViewer from './WorkflowViewer';
import MetadataCard from './MetadataCard';
import ContentView from './ContentView';
import { parseCarZip } from '../utils/zipParser';
import {
  PROOF_FILE_ACCEPT_MESSAGE,
  buildProofDropzoneAccept,
  proofFileValidator,
  validateProofFileName
} from '../utils/proofFiles';

type Status = 'idle' | 'loading' | 'success' | 'error';
type ViewMode = 'verify' | 'content';

const StatusBanner = ({ status, message }: { status: Status; message: string }) => {
  const icon = {
    idle: UploadCloud,
    loading: Loader2,
    success: CheckCircle2,
    error: AlertCircle
  }[status];

  const Icon = icon;

  return (
    <div
      className={clsx(
        'flex items-center gap-3 rounded-lg border px-4 py-3 text-sm shadow',
        {
          'border-slate-700 bg-slate-900/80 text-slate-300': status === 'idle',
          'border-slate-700 bg-slate-900/80 text-slate-200': status === 'loading',
          'border-emerald-500/40 bg-emerald-500/10 text-emerald-200': status === 'success',
          'border-rose-500/40 bg-rose-500/10 text-rose-200': status === 'error'
        }
      )}
    >
      <Icon className={clsx('h-5 w-5', status === 'loading' && 'animate-spin')} />
      <span>{message}</span>
    </div>
  );
};

const defaultJsonPlaceholder = `{
  "status": "verified",
  "car_id": "car:123...",
  "run_id": "run-demo",
  "created_at": "2024-05-01T10:34:00Z",
  "model": {
    "name": "gpt-4.1-mini",
    "version": "2024-05-01",
    "kind": "text"
  },
  "summary": {
    "checkpoints_verified": 0,
    "checkpoints_total": 0,
    "provenance_verified": 0,
    "provenance_total": 0,
    "attachments_verified": 0,
    "attachments_total": 0,
    "hash_chain_valid": false,
    "signatures_valid": false,
    "content_integrity_valid": false
  },
  "workflow": {
    "steps": []
  }
}`;

const LoadingSkeleton = () => (
  <section className="grid grid-cols-1 gap-6 animate-pulse lg:grid-cols-[minmax(0,1fr)_360px]">
    <div className="rounded-2xl border border-slate-800 bg-slate-900/50 p-6">
      <div className="h-6 w-48 rounded bg-slate-800/70" />
      <div className="mt-5 space-y-3">
        <div className="h-4 rounded bg-slate-800/60" />
        <div className="h-4 rounded bg-slate-800/60" />
        <div className="h-4 rounded bg-slate-800/50" />
        <div className="h-4 rounded bg-slate-800/50" />
      </div>
    </div>
    <aside className="flex flex-col gap-4">
      <div className="rounded-2xl border border-slate-800 bg-slate-900/50 p-5">
        <div className="h-5 w-32 rounded bg-slate-800/70" />
        <div className="mt-4 space-y-2">
          <div className="h-3 rounded bg-slate-800/60" />
          <div className="h-3 rounded bg-slate-800/60" />
          <div className="h-3 rounded bg-slate-800/60" />
        </div>
      </div>
      <div className="rounded-2xl border border-slate-800 bg-slate-900/50 p-5">
        <div className="h-5 w-24 rounded bg-slate-800/70" />
        <div className="mt-4 space-y-2">
          <div className="h-3 rounded bg-slate-800/60" />
          <div className="h-3 rounded bg-slate-800/60" />
          <div className="h-3 rounded bg-slate-800/60" />
        </div>
      </div>
    </aside>
  </section>
);

const ErrorAlert = ({ message, rawJson }: { message: string; rawJson?: string }) => (
  <div className="space-y-3 rounded-lg border border-rose-600/60 bg-rose-900/40 p-4 text-sm text-rose-100">
    <p className="font-semibold">{message}</p>
    {rawJson ? (
      <pre className="max-h-64 overflow-auto rounded-md border border-rose-600/40 bg-rose-950/70 p-3 text-xs leading-relaxed text-rose-100/90">
        {rawJson}
      </pre>
    ) : null}
    <p className="text-xs text-rose-200/70">
      Ensure the WASM bundle is available in <code>public/pkg</code> and that the file was exported from Intelexta.
    </p>
  </div>
);

const Verifier = () => {
  const [status, setStatus] = useState<Status>('idle');
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<VerificationReport | null>(null);
  const [rawJson, setRawJson] = useState<string>('');
  const [droppedFileName, setDroppedFileName] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('verify');
  const [parsedCar, setParsedCar] = useState<Car | null>(null);
  const [attachments, setAttachments] = useState<AttachmentPreview[]>([]);

  useEffect(() => {
    initVerifier().catch((err) => {
      console.warn('Failed to eagerly initialise verifier', err);
    });
  }, []);

  const onDrop = useCallback(async (acceptedFiles: File[]) => {
    if (!acceptedFiles.length) return;

    const file = acceptedFiles[0];
    setDroppedFileName(file.name);
    setStatus('loading');
    setError(null);
    setResult(null);
    setRawJson('');
    setParsedCar(null);
    setAttachments([]);

    const validation = validateProofFileName(file.name);
    if (!validation.valid) {
      setStatus('error');
      setError(validation.message);
      return;
    }

    try {
      if (validation.kind === 'json') {
        const json = await file.text();
        // Parse the CAR JSON for content view
        try {
          const carData = JSON.parse(json) as Car;
          setParsedCar(carData);
          // No attachments for JSON-only files
          setAttachments([]);
        } catch (parseErr) {
          console.warn('Failed to parse CAR JSON for content view:', parseErr);
        }
        const verification = await verifyCarJson(json);
        setResult(verification);
        setRawJson(JSON.stringify(verification, null, 2));
        if (verification.status === 'verified') {
          setStatus('success');
        } else {
          setStatus('error');
          setError(verification.error ?? 'Verification failed. Review the raw output for details.');
        }
      } else {
        // ZIP file - extract CAR and attachments for content view
        try {
          const { car, attachments: extractedAttachments } = await parseCarZip(file);
          setParsedCar(car);
          setAttachments(extractedAttachments);
        } catch (parseErr) {
          console.warn('Failed to parse ZIP for content view:', parseErr);
          // Continue with verification even if content parsing fails
        }

        // Verify using WASM (as before)
        const buffer = await file.arrayBuffer();
        const bytes = new Uint8Array(buffer);
        const verification = await verifyCarBytes(bytes);
        setResult(verification);
        setRawJson(JSON.stringify(verification, null, 2));
        if (verification.status === 'verified') {
          setStatus('success');
        } else {
          setStatus('error');
          setError(verification.error ?? 'Verification failed. Review the raw output for details.');
        }
      }
    } catch (err) {
      console.error(err);
      setStatus('error');
      setResult(null);
      setRawJson('');
      setParsedCar(null);
      setAttachments([]);
      setError(err instanceof Error ? err.message : 'Unknown error while verifying file');
    }
  }, []);

  const onDropRejected = useCallback((fileRejections: FileRejection[]) => {
    if (!fileRejections.length) return;
    const [rejection] = fileRejections;
    const firstError = rejection.errors[0];
    setDroppedFileName(null);
    setResult(null);
    setRawJson('');
    setStatus('error');
    setError(firstError?.message ?? PROOF_FILE_ACCEPT_MESSAGE);
  }, []);

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    onDropRejected,
    accept: buildProofDropzoneAccept(),
    validator: proofFileValidator,
    multiple: false
  });

  const statusMessage = useMemo(() => {
    switch (status) {
      case 'loading':
        return 'Verifying proof with WASM verifier...';
      case 'success':
        if (droppedFileName) {
          const isZip = droppedFileName.endsWith('.car.zip');
          const hint = isZip
            ? ' — You can extract the archive to examine individual artifacts (summary.md, manifest.json, receipts/)'
            : '';
          return `Successfully verified \`${droppedFileName}\`${hint}`;
        }
        return 'Verification completed.';
      case 'error':
        return error ?? 'Verification failed. Check the details below.';
      default:
        return 'Drop a .car.json transcript or .car.zip archive to start verification.';
    }
  }, [status, droppedFileName, error]);

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col gap-8 px-6 py-12">
      <header className="flex flex-col gap-2">
        <p className="text-sm uppercase tracking-[0.35em] text-brand-400">Intelexta</p>
        <h1 className="text-4xl font-semibold text-white sm:text-5xl">Workflow Proof Verifier</h1>
        <p className="text-base text-slate-300 sm:text-lg">
          Validate signed workflow archives directly in your browser. Upload a CAR bundle exported from Intelexta or drop a JSON transcript to preview steps, prompts, and outputs.
        </p>
      </header>

      <section
        {...getRootProps({
          className: clsx(
            'group relative flex flex-col items-center justify-center rounded-2xl border-2 border-dashed px-10 py-16 text-center transition',
            'bg-slate-900/60 backdrop-blur hover:border-brand-500/70 hover:bg-slate-900/80',
            isDragActive ? 'border-brand-500 text-brand-100 shadow-lg' : 'border-slate-700'
          )
        })}
      >
        <input {...getInputProps()} />
        <UploadCloud className="mb-4 h-12 w-12 text-brand-300" />
        <p className="text-lg font-medium text-slate-100">
          {isDragActive
            ? 'Release to verify your file'
            : 'Drag & drop a .car.json or .car.zip file here'}
        </p>
        <p className="mt-2 max-w-md text-sm text-slate-400">
          Supports Intelexta signed CAR archives and JSON transcripts. Files stay in the browser and are never uploaded.
        </p>
        {droppedFileName && (
          <p className="mt-4 rounded-full border border-slate-700 bg-slate-800/80 px-5 py-1 text-xs uppercase tracking-wide text-slate-300">
            Last file: {droppedFileName}
          </p>
        )}
      </section>

      <StatusBanner status={status} message={statusMessage} />

      {/* View Mode Toggle */}
      {result && status !== 'loading' && (
        <div className="flex items-center gap-2 rounded-lg border border-slate-800 bg-slate-900/60 p-1">
          <button
            onClick={() => setViewMode('verify')}
            className={clsx(
              'flex-1 rounded-md px-4 py-2 text-sm font-medium transition-all',
              viewMode === 'verify'
                ? 'bg-brand-500 text-white shadow-lg shadow-brand-500/20'
                : 'text-slate-400 hover:text-slate-200'
            )}
          >
            Verification
          </button>
          <button
            onClick={() => setViewMode('content')}
            className={clsx(
              'flex-1 rounded-md px-4 py-2 text-sm font-medium transition-all',
              viewMode === 'content'
                ? 'bg-brand-500 text-white shadow-lg shadow-brand-500/20'
                : 'text-slate-400 hover:text-slate-200'
            )}
          >
            Visualize Content
          </button>
        </div>
      )}

      {status === 'error' && error && (
        <ErrorAlert message={error} rawJson={rawJson || undefined} />
      )}

      {status === 'loading' && <LoadingSkeleton />}

      {result && status !== 'loading' && viewMode === 'verify' && (
        <section className="grid grid-cols-1 gap-6 lg:grid-cols-[minmax(0,1fr)_360px]">
          <WorkflowViewer report={result} />
          <aside className="flex flex-col gap-4">
            <MetadataCard report={result} />
            <div className="rounded-2xl border border-slate-800 bg-slate-900/70 p-5">
              <h2 className="text-lg font-semibold text-slate-100">Raw Output</h2>
              <p className="mb-4 text-sm text-slate-400">
                Review the normalized JSON payload returned from the verifier.
              </p>
              <pre className="max-h-[420px] overflow-auto rounded-lg bg-slate-950/80 p-4 text-xs leading-relaxed text-slate-200">
                {rawJson || defaultJsonPlaceholder}
              </pre>
            </div>
          </aside>
        </section>
      )}

      {result && status !== 'loading' && viewMode === 'content' && (
        <ContentView car={parsedCar} attachments={attachments} />
      )}
    </main>
  );
};

export default Verifier;
