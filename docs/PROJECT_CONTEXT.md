**Project Master Context: Intelexta (v4)**

Use this document as the foundational context for all discussions, code generation, and strategic refinements related to the Intelexta project.

1. Core Identity & Mottos
    • Project Name: Intelexta
    • One-Liner: A local-first control plane for verifiable Human+AI workflows.
    • Mottos:
        ◦ "Proof, not vibes."
        ◦ "Exact where possible, accountable where not."

2. Mission Alignment (The "Why")
Intelexta is an embodiment of the Gauge Freedom mission, designed to build a scaffold of trust for cognitive work in the age of AI.
    • Human+AI Symbiosis (Agency over Autonomy): Intelexta is a control plane, not an autonomous agent. The human is the strategic director, defining the rules of engagement. This is achieved through local-first keys, project-wide Policies & Budgets, and checkpoint-level configuration snapshots that prove how each step complied. The AI is a powerful cognitive partner, operating within a provable envelope of human-defined policy.
    • Consciousness & Culture Preservation (Integrity of Knowledge): Intelexta is a provenance engine. By creating a permanent, tamper-evident record of how information is generated (Signed Checkpoints), it preserves the context and integrity of knowledge, solving the modern crisis of attribution. The Content-Addressable Receipt (CAR) acts as a digital provenance card for intellectual work.
    • Energy & Climate Impact (Accountable Efficiency): Intelexta makes energy consumption a first-class design parameter. Carbon emissions (gCO₂e) are governed through project-wide budgets that every checkpoint must reconcile against. The system provides transparent reporting, turning an abstract externality into a concrete, manageable metric.

3. Default Guardrails & Posture
Intelexta is designed with a "secure by default" posture to protect the user and their work.
    • Default Posture: Network egress is disabled by default. Local models are preferred. Cryptographic keys are stored in the OS keychain, not the main database. A content-addressed cache is enabled to prevent redundant computation.
    • Policy & Budget Governance: Policies set allowances at the project scope, while each checkpoint persists a signed configuration snapshot, policy revision pointer, and budget reconciliation so enforcement is provable across edits, reruns, and exports.
    • Audit Trail: Every policy change, budget denial, or rule violation becomes a signed, verifiable "incident" checkpoint in the project's history, preserving the context needed to justify re-runs or overrides.
    • Privacy & Portability: No data leaves the user's machine unless explicitly exported by the user. When it is exported (.ixp or .car.json), the package carries the policy lineage and checkpoint configurations required for third-party verification without leaking excess project data.

4. User Experience Philosophy: Control Plane for Checkpointed Runs
Intelexta deliberately trades the raw velocity of a conventional chat UI for the rigor of a verifiable control plane.
    • Runs as Editable Checkpoint Sequences: Every run is defined as an ordered list of checkpoint configurations. Until the run is sealed, users can revise, reorder, or replace checkpoints, and every edit emits a new canonical state that is ready to be signed.
    • EditorPanel as Workflow Builder: The EditorPanel is a workflow builder, not a prompt box. Its RunSpec scaffolding orchestrates checkpoint configuration, policy alignment, budget checks, and signing prep so authored sequences can execute or re-run with proof.
    • Verifiability Over Velocity: Tools like Cursor are optimized for developer velocity. Intelexta is optimized for process verifiability. Every UI element exists to surface checkpoint metadata, budgets, and signatures so that edits remain accountable.
    • Chaining-Ready Foundation: The orchestrator.rs module and the dag_json field in the RunSpec prepare the system for chained execution. V1 focuses on trustworthy single- and multi-step checkpoint sequencing while the engine for richer DAG flows is hardened.
    • Interactive Chat Post-V1: Conversational checkpoints are explicitly deferred until after V1. They will return as a specialized post-V1 workflow mode so interactive collaboration inherits the same audited timeline once the core proof engine is stable.

