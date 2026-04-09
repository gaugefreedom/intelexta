 Plan: Localize apps/web-verifier to pt-BR                                                                                                                                                                        
                                                                                                                                                                                                                   
  Approach: Mirror the intelexta-validator pattern exactly — i18next + react-i18next + LanguageDetector, with a flat namespace structure, en and pt-BR locale files, and useTranslation() hooks in each component. 
                                                                                                                                                                                                                   
  ---             
  Step 1 — Install dependencies                                                                                                                                                                                    
                                                                                                                                                                                                                   
  cd apps/web-verifier && npm install i18next react-i18next i18next-browser-languagedetector
                                                                                                                                                                                                                   
  ---             
  Step 2 — Create src/i18n.ts
                                                                                                                                                                                                                   
  Single namespace common (the app is small, no need to split like validator). Mirrors the validator pattern: LanguageDetector with localStorage key intelexta_locale, pt → pt-BR mapping.
                                                                                                                                                                                                                   
  ---                                                                                                                                                                                                              
  Step 3 — Create locale files                                                                                                                                                                                     
                                                                                                                                                                                                                   
  src/locales/en/common.json and src/locales/pt-BR/common.json covering all hardcoded strings across:
                                                                                                                                                                                                                   
  ┌──────────────────────────┬──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐  
  │        Component         │                                                                                   Key strings                                                                                    │  
  ├──────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤  
  │ Layout.tsx               │ About, Go to Validator, Verify another file, Verification Report, Content Visualizer, Upload a CAR receipt to begin, File, Verified, Invalid, Client-side verification (WASM)…,  │
  │                          │ Intelexta Verifier, Independent Verification…, Built by Gauge Freedom, tagline                                                                                                   │
  ├──────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤  
  │ Verifier.tsx             │ Drop receipt here, Verify a Workflow Receipt, drag-and-drop description, Verifying cryptographic proofs…, Verification Error, Receipt Verified, Verification Failed, The         │
  │                          │ cryptographic signature…, Raw JSON Output                                                                                                                                        │  
  ├──────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ WorkflowViewer.tsx       │ Verification Timeline, Step-by-step cryptographic validation…, Steps, No workflow steps found…, Content, Attachments, No content data., No attachments.                          │  
  ├──────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤  
  │ MetadataCard.tsx         │ Verification Summary, Drop a CAR archive to inspect…, Verified, Verification failed, Run ID, CAR ID, Signer, Workflow Model, Timestamp, Metrics, Checkpoints, Provenance,        │
  │                          │ Attachments, Integrity Checks, Hash chain integrity, Signature validation, Content integrity, Passed, Failed                                                                     │  
  ├──────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ WorkflowOverviewCard.tsx │ Workflow, Overview, description, Workflow Name, Created, Run & Proof Mode, Model & Steps, step/steps, checkpoint/checkpoints, Budgets, Tokens, USD, Nature Cost, Stewardship     │  
  │                          │ Score                                                                                                                                                                            │  
  ├──────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ WorkflowStepsCard.tsx    │ Steps, Workflow Steps, step/steps configured, Step N –, Model, Proof Mode, Epsilon, Token Budget, Prompt, Configuration, Collapse, Expand, Prompt truncated…, No steps found…    │  
  ├──────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤  
  │ AttachmentsCard.tsx      │ Attachments, Content Files, file/files extracted, with preview, Content Preview, Collapse/View Full, Preview truncated…, Binary attachment…, Content Hash, Provenance note       │
  │                          │ strings                                                                                                                                                                          │  
  ├──────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
  │ ContentView.tsx          │ Content Visualizer, Upload a CAR file to visualize…                                                                                                                              │  
  ├──────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤  
  │                          │ Public View • No Login Required, Verifying cryptographic chain…, Receipt Not Found, Error Loading Receipt, …, Receipt Verified/Failed, Receipt Metadata, Receipt ID, Created,    │
  │ PublicReceiptPage.tsx    │ Engine / Tier, Stewardship Score, Integrity Analysis, Factual Reliability, Heuristic Novelty, AI Usage Estimate, Document Type, Purpose, Key Claims Analysis, Areas Needing      │  
  │                          │ Attention, Well Supported, Partially Supported, Weakly Supported, Unclear, Verify your own work, About the Protocol                                                              │
  ├──────────────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤  
  │ App.tsx                  │ Page titles and meta descriptions                                                                                                                                                │
  └──────────────────────────┴──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘  
   
  pt-BR translations will follow the same register as the validator's pt-BR strings: professional Brazilian Portuguese, consistent terminology (e.g., "Recibo" for receipt, "Verificado" for verified, etc.).      
                  
  ---                                                                                                                                                                                                              
  Step 4 — Update src/main.tsx
                              
  Import ./i18n before App renders (same as validator). Also remove document.documentElement.lang = 'en' hardcode — let i18n control it, or at least set it dynamically.
                                                                                                                                                                                                                   
  ---
  Step 5 — Add language toggle to Layout.tsx                                                                                                                                                                       
                                                                                                                                                                                                                   
  A small PT | EN toggle button in the header (right side, near "Verify another file"), using i18n.changeLanguage() and persisting to localStorage with the intelexta_locale key.
                                                                                                                                                                                                                   
  ---             
  Step 6 — Wire useTranslation() into each component                                                                                                                                                               
                                                    
  Replace all hardcoded strings with t('key') calls. For plurals (step/steps, file/files), use i18next's _one/_other or _zero/_one/_other count variants.
                                                                                                                                                                                                                   
  ---
  Out of scope for this task                                                                                                                                                                                       
                            
  - Localizing utils/proofFiles.ts error messages (those feed setError directly; will leave for a follow-up)
  - Localizing WASM-originated error strings (those come from Rust)                                                                                                                                                
                                                                                                                                                                                                                   
  ---                                                                                                                                                                                                              
  Ready to implement? This will touch ~10 files and add ~2 new locale files.           
