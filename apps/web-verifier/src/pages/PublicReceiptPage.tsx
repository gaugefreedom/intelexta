import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import {
  AlertCircle,
  CheckCircle2,
  Loader2,
  ExternalLink,
  Calendar,
  Cpu,
  Gauge,
  Shield,
  AlertTriangle,
  ChevronRight,
  FileText,
  Activity,
  Grid
} from 'lucide-react';
import clsx from 'clsx';
import { initVerifier, verifyCarJson } from '../wasm/loader';
import type { VerificationReport } from '../types/verifier';

const VALIDATOR_API_BASE =
  import.meta.env.VITE_VALIDATOR_API_BASE?.toString() ||
  'https://validator.intelexta.com/api';

// --- TYPES (Unchanged) ---
interface PublicReceiptMeta {
  tier: string;
  engine_name: string;
  privacy_mode: string;
  created_at: string;
}

interface PublicReceiptReport {
  doc_type?: string;
  purpose?: string;
  summary_preview?: string;
  key_claims?: Array<{
    id: string;
    statement_preview?: string;
    support_assessment: string;
    citations_needed?: string[];
  }>;
  factual_reliability?: {
    overall_score_0_100?: number;
    flagged_fragile_sections?: Array<{
      description: string;
      reason_preview?: string;
    }>;
  };
  novelty_assessment?: {
    heuristic_score_0_100?: number;
    interpretation?: string;
  };
  ai_usage_estimate?: {
    model_guess_0_100?: number;
  };
  structure?: {
    section_count?: number;
  };
}

interface PublicReceiptCar {
  id: string;
  run_id: string;
  created_at: string;
  run?: {
    kind?: string;
    name?: string;
    model?: string;
    version?: string;
  };
  policy_ref?: {
    estimator?: string;
  };
  budgets?: {
    usd?: number;
    tokens?: number;
    nature_cost?: number;
  };
  provenance?: Array<{
    claim_type: string;
    sha256: string;
  }>;
  sgrade?: {
    score: number;
    components?: {
      provenance?: number;
      energy?: number;
      replay?: number;
      consent?: number;
      incidents?: number;
    };
  };
  signer_public_key?: string;
  signatures?: string[];
}

interface PublicReceiptResponse {
  receipt_id: string;
  report: PublicReceiptReport;
  receipt: Record<string, unknown>;
  receipt_display: PublicReceiptCar;
  meta: PublicReceiptMeta;
}

type PageStatus = 'loading' | 'success' | 'not_found' | 'error';