5. Artifact & Governance Taxonomy
    • Checkpoint: The atomic unit of proof and configuration. Each checkpoint packages its configuration JSON, resolved policy revision, inputs/outputs, and budget reconciliation into a signed bundle so enforcement decisions can be reproduced across reruns and exports.
    • Run: A versioned sequence of checkpoints bound to a RunSpec and proof mode. Drafts capture in-progress edits; sealing freezes the canonical checkpoint order, policy bindings, and governance context for execution, rerun, CAR generation, and export.
    • Project Policies & Budgets: Policies define allowable models, egress, and carbon budgets. Budgets reconcile at every checkpoint and surface denials as incident checkpoints, ensuring sealed runs and reruns provably respect project rules.
    • CAR (Content-Addressable Receipt): A portable, self-contained JSON receipt that carries sealed checkpoint configurations, policy lineage, and budget outcomes. CARs are being prepared for external verification flows via intelexta-verify.
    • IXP (Intelexta Project Export): A signed archive (.zip) containing project.json, full policy history, checkpoint configurations, inspector layouts, and referenced CARs so exports/imports transfer governance context alongside workflow artifacts.

 6. Product Roadmap & Future Capabilities

(This section documents the path from MVP to the full vision.)

Intelexta is developed through a phased roadmap. The V1.0 release focuses on establishing the core "proof engine" as a robust, usable tool. Post-V1 releases will expand its power and ecosystem connectivity.

    • V1.0 (Core Engine): Sprints 2B-3A

        • Goal: Ship a polished, local-first control plane for verifiable AI workflows.

        • Key Features: Exact and Concordant proof modes, editable checkpoint sequencing with checkpoint CRUD and rerun support, chaining-aware RunSpecs with inspector introspection, project policy governance across sealing and reruns, portable signed CAR generation, and project export/import (.ixp) with verification hooks.

        • Deferred Scope: Interactive chat and conversational checkpoint types are explicitly parked for post-V1 so the core workflow builder, governance, and portability milestones can reach production quality first.

    • V1.X (Post-MVP): Sprints 4+

        • Goal: Evolve from a control plane into an intelligent workflow builder and a trusted hub for verifiable knowledge.

        • Intelligent Orchestration (Advanced DAG Engine): Build on the V1 chaining foundation to support branching workflows, comparisons, and reusable workflow templates.

        • Interactive Co-Agency: Reintroduce conversational checkpoints as collaborative notebooks and multi-party review flows once the checkpoint governance layer is proven in production.

        • Optional Blockchain Anchoring: A feature to take a finalized CAR's ID and publish it to a public blockchain. This provides a decentralized, immutable, and universally verifiable timestamp, proving the CAR's existence at a specific point in time. This is critical for academic, legal, and IP-sensitive use cases.

        • Collaboration & Integration: Features for team-based projects, Git integration for CARs, and a robust command-line interface (CLI) for headless execution and scripting.
    
    • The Verification Ecosystem: The intelexta-verify CLI

        • To complete the trust chain and enable third-party validation, the Intelexta project includes a crucial companion: intelexta-verify. This lightweight, open-source CLI verifies exported CARs without requiring the desktop application.

        • Core Functions:

            1. Integrity Check: Verifies the CAR's internal signatures and hash-chain to prove it is tamper-evident.

            2. Reproducibility Check: Reads the RunSpec from the CAR and re-executes the workflow to prove the results are reproducible.

            3. Governance Check: Confirms policy linkage, budget reconciliation, and rerun references to certify that exported runs honored project rules.

            4. Timestamp Check (Optional): For anchored CARs, query public blockchains to confirm immutable timestamps.

7. Core Data Structures & Technical Concepts
    • Rust Structs (Locked):
      // Policy & Budgets
      // Run Configuration
    • Proof Modes:
        ◦ Exact: For deterministic tasks. Acceptance requires a bytewise digest match.
        ◦ Concordant: For stochastic tasks. Acceptance requires passing declared invariants (e.g., semantic distance ≤ ε).
        ◦ Interactive (Post-V1): For collaboration. Acceptance is a "process proof"—verifying the sequence of checkpoints once conversational checkpoints are reintroduced.
    • Canonicalization: All hashed material must be processed by a stable-JSON helper (sorted keys, no insignificant whitespace) before hashing to ensure deterministic digests.
    • CAR v0.2 Schema: The CAR id is the sha256 of its canonicalized body (excluding signatures). See previous discussions for the full field list.
