import { useCallback, useEffect, useState } from 'react';
import { type FileRejection, useDropzone } from 'react-dropzone';
import clsx from 'clsx';
import { AlertCircle, CheckCircle2, Loader2, UploadCloud, FileJson, Package } from 'lucide-react';
import { initVerifier, verifyCarBytes, verifyCarJson } from '../wasm/loader';
import type { VerificationReport } from '../types/verifier';
import type { Car, AttachmentPreview } from '../types/car';
import WorkflowViewer from './WorkflowViewer';
import MetadataCard from './MetadataCard';
import ContentView from './ContentView';
import { parseCarZip } from '../utils/zipParser';
import { Layout } from './Layout';
import {
  PROOF_FILE_ACCEPT_MESSAGE,
  buildProofDropzoneAccept,
  proofFileValidator,
  validateProofFileName
} from '../utils/proofFiles';

type Status = 'idle' | 'loading' | 'success' | 'error';
type ViewMode = 'verify' | 'content';

const LoadingSkeleton = () => (
  <div className="flex flex-col items-center justify-center py-20 gap-4 text-slate-500">
    <Loader2 className="w-10 h-10 animate-spin text-emerald-600" />
    <p className="font-medium animate-pulse">Verifying cryptographic proofs...</p>
  </div>
);