// --- HELPERS ---
const formatDate = (dateStr: string) => {
  try {
    return new Intl.DateTimeFormat('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    }).format(new Date(dateStr));
  } catch {
    return dateStr;
  }
};

const supportLabelMap: Record<string, { label: string; color: string }> = {
  well_supported: { label: 'Well Supported', color: 'bg-emerald-100 text-emerald-700 border-emerald-200' },
  partially_supported: { label: 'Partially Supported', color: 'bg-amber-100 text-amber-700 border-amber-200' },
  weakly_supported: { label: 'Weakly Supported', color: 'bg-orange-100 text-orange-700 border-orange-200' },
  unclear: { label: 'Unclear', color: 'bg-rose-100 text-rose-700 border-rose-200' },
};

// --- COMPONENT ---
const PublicReceiptPage = () => {
  const { receiptId } = useParams<{ receiptId: string }>();
  const [pageStatus, setPageStatus] = useState<PageStatus>('loading');
  const [error, setError] = useState<string | null>(null);
  const [publicReceipt, setPublicReceipt] = useState<PublicReceiptResponse | null>(null);
  const [verificationStatus, setVerificationStatus] = useState<'pending' | 'verified' | 'failed'>('pending');

  useEffect(() => {
    if (!receiptId) {
      setPageStatus('not_found');
      return;
    }

    const fetchAndVerify = async () => {
      setPageStatus('loading');
      setError(null);

      try {
        await initVerifier().catch(console.warn);

        const response = await fetch(`${VALIDATOR_API_BASE}/public/r/${receiptId}`);
        if (response.status === 404) {
          setPageStatus('not_found');
          return;
        }
        if (!response.ok) {
          throw new Error(`Failed to fetch receipt: ${response.status}`);
        }

        const data: PublicReceiptResponse = await response.json();
        setPublicReceipt(data);

        try {
          const carJson = JSON.stringify(data.receipt);
          const verification = await verifyCarJson(carJson);
          setVerificationStatus(verification.status === 'verified' ? 'verified' : 'failed');
        } catch (verifyErr) {
          console.warn('WASM verification failed:', verifyErr);
          setVerificationStatus('failed');
        }

        setPageStatus('success');
      } catch (err) {
        console.error('Error loading public receipt:', err);
        setError(err instanceof Error ? err.message : 'Failed to load receipt');
        setPageStatus('error');
      }
    };

    fetchAndVerify();
  }, [receiptId]);

  // --- SUB-COMPONENTS FOR LAYOUT ---
  const AuditorHeader = () => (
    <header className="bg-white border-b border-slate-200 h-14 flex items-center justify-between px-4 lg:px-8 sticky top-0 z-50">
      <div className="flex items-center gap-4">
        <a href="/" className="p-2 text-slate-400 hover:text-slate-600 hover:bg-slate-100 rounded-md transition-colors" title="Back to Home">
          <Grid size={20} />
        </a>
        <div className="flex items-center gap-2 group">
          <div className="w-8 h-8 bg-slate-900 rounded-lg flex items-center justify-center text-white font-bold">
            <img src="/icon.png" className="w-6 h-6 rounded-md" alt="Logo" onError={(e) => e.currentTarget.style.display = 'none'} />
          </div>
          <span className="font-bold text-slate-800 text-lg tracking-tight">
            Intelexta <span className="text-slate-400 font-normal">Auditor</span>
          </span>
        </div>
      </div>
      <div className="text-xs font-medium bg-slate-100 text-slate-600 px-3 py-1.5 rounded-full border border-slate-200 hidden sm:block">
        Public View • No Login Required
      </div>
    </header>
  );

  const AuditorFooter = () => (
    <footer className="border-t bg-white mt-auto">
      <div className="max-w-4xl mx-auto px-4 py-6 text-xs text-slate-400 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div className="flex flex-col gap-1">
          <strong className="text-slate-600 font-semibold">Intelexta Validator</strong>
          <span className="flex items-center gap-2">
            <span className="w-1.5 h-1.5 bg-emerald-500 rounded-full"></span>
            Verified Public Record • CAR v0.3
          </span>
        </div>
        <div className="text-right">
          <span>Built by <a href="https://www.gaugefreedom.com/" className="hover:text-slate-600 underline">Gauge Freedom</a></span>
          <br/>
          <span className="italic">"Exact where possible, accountable where not."</span>
        </div>
      </div>
    </footer>
  );

  // --- STATES ---
  if (pageStatus === 'loading') {
    return (
      <div className="min-h-screen bg-slate-50 flex flex-col font-sans">
        <AuditorHeader />
        <main className="flex-1 flex flex-col items-center justify-center gap-4">
          <Loader2 className="w-10 h-10 animate-spin text-emerald-600" />
          <p className="text-slate-500 font-medium animate-pulse">Verifying cryptographic chain...</p>
        </main>
      </div>
    );
  }

  if (pageStatus === 'not_found' || pageStatus === 'error') {
    return (
      <div className="min-h-screen bg-slate-50 flex flex-col font-sans">
        <AuditorHeader />
        <main className="flex-1 flex flex-col items-center justify-center gap-6 p-4 text-center">
          <div className="w-16 h-16 bg-slate-100 rounded-full flex items-center justify-center">
            <AlertCircle className="w-8 h-8 text-slate-400" />
          </div>
          <div>
            <h1 className="text-xl font-bold text-slate-900">
              {pageStatus === 'not_found' ? 'Receipt Not Found' : 'Error Loading Receipt'}
            </h1>
            <p className="text-slate-500 mt-2 max-w-sm mx-auto">
              {pageStatus === 'not_found' 
                ? "This proof link doesn't exist or hasn't been shared yet." 
                : error || "An unexpected error occurred."}
            </p>
          </div>
          <a href="/" className="px-4 py-2 bg-slate-900 text-white rounded-lg text-sm font-medium hover:bg-slate-800 transition-colors">
            Go to Verifier Home
          </a>
        </main>
        <AuditorFooter />
      </div>
    );
  }

  // --- SUCCESS STATE ---
  const { report, receipt_display, meta } = publicReceipt!;

  return (
    <div className="min-h-screen bg-slate-50 flex flex-col font-sans">
      <AuditorHeader />

      <main className="flex-1 w-full max-w-4xl mx-auto px-4 py-8 flex flex-col gap-6">
        
        {/* Verification Status Banner */}
        <div className={clsx(
          "rounded-xl border p-4 flex items-start gap-4 shadow-sm transition-all",
          verificationStatus === 'verified' ? "bg-emerald-50 border-emerald-200" : "bg-rose-50 border-rose-200"
        )}>
          {verificationStatus === 'verified' ? (
            <CheckCircle2 className="text-emerald-600 h-6 w-6 mt-0.5 shrink-0" />
          ) : (
            <AlertTriangle className="text-rose-600 h-6 w-6 mt-0.5 shrink-0" />
          )}
          <div>
            <h3 className={clsx("text-lg font-bold", verificationStatus === 'verified' ? "text-emerald-900" : "text-rose-900")}>
              {verificationStatus === 'verified' ? 'Receipt Verified' : 'Verification Failed'}
            </h3>
            <p className={clsx("text-sm mt-1 leading-relaxed", verificationStatus === 'verified' ? "text-emerald-700" : "text-rose-700")}>
              {verificationStatus === 'verified' 
                ? "The cryptographic signature and hash chain of this receipt are valid. The content matches the original execution log." 
                : "Could not verify receipt integrity. Data may be incomplete or tampered."}
            </p>
          </div>
        </div>

        {/* Metadata Grid */}
        <section className="bg-white border border-slate-200 rounded-2xl shadow-sm p-5">
          <h2 className="text-sm font-bold text-slate-900 uppercase tracking-wide mb-4 flex items-center gap-2">
            <Activity size={16} className="text-slate-400" /> Receipt Metadata
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="p-3 bg-slate-50 rounded-lg border border-slate-100">
              <div className="text-[10px] uppercase text-slate-400 font-bold mb-1">Receipt ID</div>
              <div className="font-mono text-sm text-slate-700 break-all">{receipt_display.id}</div>
            </div>
            <div className="p-3 bg-slate-50 rounded-lg border border-slate-100">
              <div className="text-[10px] uppercase text-slate-400 font-bold mb-1">Created</div>
              <div className="text-sm text-slate-700 flex items-center gap-2">
                <Calendar size={14} className="text-slate-400" />
                {formatDate(meta.created_at)}
              </div>
            </div>
            <div className="p-3 bg-slate-50 rounded-lg border border-slate-100">
              <div className="text-[10px] uppercase text-slate-400 font-bold mb-1">Engine / Tier</div>
              <div className="text-sm text-slate-700 flex items-center gap-2">
                <Cpu size={14} className="text-slate-400" />
                <span className="font-medium">{meta.tier}</span>
                <span className="text-slate-300">|</span>
                <span>{meta.engine_name}</span>
              </div>
            </div>
            {receipt_display.sgrade && (
               <div className="p-3 bg-slate-50 rounded-lg border border-slate-100">
                <div className="text-[10px] uppercase text-slate-400 font-bold mb-1">Stewardship Score</div>
                <div className="flex items-center gap-3">
                  <Gauge size={14} className="text-emerald-600" />
                  <span className="text-lg font-bold text-slate-900">{receipt_display.sgrade.score}<span className="text-xs text-slate-400 font-normal">/100</span></span>
                  <div className="h-1.5 flex-1 bg-slate-200 rounded-full overflow-hidden">
                    <div className="h-full bg-emerald-500 rounded-full" style={{ width: `${receipt_display.sgrade.score}%` }} />
                  </div>
                </div>
              </div>
            )}
          </div>
        </section>

        {/* Integrity Analysis */}
        <section className="bg-white border border-slate-200 rounded-2xl shadow-sm p-6 space-y-6">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-bold text-slate-900">Integrity Analysis</h2>
            <span className="text-xs font-medium bg-slate-100 text-slate-500 px-2 py-1 rounded">Summary View</span>
          </div>

          {/* Scores Row */}
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
             <ScoreCard 
               label="Factual Reliability" 
               score={report.factual_reliability?.overall_score_0_100} 
               description="Weighted accuracy of key claims."
             />
             <ScoreCard 
               label="Heuristic Novelty" 
               score={report.novelty_assessment?.heuristic_score_0_100} 
               description={report.novelty_assessment?.interpretation}
             />
             <ScoreCard 
               label="AI Usage Estimate" 
               score={report.ai_usage_estimate?.model_guess_0_100} 
               description="Estimated model contribution."
             />
          </div>

          {/* Document Type */}
          {(report.doc_type || report.purpose) && (
            <div className="bg-indigo-50/50 border border-indigo-100 rounded-xl p-4">
               <div className="flex flex-col sm:flex-row gap-4 sm:gap-12">
                 {report.doc_type && (
                   <div>
                     <dt className="text-[10px] uppercase font-bold text-indigo-400">Document Type</dt>
                     <dd className="text-sm font-medium text-indigo-900">{report.doc_type}</dd>
                   </div>
                 )}
                 {report.purpose && (
                   <div>
                     <dt className="text-[10px] uppercase font-bold text-indigo-400">Purpose</dt>
                     <dd className="text-sm font-medium text-indigo-900">{report.purpose}</dd>
                   </div>
                 )}
               </div>
               {report.summary_preview && (
                 <div className="mt-3 pt-3 border-t border-indigo-100/50">
                    <p className="text-sm text-indigo-800/80 italic">"{report.summary_preview}"</p>
                 </div>
               )}
            </div>
          )}

          {/* Key Claims */}
          {report.key_claims && report.key_claims.length > 0 && (
            <div>
              <h3 className="text-sm font-bold text-slate-900 mb-3 flex items-center gap-2">
                 <FileText size={16} /> Key Claims Analysis
              </h3>
              <ul className="space-y-3">
                {report.key_claims.map((claim, idx) => {
                  const support = supportLabelMap[claim.support_assessment] || supportLabelMap.unclear;
                  return (
                    <li key={idx} className="bg-slate-50 rounded-lg border border-slate-200 p-3 flex flex-col sm:flex-row sm:items-start justify-between gap-3">
                      <span className="text-sm text-slate-700 leading-snug">{claim.statement_preview || claim.id}</span>
                      <span className={clsx('shrink-0 text-[10px] font-bold uppercase tracking-wide px-2 py-0.5 rounded border', support.color)}>
                        {support.label}
                      </span>
                    </li>
                  );
                })}
              </ul>
            </div>
          )}

          {/* Fragile Sections */}
          {report.factual_reliability?.flagged_fragile_sections && report.factual_reliability.flagged_fragile_sections.length > 0 && (
            <div className="rounded-xl border border-amber-200 bg-amber-50 p-4">
              <h3 className="text-sm font-bold text-amber-800 mb-2 flex items-center gap-2">
                <AlertTriangle size={16} /> Areas Needing Attention
              </h3>
              <ul className="space-y-2">
                {report.factual_reliability.flagged_fragile_sections.map((section, idx) => (
                  <li key={idx} className="text-sm text-amber-900/80">
                    <span className="font-semibold text-amber-900">{section.description}:</span> {section.reason_preview}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </section>

        {/* CTA */}
        <div className="mt-4 flex flex-col sm:flex-row gap-4 justify-center">
          <a
            href="https://validator.intelexta.com"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center justify-center gap-2 rounded-lg bg-slate-900 px-6 py-3 text-sm font-bold text-white hover:bg-slate-800 transition-all shadow-md"
          >
            Verify your own work <ExternalLink size={14} />
          </a>
          <a
            href="https://intelexta.com"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center justify-center gap-2 rounded-lg border border-slate-200 bg-white px-6 py-3 text-sm font-bold text-slate-700 hover:bg-slate-50 transition-all shadow-sm"
          >
            About the Protocol <ChevronRight size={14} />
          </a>
        </div>

      </main>

      <AuditorFooter />
    </div>
  );
};

// --- HELPER COMPONENT ---
function ScoreCard({ label, score, description }: { label: string, score?: number, description?: string }) {
  if (score === undefined) return null;
  
  let colorClass = "bg-slate-500";
  if (score >= 70) colorClass = "bg-emerald-500";
  else if (score >= 40) colorClass = "bg-amber-500";
  else colorClass = "bg-rose-500";

  return (
    <div className="bg-slate-50 rounded-xl border border-slate-200 p-4 flex flex-col justify-between">
      <div>
        <div className="text-xs font-medium text-slate-500 uppercase tracking-wide">{label}</div>
        <div className="mt-2 text-3xl font-bold text-slate-900">
          {score}<span className="text-sm font-normal text-slate-400">/100</span>
        </div>
        <div className="mt-2 h-1.5 w-full bg-slate-200 rounded-full overflow-hidden">
          <div className={`h-full rounded-full ${colorClass}`} style={{ width: `${score}%` }} />
        </div>
      </div>
      {description && <div className="mt-3 text-xs text-slate-500 leading-tight">{description}</div>}
    </div>
  );
}

export default PublicReceiptPage;