8. Key Metrics (For Mission Reporting)
    • Replay Fidelity: Pass-rate of replay attempts for each proof mode.
    • CAR Coverage: Percentage of runs that successfully emit a CAR.
    • Energy Transparency: Percentage of runs with a non-null gCO₂e estimate.
    • Re-compute Avoided: Cumulative gCO₂e saved via cache hits.
    • Policy Adherence: Number of policy violations successfully blocked and recorded as incidents.


**Intelexta - Sprint 2B Milestone Plan (Workflow Foundation)**
Based on: PROJECT_CONTEXT.md v5
Date: September 23, 2025 (Assumes start after S2A completion)

1. Sprint Goal
"Deliver the schema, orchestrator plumbing, and UI needed to author checkpoint sequences, edit them safely, and seal workflows with provable governance context."
This sprint turns the EditorPanel into a full workflow builder with checkpoint CRUD support while establishing policy-aware persistence and deterministic reruns.
2. Actionable Steps & Tasks
Phase 1: Schema & Persistence Upgrades
    1. Task: Add run revisioning columns (e.g., draft_hash TEXT, sealed_at TIMESTAMP NULL, rerun_of TEXT) so drafts, sealed runs, and reruns are first-class records.
    2. Task: Extend checkpoints with config_json, policy_snapshot_id, budget_snapshot_json, and position INTEGER so edits are captured precisely and auditable.
    3. Task: Introduce checkpoint_edit_log to capture create/update/delete operations with actor, timestamp, and diff payloads for accountability.
Phase 2: Orchestrator & Command Surface
    1. Task: Expose Tauri commands for checkpoint create, update, reorder, and delete that validate against current project policy and budgets before persisting.
    2. Task: Implement seal_run(run_id) to freeze the draft, record the governing policy snapshot, and produce the initial signed checkpoint sequence digest.
    3. Task: Implement rerun_run(run_id, reason) that clones the sealed configuration, emits a rerun record linked to the original run, and prepares the orchestrator to execute it deterministically.
Phase 3: EditorPanel Workflow Builder UX
    1. Task: Replace the single-submit form with a checkpoint list editor supporting add, duplicate, edit, reorder, and delete interactions with inline validation feedback.
    2. Task: Display live budget projections per checkpoint and flag edits that exceed policy allowances before they can be saved.
    3. Task: Add "Seal" and "Re-Run" affordances that call the new commands, surface policy references, and update the run timeline without requiring a full refresh.
3. Acceptance Criteria (Definition of "Done")
    • CRUD-01: Draft runs support checkpoint insert, update, reorder, and delete operations until sealed, with every action recorded in checkpoint_edit_log.
    • CRUD-02: Sealing a run freezes the checkpoint sequence, records the governing policy snapshot, and emits a signed initial checkpoint ready for execution.
    • RERUN-01: Triggering rerun_run on a sealed run produces a linked rerun record, reuses the checkpoint configurations, and prepares the orchestrator to execute without manual re-entry.
    • GOV-READY-01: Checkpoint edits that violate active project policies or budgets are blocked with actionable errors and recorded in the checkpoint_edit_log for audit.

**Intelexta - Sprint 2C Milestone Plan (Chaining, Inspector Introspection & Governance)**
Based on: PROJECT_CONTEXT.md v5
Date: September 30, 2025 (Assumes start after S2B completion)

1. Sprint Goal
"Activate checkpoint chaining, deepen inspector introspection, and extend governance coverage so multi-step workflows remain explainable and policy compliant."
This sprint connects the workflow builder to a chaining-capable orchestrator, layers governance signals onto each node, and equips inspectors with rich detail views.
2. Actionable Steps & Tasks
Phase 1: Orchestrator Chaining Enhancements
    1. Task: Expand dag_json to support ordered node identifiers, dependency lists, and checkpoint templates for each node.
    2. Task: Update orchestrator.rs to execute nodes in dependency order, emitting checkpoints (or incident checkpoints) per node and honoring resumable execution points.
    3. Task: Persist chain metadata on the run so replay and exports can reproduce the authored structure.
