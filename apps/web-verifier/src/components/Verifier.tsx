import { useCallback, useEffect, useMemo, useState } from 'react';
import { useDropzone } from 'react-dropzone';
import clsx from 'clsx';
import { AlertCircle, CheckCircle2, Loader2, UploadCloud } from 'lucide-react';
import {
  initVerifier,
  VerificationResult,
  verifyCarBytes,
  verifyCarJson
} from '../wasm/loader';
import WorkflowTimeline from './WorkflowTimeline';
import MetadataCard from './MetadataCard';

type Status = 'idle' | 'loading' | 'success' | 'error';

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

const exampleAccept = {
  'application/vnd.ipld.car': ['.car'],
  'application/json': ['.json']
};

const defaultJsonPlaceholder = `{
  "metadata": {
    "runId": "123",
    "signer": "did:key:z6Mk...",
    "model": "gpt-4.1-mini",
    "createdAt": "2024-05-01T10:34:00Z"
  }
}`;

const Verifier = () => {
  const [status, setStatus] = useState<Status>('idle');
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<VerificationResult | null>(null);
  const [rawJson, setRawJson] = useState<string>('');
  const [droppedFileName, setDroppedFileName] = useState<string | null>(null);

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

    try {
      if (file.name.toLowerCase().endsWith('.json')) {
        const json = await file.text();
        setRawJson(json || defaultJsonPlaceholder);
        const verification = await verifyCarJson(json);
        setResult(verification);
      } else {
        const buffer = await file.arrayBuffer();
        const bytes = new Uint8Array(buffer);
        const verification = await verifyCarBytes(bytes);
        setResult(verification);
        setRawJson(JSON.stringify(verification, null, 2));
      }
      setStatus('success');
    } catch (err) {
      console.error(err);
      setStatus('error');
      setError(err instanceof Error ? err.message : 'Unknown error while verifying file');
    }
  }, []);

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    accept: exampleAccept,
    multiple: false
  });

  const statusMessage = useMemo(() => {
    switch (status) {
      case 'loading':
        return 'Verifying archive with WASM verifier...';
      case 'success':
        return droppedFileName
          ? `Successfully verified \`${droppedFileName}\``
          : 'Verification completed';
      case 'error':
        return error ?? 'Verification failed. Check the console for details.';
      default:
        return 'Drop a .car or .json proof to start verification.';
    }
  }, [status, droppedFileName, error]);

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col gap-8 px-6 py-12">
      <header className="flex flex-col gap-2">
        <p className="text-sm uppercase tracking-[0.35em] text-brand-400">IntelexTA</p>
        <h1 className="text-4xl font-semibold text-white sm:text-5xl">Workflow Proof Verifier</h1>
        <p className="text-base text-slate-300 sm:text-lg">
          Validate signed workflow archives directly in your browser. Upload a CAR bundle
          exported from IntelexTA or drop a JSON transcript to preview steps, prompts, and
          outputs.
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
          {isDragActive ? 'Release to verify your file' : 'Drag & drop a CAR or JSON file here'}
        </p>
        <p className="mt-2 max-w-md text-sm text-slate-400">
          Supports IntelexTA signed CAR archives and JSON transcripts. Files stay in the browser and
          are never uploaded.
        </p>
        {droppedFileName && (
          <p className="mt-4 rounded-full border border-slate-700 bg-slate-800/80 px-5 py-1 text-xs uppercase tracking-wide text-slate-300">
            Last file: {droppedFileName}
          </p>
        )}
      </section>

      <StatusBanner status={status} message={statusMessage} />

      {status === 'error' && error && (
        <div className="rounded-lg border border-rose-600/40 bg-rose-500/10 px-4 py-3 text-sm text-rose-100">
          <p>{error}</p>
          <p className="mt-1 text-xs text-rose-200/70">
            Ensure the WASM bundle is available in <code>public/pkg</code> and the file is a valid
            IntelexTA proof archive.
          </p>
        </div>
      )}

      {result && (
        <section className="grid grid-cols-1 gap-6 lg:grid-cols-[1.2fr_0.8fr]">
          <div className="flex flex-col gap-6">
            <MetadataCard metadata={result.metadata} />
            <WorkflowTimeline steps={result.workflow ?? []} />
          </div>
          <aside className="flex flex-col gap-4">
            <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-5">
              <h2 className="text-lg font-semibold text-slate-100">Raw Output</h2>
              <p className="mb-4 text-sm text-slate-400">
                Review the JSON payload returned from the verifier. Collapse sections to inspect
                prompts, model responses, and signatures.
              </p>
              <pre className="max-h-[420px] overflow-auto rounded-lg bg-slate-950/80 p-4 text-xs leading-relaxed text-slate-200">
                {rawJson || defaultJsonPlaceholder}
              </pre>
            </div>
          </aside>
        </section>
      )}
    </main>
  );
};

export default Verifier;