const Verifier = () => {
  const [status, setStatus] = useState<Status>('idle');
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<VerificationReport | null>(null);
  const [rawJson, setRawJson] = useState<string>('');
  const [droppedFileName, setDroppedFileName] = useState<string | null>(null);
  const [fileKind, setFileKind] = useState<'json' | 'car' | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('verify');
  const [parsedCar, setParsedCar] = useState<Car | null>(null);
  const [attachments, setAttachments] = useState<AttachmentPreview[]>([]);

  useEffect(() => {
    initVerifier().catch((err) => console.warn('Verifier init warning', err));
  }, []);

  const handleReset = useCallback(() => {
    setStatus('idle');
    setResult(null);
    setDroppedFileName(null);
    setError(null);
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
    setFileKind(null);

    const validation = validateProofFileName(file.name);
    if (!validation.valid) {
      setStatus('error');
      setError(validation.message);
      return;
    }

    setFileKind(validation.kind);

    try {
      if (validation.kind === 'json') {
        const json = await file.text();
        try {
          const carData = JSON.parse(json) as Car;
          setParsedCar(carData);
          setAttachments([]);
        } catch (parseErr) {
          console.warn('Failed to parse CAR JSON:', parseErr);
        }
        const verification = await verifyCarJson(json);
        setResult(verification);
        setRawJson(JSON.stringify(verification, null, 2));
        setStatus(verification.status === 'verified' ? 'success' : 'error');
        if (verification.status !== 'verified') setError(verification.error || 'Verification failed');
      } else {
        try {
          const { car, attachments: extractedAttachments } = await parseCarZip(file);
          setParsedCar(car);
          setAttachments(extractedAttachments);
        } catch (parseErr) {
          console.warn('Failed to parse ZIP:', parseErr);
        }

        const buffer = await file.arrayBuffer();
        const bytes = new Uint8Array(buffer);
        const verification = await verifyCarBytes(bytes);
        setResult(verification);
        setRawJson(JSON.stringify(verification, null, 2));
        setStatus(verification.status === 'verified' ? 'success' : 'error');
        if (verification.status !== 'verified') setError(verification.error || 'Verification failed');
      }
    } catch (err) {
      console.error(err);
      setStatus('error');
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  }, []);

  const onDropRejected = useCallback((fileRejections: FileRejection[]) => {
    if (!fileRejections.length) return;
    setStatus('error');
    setError(fileRejections[0].errors[0]?.message ?? PROOF_FILE_ACCEPT_MESSAGE);
  }, []);

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    onDropRejected,
    accept: buildProofDropzoneAccept(),
    validator: proofFileValidator,
    multiple: false
  });

  return (
    <Layout 
      viewMode={viewMode} 
      setViewMode={setViewMode} 
      hasResult={!!result}
      fileName={droppedFileName}
      status={status}
      onReset={handleReset}
    >
      {/* 1. IDLE STATE: Large Dropzone */}
      {!result && status !== 'loading' && (
        <div className="max-w-2xl mx-auto mt-12">
          <div
            {...getRootProps({
              className: clsx(
                'group relative flex flex-col items-center justify-center rounded-2xl border-2 border-dashed px-10 py-20 text-center transition-all cursor-pointer',
                isDragActive 
                  ? 'border-emerald-500 bg-emerald-50/50 shadow-lg scale-[1.02]' 
                  : 'border-slate-300 bg-white hover:border-emerald-400 hover:bg-slate-50 hover:shadow-md'
              )
            })}
          >
            <input {...getInputProps()} />
            <div className={`p-4 rounded-full mb-4 transition-colors ${isDragActive ? 'bg-emerald-100 text-emerald-600' : 'bg-slate-100 text-slate-400 group-hover:bg-emerald-50 group-hover:text-emerald-500'}`}>
               <UploadCloud className="h-10 w-10" />
            </div>
            
            <h2 className="text-xl font-bold text-slate-900 mb-2">
              {isDragActive ? 'Drop receipt here' : 'Verify a Workflow Receipt'}
            </h2>
            <p className="text-slate-500 max-w-sm mx-auto mb-6">
              Drag and drop a <code>.car.json</code> or <code>.car.zip</code> file to verify its cryptographic integrity.
            </p>
            
            <div className="flex gap-4 text-xs text-slate-400">
               <span className="flex items-center gap-1"><FileJson size={14}/> .car.json</span>
               <span className="flex items-center gap-1"><Package size={14}/> .car.zip</span>
            </div>
          </div>
          
          {error && (
            <div className="mt-6 rounded-lg border border-rose-200 bg-rose-50 p-4 text-sm text-rose-700 flex items-start gap-3 shadow-sm">
              <AlertCircle className="h-5 w-5 shrink-0 text-rose-500" />
              <div>
                <strong className="font-semibold block mb-1">Verification Error</strong>
                {error}
              </div>
            </div>
          )}
        </div>
      )}

      {/* 2. LOADING STATE */}
      {status === 'loading' && <LoadingSkeleton />}

      {/* 3. RESULT STATE */}
      {result && (
        <div className="animate-in fade-in slide-in-from-bottom-4 duration-500">
          {/* Status Alert */}
          <div className={clsx(
            "mb-6 rounded-xl border p-4 flex items-start gap-4 shadow-sm",
            status === 'success' ? "bg-emerald-50 border-emerald-200" : "bg-rose-50 border-rose-200"
          )}>
            {status === 'success' ? <CheckCircle2 className="text-emerald-600 h-6 w-6 mt-0.5" /> : <AlertCircle className="text-rose-600 h-6 w-6 mt-0.5" />}
            <div>
              <h3 className={clsx("text-lg font-bold", status === 'success' ? "text-emerald-900" : "text-rose-900")}>
                {status === 'success' ? 'Receipt Verified' : 'Verification Failed'}
              </h3>
              <p className={clsx("text-sm mt-1", status === 'success' ? "text-emerald-700" : "text-rose-700")}>
                {status === 'success' 
                  ? "The cryptographic signature and hash chain of this receipt are valid. The content has not been tampered with since generation."
                  : error || "Critical integrity check failed."}
              </p>
            </div>
          </div>

          {viewMode === 'verify' && (
            <div className="grid grid-cols-1 lg:grid-cols-[1fr_350px] gap-6">
              <div className="space-y-6">
                <WorkflowViewer report={result} />
              </div>
              <aside className="space-y-6">
                <MetadataCard report={result} />
                <div className="rounded-xl border border-slate-200 bg-white shadow-sm overflow-hidden">
                  <div className="bg-slate-50 border-b border-slate-200 px-4 py-3">
                    <h3 className="text-sm font-semibold text-slate-700">Raw JSON Output</h3>
                  </div>
                  <pre className="p-4 text-[10px] leading-relaxed text-slate-600 font-mono overflow-auto max-h-[300px]">
                    {rawJson}
                  </pre>
                </div>
              </aside>
            </div>
          )}

          {viewMode === 'content' && (
            <ContentView car={parsedCar} attachments={attachments} />
          )}
        </div>
      )}
    </Layout>
  );
};

export default Verifier;