Phase 2: Governance Signals for Chained Runs
    1. Task: Extend checkpoint schema entries for chained runs with node-level policy snapshot IDs, budget deltas, and dependency approvals.
    2. Task: Update orchestrator.rs to evaluate policy and budget checks before executing each node, emitting incident checkpoints when denials occur and halting downstream execution.
    3. Task: Record governance summaries per node (approvals, denials, overrides) so inspectors and CARs surface the compliance state alongside outputs.
Phase 3: Inspector Introspection
    1. Task: Build an Inspector detail panel that displays the selected checkpoint's configuration JSON, policy snapshot metadata, budget usage, and dependency context.
    2. Task: Add per-checkpoint replay status, rerun lineage, and governance annotations so reviewers see when and how steps were re-verified or blocked.
    3. Task: Support chained navigation (previous/next, dependency graph) from the Inspector to help reviewers follow multi-step flows and understand governance outcomes.
3. Acceptance Criteria (Definition of "Done")
    • CHAIN-01: Runs may define chained checkpoints with explicit dependencies, and the orchestrator executes them in order, producing checkpoints (or incidents) per node.
    • GOV-CHAIN-01: Governance checks run per node, blocking execution on policy or budget violations and logging incident checkpoints with dependency context.
    • INSP-01: The Inspector detail view surfaces configuration, policy, budget, dependency, and replay context for any checkpoint in a chain.

**Intelexta - Sprint 3A Milestone Plan (Portability & Verification)**
Based on: PROJECT_CONTEXT.md v5
Date: October 7, 2025 (Assumes start after S2C completion)

1. Sprint Goal
"Make checkpointed workflows portable across machines and deliver independent CAR verification for exported artifacts."
This sprint packages checkpoint configuration bundles with policy lineage for export/import and equips auditors with robust verification tools.
2. Actionable Steps & Tasks
Phase 1: Policy Lineage Packaging
    1. Task: Implement policy versioning with effective_from timestamps and ensure every checkpoint references the correct policy revision and budget snapshot ID.
    2. Task: Persist run-level governance summaries (policy revision, budget totals, incidents) so sealed runs capture the context required for export and verification.
    3. Task: Surface governance incidents in the UI with remediation actions and ensure they propagate into CARs and exports.
Phase 2: Project Portability
    1. Task: Expand export_project to package project.json, policy history, runs, checkpoint configuration bundles, chaining metadata, inspector layouts, and CARs into the .ixp archive with manifest signatures.
    2. Task: Implement import_project with schema validation, policy/budget reconciliation, and optional dry-run replay to confirm integrity before committing.
    3. Task: Provide UI previews that highlight inbound policy differences, checkpoint conflicts, and CAR verification status prior to import.
Phase 3: CAR Verification Tooling
    1. Task: Ship intelexta-verify CLI v1 that validates CAR signatures, policy linkage, checkpoint budgets, and rerun references offline.
    2. Task: Add a "Verify Externally" affordance in the Inspector that exports the selected run's CAR and invokes the CLI with captured output.
    3. Task: Publish documentation describing the verification flow, canonical_json expectations, and how policy governance is represented in CARs.
3. Acceptance Criteria (Definition of "Done")
    • POLICY-LINEAGE-01: Sealed runs capture the policy revision, budget totals, and incidents required for export and CAR verification, and inspectors surface that context.
    • PORT-01: Exporting a project produces an .ixp that, when inspected, contains runs, checkpoint configuration bundles, policies, inspector metadata, and CARs with consistent digests and manifests.
    • VERIFY-01: The intelexta-verify CLI validates a CAR's checkpoint chain, policy linkage, budget compliance, and rerun references, returning actionable diagnostics on mismatch.
