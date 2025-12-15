import React from "react";
import { ExternalLink, Shield, FileText, UploadCloud, Grid } from "lucide-react";

interface LayoutProps {
  children: React.ReactNode;
  viewMode: 'verify' | 'content';
  setViewMode: (mode: 'verify' | 'content') => void;
  hasResult: boolean;
  fileName?: string | null;
  status?: 'idle' | 'loading' | 'success' | 'error';
  onReset?: () => void;
}

export function Layout({ 
  children, 
  viewMode, 
  setViewMode, 
  hasResult, 
  fileName,
  status,
  onReset
}: LayoutProps) {
  
  return (
    <div className="min-h-screen bg-slate-50 flex flex-col font-sans text-slate-900">
      {/* 1. GLOBAL HEADER */}
      <header className="bg-white border-b border-slate-200 h-14 flex items-center justify-between px-4 lg:px-8 sticky top-0 z-50">
        <div className="flex items-center gap-4">
          <button className="p-2 text-slate-400 hover:text-slate-600 hover:bg-slate-100 rounded-md transition-colors">
            <Grid size={20} />
          </button>
          
          <a href="/" className="flex items-center gap-2 group">
            <div className="w-8 h-8 bg-slate-900 rounded-lg flex items-center justify-center text-white font-bold">
              {/* Use your actual image logo here if available */}
              <img src="/icon.png" className="w-6 h-6 rounded-md" alt="Logo" onError={(e) => e.currentTarget.style.display = 'none'} /> 
            </div>
            <span className="font-bold text-slate-800 text-lg tracking-tight group-hover:text-emerald-700 transition-colors">
              Intelexta <span className="text-slate-400 font-normal">Verifier</span>
            </span>
          </a>

          <nav className="hidden md:flex items-center gap-4 text-sm font-medium text-slate-500 border-l border-slate-200 pl-6 h-6">
            <a href="https://intelexta.com" target="_blank" rel="noreferrer" className="hover:text-emerald-700 transition-colors">
              About
            </a>
            <a href="https://validator.intelexta.com" target="_blank" rel="noreferrer" className="hover:text-emerald-700 flex items-center gap-1 transition-colors">
              Go to Validator <ExternalLink size={12} />
            </a>
          </nav>
        </div>

        {/* Right Actions */}
        <div>
          {hasResult && (
            <button 
              onClick={onReset}
              className="text-xs font-medium bg-white border border-slate-200 hover:bg-slate-50 text-slate-600 px-3 py-1.5 rounded-full transition-colors flex items-center gap-2"
            >
              <UploadCloud size={14} /> Verify another file
            </button>
          )}
        </div>
      </header>

      {/* 2. NAVIGATION BAR (Only visible when we have results) */}
      <div className="bg-white border-b border-slate-200 px-4 lg:px-8 py-0 shadow-sm z-40 h-12">
        <div className="flex items-center h-full gap-2">
          {hasResult ? (
            <>
              <NavButton 
                active={viewMode === 'verify'} 
                onClick={() => setViewMode('verify')} 
                icon={<Shield size={14} />} 
                label="Verification Report" 
              />
              <NavButton 
                active={viewMode === 'content'} 
                onClick={() => setViewMode('content')} 
                icon={<FileText size={14} />} 
                label="Content Visualizer" 
              />
            </>
          ) : (
            <span className="text-xs font-medium text-slate-400 flex items-center gap-2">
              <UploadCloud size={14}/> Upload a CAR receipt to begin
            </span>
          )}
        </div>
      </div>

      {/* 3. STATUS BAR */}
      <div className="bg-slate-50 border-b border-slate-200 px-4 lg:px-8 py-2 flex items-center justify-between text-xs min-h-[40px]">
        <div className="flex items-center gap-2">
          {fileName ? (
            <>
              <span className="font-semibold text-slate-700">File:</span>
              <span className="font-mono text-slate-600">{fileName}</span>
              {status === 'success' && (
                <span className="bg-emerald-100 text-emerald-700 px-1.5 py-0.5 rounded text-[10px] font-bold uppercase tracking-wide border border-emerald-200">
                  Verified
                </span>
              )}
              {status === 'error' && (
                <span className="bg-rose-100 text-rose-700 px-1.5 py-0.5 rounded text-[10px] font-bold uppercase tracking-wide border border-rose-200">
                  Invalid
                </span>
              )}
            </>
          ) : (
             <span className="text-slate-400 italic">No file loaded.</span>
          )}
        </div>
        
        <div className="text-slate-400 hidden sm:block">
           Client-side verification (WASM) • Zero-knowledge upload
        </div>
      </div>

      {/* 4. MAIN CONTENT */}
      <main className="flex-1 max-w-7xl mx-auto w-full p-4 lg:p-8">
        {children}
      </main>

      {/* 5. FOOTER */}
      <footer className="border-t bg-white mt-auto">
        <div className="max-w-7xl mx-auto px-4 py-6 text-xs text-slate-400 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <div className="flex flex-col gap-1">
            <strong className="text-slate-600 font-semibold">Intelexta Verifier</strong>
            <span className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 bg-emerald-500 rounded-full"></span>
              Independent Verification • CAR v0.3
            </span>
          </div>
          <div className="text-right">
             <span>Built by <a href="https://www.gaugefreedom.com/" className="hover:text-slate-600 underline">Gauge Freedom</a></span>
             <br/>
             <span className="italic">"Exact where possible, accountable where not."</span>
          </div>
        </div>
      </footer>
    </div>
  );
}

function NavButton({ active, onClick, label, icon }: any) {
  return (
    <button
      onClick={onClick}
      className={`px-4 py-1.5 text-xs font-bold rounded-full border transition-all duration-200 flex items-center gap-2 ${
        active 
          ? "bg-emerald-700 text-white border-emerald-700 shadow-md ring-2 ring-emerald-100" 
          : "bg-white text-emerald-700 hover:bg-emerald-50 border-emerald-200"
      }`}
    >
      {icon} {label}
    </button>
  );
}