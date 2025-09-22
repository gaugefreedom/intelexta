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
    • Runs as Editable Checkpoint Sequences: Every run is curated as an editable sequence of checkpoints. Until the run is sealed, users can reorder, refine, or replace checkpoints, and every adjustment produces a new canonicalized state that is ready to be signed.
    • EditorPanel as Workflow Builder: The EditorPanel functions as a structured workflow builder. Its explicit RunSpec fields orchestrate checkpoint configuration, budget alignment, and signing prep so the authored sequence can be executed or re-run with proof.
    • Verifiability Over Velocity: Tools like Cursor are optimized for developer velocity. Intelexta is optimized for process verifiability. Every UI element exists to surface checkpoint metadata, budgets, and signatures so that edits remain accountable.
    • Foundations for Chaining: The current orchestrator.rs module and the dag_json field in the RunSpec seed the advanced workflow builder capability. V1 emphasizes perfecting single-step execution and editable sequencing while preparing for multi-node chains.
    • Sprint 2C Interactive Checkpoint: Interactive chat emerges as a specialized checkpoint type delivered in Sprint 2C. Each exchange (human prompt, AI response) is captured as a signed checkpoint, letting conversational collaboration live within the same audited workflow timeline.

5. Artifact Taxonomy
    • Checkpoint: The atomic unit of proof and configuration. Each checkpoint stores its configuration JSON, resolved policy snapshot, inputs/outputs, and budget reconciliation, all signed and hash-chained so enforcement decisions are reproducible.
    • Run: An editable, versioned sequence of checkpoints defined by a specific RunSpec and proof mode. Drafts capture in-progress edits; sealing freezes the canonical sequence for execution, re-run, CAR generation, and export.
    • CAR (Content-Addressable Receipt): A portable, self-contained JSON file that carries the sealed checkpoint configurations, policy lineage, and budget outcomes for a run, enabling third parties to verify compliance without accessing the full project.
    • IXP (Intelexta Project Export): A compressed archive (.zip) containing project.json, full policy and budget history, run and checkpoint data (including interactive checkpoints), and referenced CARs so governance context and portability travel together.

 6. Product Roadmap & Future Capabilities

(This new section formally documents the path from MVP to the full vision.)

Intelexta is developed through a phased roadmap. The V1.0 release focuses on establishing the core "proof engine" as a robust, usable tool. Post-V1 releases will expand its power and ecosystem connectivity.

    • V1.0 (Core Engine): Sprints 1-3

        • Goal: Ship a polished, local-first control plane for verifiable AI workflows.

        • Key Features: Exact and Concordant proof modes, editable checkpoint sequencing with checkpoint CRUD and rerun support, interactive checkpoint type for guided chat, portable signed CAR generation, project export/import (.ixp), and integration with local and online AI models under strict governance.

    • V1.X (Post-MVP): Sprints 4+

        • Goal: Evolve from a control plane into an intelligent workflow builder and a trusted hub for verifiable knowledge.

        • Intelligent Orchestration (DAG Engine): The orchestrator.rs will be enhanced to support multi-step, branching workflows defined in dag_json. This will enable users to chain AI calls, run comparisons, and build complex, reproducible research pipelines.

        • Interactive Co-Agency: Post-V1 work grows the interactive checkpoint into collaborative notebooks and multi-party review flows. Each turn (human prompt, AI response) remains a checkpoint so the transcript itself stays auditable while unlocking richer co-agency patterns.

        • Optional Blockchain Anchoring: A feature to take a finalized CAR's ID and publish it to a public blockchain. This provides a decentralized, immutable, and universally verifiable timestamp, proving the CAR's existence at a specific point in time. This is critical for academic, legal, and IP-sensitive use cases.

        • Collaboration & Integration: Features for team-based projects, Git integration for CARs, and a robust command-line interface (CLI) for headless execution and scripting.
    
    • The Verification Ecosystem: The intelexta-verify CLI

        • To complete the trust chain and enable third-party validation, the Intelexta project includes a crucial companion: intelexta-verify. This is a lightweight, open-source, standalone command-line tool.

        • Purpose: It allows anyone to verify a .car.json file without needing to install the full Intelexta desktop application. This is essential for academic reviewers, legal teams, and auditors.

        • Core Functions:

            1. Integrity Check: Verifies the CAR's internal signatures and hash-chain to prove it is tamper-evident.

            2. Reproducibility Check: Reads the RunSpec from the CAR and re-executes the entire workflow to prove the results are reproducible.

            3. Timestamp Check (Optional): For anchored CARs, it queries the public blockchain to confirm the immutable timestamp.

7. Core Data Structures & Technical Concepts
    • Rust Structs (Locked):
      // Policy & Budgets
      // Run Configuration
    • Proof Modes:
        ◦ Exact: For deterministic tasks. Acceptance requires a bytewise digest match.
        ◦ Concordant: For stochastic tasks. Acceptance requires passing declared invariants (e.g., semantic distance ≤ ε).
        ◦ Interactive: For collaboration. Acceptance is a "process proof"—verifying the sequence of checkpoints.
    • Canonicalization: All hashed material must be processed by a stable-JSON helper (sorted keys, no insignificant whitespace) before hashing to ensure deterministic digests.
    • CAR v0.2 Schema: The CAR id is the sha256 of its canonicalized body (excluding signatures). See previous discussions for the full field list.
8. Key Metrics (For Mission Reporting)
    • Replay Fidelity: Pass-rate of replay attempts for each proof mode.
    • CAR Coverage: Percentage of runs that successfully emit a CAR.
    • Energy Transparency: Percentage of runs with a non-null gCO₂e estimate.
    • Re-compute Avoided: Cumulative gCO₂e saved via cache hits.
    • Policy Adherence: Number of policy violations successfully blocked and recorded as incidents.


**Sprint 0 - Architectural Hardening**

    • Duration: 1 Week (Focused effort)
    • Goal: Implement the 7 blockers identified in the review to establish a robust, auditable foundation before any major feature work begins. This sprint is about building the bedrock.

Actionable Steps & Tasks for Sprint 0:

    1. Lock Dependencies:
        ◦ In Cargo.toml, add serde_jcs, and ensure versions for sha2, keyring, and ed25519-dalek are pinned.
    2. Update Database Schema:
        ◦ In schema.sql, apply the "SQL deltas" from the review. Add the migrations table, modify runs, checkpoints, and receipts, and create the new indexes.
    3. Implement Normative Rules in Code:
        ◦ provenance.rs: Implement the canonical_json and sha256_hex helpers using the specified crates. Implement the key lifecycle logic (storage in keychain, stubs for export/rotation).
        ◦ governance.rs: Implement the core logic for budget and egress enforcement that creates a signed Incident checkpoint on violation.
        ◦ orchestrator.rs: Ensure the orchestrator calls the governance rules before execution.
    4. Pin Core Types & Schemas:
        ◦ In a shared types.rs or lib.rs, define the CheckpointKind and Incident Rust structs.
        ◦ Create a new file schemas/car-v0.2.schema.json and paste the provided JSON-Schema draft into it.
    5. Create Placeholder Verify Tool:
        ◦ Create a new binary crate in your workspace: src-tauri/crates/intelexta-.
        ◦ Implement a basic CLI (using clap) that accepts a file path. For now, it can just parse the file as JSON and print "Verified (stub)" if successful. The full logic can be built out later.

Acceptance Criteria for Sprint 0 (Definition of "Done"):
(These are adapted directly from the review's test matrix)
    • [ ] DB-01: The new database migrations are idempotent, and all NOT NULL constraints are enforced.
    • [ ] CANON-01: A test proves that serializing the same struct with canonical_json produces identical bytes.
    • [ ] KEY-01: create_project successfully stores the secret key in the OS keychain. A test can retrieve it.
    • [ ] GOV-01: A test run with a budget of 5 vs. a usage of 10 correctly emits a signed Incident(kind="budget_ into the checkpoints table.
    • [ ] RUN-01: The "Hello-Run" stub still writes a valid, signed checkpoint that conforms to the new, more complex schema.



**Intelexta - Sprint 1A Implementation Plan (v2)**
Based on: PROJECT_CONTEXT.md v3
Date: September 2, 2025

1. Sprint Goal
"Implement the Policy & Budgets vertical slice and a 'Hello-Run' to prove the core orchestrator and checkpoint plumbing."
This sprint de-risks the project by building a complete, thin slice of functionality. It ensures the database, security model, and core workflow engine are functional before we build more complex features like Replay and CAR generation on top.
2. Actionable Steps & Tasks
Phase 1: Foundational Backend Setup
    1. Task: Evolve Database Schema
        ◦ File: src-tauri/src/store/schema.sql
        ◦ Action: Add a migrations table (version INTEGER PRIMARY KEY) to make future schema changes idempotent.
        ◦ Action: Add the new proof-mode fields to the full schema:
            ▪ In runs: kind TEXT NOT NULL DEFAULT 'exact', sampler_json TEXT
            ▪ In checkpoints: semantic_digest TEXT
            ▪ In receipts: match_kind TEXT, epsilon REAL
        1.1 Task: integrate a dedicated Rust migration crate like rusqlite_migrations or refinery. This automates schema changes, ensures they are applied consistently and in the correct order, and makes the database setup far more robust and maintainable
    2. Task: Implement Secure Key Storage
        ◦ File: src-tauri/Cargo.toml
        ◦ Action: Add the keyring crate.
        ◦ File: src-tauri/src/api.rs
        ◦ Action: In create_project, store the generated secret key in the OS keychain using service="intelexta", username=<project_id>. The public key remains in the DB.
        ◦ Action: Create a get_secret_key(project_id: &str) helper that retrieves the key when needed for signing.
    3. Task: Create Canonicalization Helpers
        ◦ File: src-tauri/src/provenance.rs (or a new utils.rs module)
        ◦ Action: Implement canonical_json(value: &impl Serialize) -> Vec<u8>. Use a crate like serde_json_canon.
        ◦ Action: Implement sha256_hex(bytes: &[u8]) -> String.
Phase 2: Feature Implementation
    1. Task: Implement Policy & Budgets (Backend)
        ◦ File: src-tauri/src/api.rs
        ◦ Action: Implement get_policy (returns default if absent) and update_policy (UPSERTs the policy).
    2. Task: Implement "Hello-Run" Orchestrator (Backend)
        ◦ File: src-tauri/src/orchestrator.rs
        ◦ Action: Implement a basic start_run function that takes a RunSpec.
        ◦ Action: Inside, create a deterministic stub operation. Example: output_bytes = sha256(input_bytes || seed_as_le_bytes). This proves the plumbing without a real AI model.
        ◦ Action: After the stub op, create and sign a single valid checkpoint. Use the provenance.rs helpers and the secret key from the keychain. Store it in the checkpoints table.
    3. Task: Implement Budget Enforcement (Backend)
        ◦ File: src-tauri/src/governance.rs
        ◦ Action: Implement a simple enforce_budget function.
        ◦ File: src-tauri/src/orchestrator.rs
        ◦ Action: Before creating the checkpoint in your "Hello-Run," call enforce_budget with a tiny, hardcoded usage value (e.g., 10 tokens).
        ◦ Action: If the budget is exceeded, the orchestrator must create a signed error checkpoint (type: "incident", reason: "Budget exceeded") instead of a normal one.
    4. Task: Implement UI for Policy & Inspection
        ◦ File: app/src/components/
        ◦ Action: Build and wire the UI form to get_policy and update_policy.
        ◦ File: app/src/components/
        ◦ Action: Create a basic table to display checkpoints for a selected run. Columns: timestamp, node_id, inputs_sha256, outputs_sha256, usage_tokens.
        ◦ Action: Add an "Emit CAR" button (can be disabled for now, or call a placeholder emit_car command).
3. Acceptance Criteria (Definition of "Done")
    • DB-01: Migrations are idempotent; the migrations.version table exists and increments correctly on schema changes.
    • KEY-01: create_project stores the secret key in the OS keychain; retrieving it by project_id for signing works.
    • POL-01: get_policy returns a default policy if none exists; update_policy successfully UPSERTs data that persists after an app restart.
    • RUN-01: Starting a "Hello-Run" successfully writes a single, valid checkpoint to the database with a correct hash-chain and a verifiable Ed25519 signature.
    • GOV-01: Setting a token budget of 5 and running the "Hello-Run" (which uses 10 tokens) results in a signed error checkpoint, and the UI correctly surfaces a "Budget exceeded" error message.
    • CAR-01: The "Emit CAR" button calls a placeholder backend command that successfully writes a minimal car.json file with the required v0.2 fields (even if values are placeholders) and records the file path in the receipts table.


**Intelexta - Sprint 1B Implementation Plan**
Based on: PROJECT_CONTEXT.md v3
Date: September 9, 2025 (Assumes start after S1A completion)

1. Sprint Goal
"Bring the 'Proof, not vibes' motto to life by implementing the core Replay and CAR generation features for 'Exact' proof mode."
This sprint makes the project's core promise tangible. By the end, a user will not only be able to run a verifiable workflow but will also be able to generate a portable, cryptographic receipt for it and replay the run to prove its integrity.
2. Actionable Steps & Tasks
This plan focuses on activating the replay and car modules, and connecting them to the UI.
Phase 1: Implement 'Exact' Replay (Backend)
    1. Task: Define the Replay Report Structure
        ◦ File: src-tauri/src/replay.rs (or a shared types file)
        ◦ Action: Define a ReplayReport struct. It should be serializable and include fields like run_id: String, match_status: bool, original_digest: String, replay_digest: String, and an optional error_message: String.
    2. Task: Implement the Replay Logic
        ◦ File: src-tauri/src/replay.rs
        ◦ Action: Implement the core replay_exact_run(run_id: String) function.
        ◦ Logic:
            1. Query the database to fetch the original RunSpec and all associated checkpoints for the given run_id.
            2. Re-execute the deterministic stub operation from the orchestrator using the fetched RunSpec (same inputs, same seed).
            3. Generate a new output digest from this re-execution.
            4. Compare the new digest against the outputs_sha256 from the original final checkpoint.
            5. Populate and return the ReplayReport struct with the results of the comparison.
    3. Task: Expose Replay as an API Command
        ◦ File: src-tauri/src/api.rs
        ◦ Action: Create a Tauri command replay_run(run_id: String) -> Result<ReplayReport, ApiError> that calls the replay_exact_run function.
        ◦ File: src-tauri/src/main.rs
        ◦ Action: Ensure replay_run is added to the invoke_handler.
Phase 2: Implement CAR Generation (Backend)
    1. Task: Fully Implement the CAR Builder
        ◦ File: src-tauri/src/car.rs
        ◦ Action: Implement the build_car(run_id: String) function.
        ◦ Logic:
            1. Query the database for all necessary information for the given run_id: project details (pubkey), run spec, all checkpoints, and the associated policy.
            2. Assemble the complete CAR v0.2 struct using the data, populating all fields (runtime, provenance, proof, etc.) with real values.
            3. Implement the calculate_s_grade function with a simple weighted average based on the run's data.
            4. Use the canonical_json helper to serialize the CAR body (without the signatures block).
            5. Hash the canonical JSON to generate the final car.id.
            6. Use the project's secret key (retrieved from the keychain) to sign the car.id.
            7. Return the fully assembled and signed CAR struct.
    2. Task: Implement the 'Emit CAR' Command
        ◦ File: src-tauri/src/api.rs
        ◦ Action: Fully implement the emit_car(run_id: String) -> Result<String, ApiError> command.
        ◦ Logic:
            1. Call car::build_car() to get the complete CAR object.
            2. Serialize the CAR object to a JSON string.
            3. Determine a file path (e.g., in the app's data directory under <project_id>/receipts/<car_id>).
            4. Write the JSON string to the file.
            5. Insert a new record into the receipts table in the database, storing the CAR's metadata and the file path.
            6. Return the file path of the saved CAR to the frontend.
Phase 3: Wire Up the Frontend UI
    1. Task: Update Frontend API Layer
        ◦ File: app/src/lib/api.ts
        ◦ Action: Define the TypeScript interface for ReplayReport.
        ◦ Action: Add the wrapper functions replayRun(runId: string): Promise<ReplayReport> and emitCar(runId: string): Promise<string>.
    2. Task: Activate Inspector Panel Buttons
        ◦ File: app/src/components/
        ◦ Action: Add a "Replay Run" button next to the "Emit CAR" button.
        ◦ Action: Wire the onClick handler for "Replay Run" to call the replayRun API function. Use useState to manage a loading state and display the ReplayReport result to the user (e.g., "Success: Digests Match!").
        ◦ Action: Wire the onClick handler for "Emit CAR" to call the emitCar API function. On success, display a toast/message to the user with the path of the saved receipt file.
3. Acceptance Criteria (Definition of "Done")
    • REPLAY-01: Clicking "Replay Run" on a successfully completed "Hello-Run" executes the backend logic and the UI correctly displays a success status, indicating the outputs_sha256 digests match.
    • REPLAY-02 (Negative Test): After manually altering the outputs_sha256 of a saved checkpoint in the database, replaying that run results in the UI displaying a failure status, indicating a digest mismatch.
    • CAR-02: Clicking "Emit CAR" on a completed "Hello-Run" successfully generates a car.json file on the filesystem with all required v0.2 fields populated with real data from the database (not placeholders).
    • CAR-03: The signature in the generated CAR file is valid and can be successfully verified against the project's public key.
    • CAR-04: The receipts table in the database is correctly populated with the CAR's ID, S-Grade, and the absolute path to the generated file


**Intelexta - Sprint 2A Implementation Plan**
Based on: PROJECT_CONTEXT.md v3
Date: September 16, 2025 (Assumes start after S1B completion)

1. Sprint Goal
"Transition from a proof-of-concept to a functional tool by integrating a real local AI model and implementing the 'Concordant' proof and replay mode."
By the end of this sprint, a user will be able to execute a run using a local LLM, generate a semantically meaningful result, and verify that result within a given tolerance (epsilon), making the tool useful for real-world stochastic tasks.
2. Actionable Steps & Tasks
This sprint is divided into integrating the AI model, building the Concordant proof backend, and updating the UI to support these new capabilities.
Phase 1: Integrate a Local AI Model
    1. Task: Add Local LLM Crate
        ◦ File: src-tauri/Cargo.toml
        ◦ Action: Add a crate for interacting with local LLMs. A good choice would be a crate that can interface with a running Ollama instance or load a GGUF model file directly (e.g., llm or a similar crate).
    2. Task: Abstract the Execution Logic
        ◦ File: src-tauri/src/orchestrator.rs
        ◦ Action: Refactor the "deterministic stub operation" from Sprint 1A. Create a new function, execute_node, that takes a RunSpec.
        ◦ Logic: This function will now contain a match statement on the RunSpec's model identifier. For now, it can have two branches:
            ▪ "stub-model": Executes the old deterministic stub for testing.
            ▪ _ (default): Calls a new function, execute_llm_run, to handle real AI models.
    3. Task: Implement LLM Execution
        ◦ File: src-tauri/src/orchestrator.rs
        ◦ Action: Implement the execute_llm_run function.
        ◦ Logic:
            ▪ Initialize the chosen LLM crate.
            ▪ Pass the prompt from the RunSpec to the model.
            ▪ Stream the model's output and capture the full response text.
            ▪ Track token usage for the prompt and completion.
            ▪ Return the final output text and the Usage metrics.
Phase 2: Implement 'Concordant' Proof Mode (Backend)
    1. Task: Implement Semantic Hashing
        ◦ File: src-tauri/src/provenance.rs
        ◦ Action: Implement a semantic_digest(text: &str) -> String function.
        ◦ Logic (Simple First Pass): Use a crate like simhash or implement it manually: tokenize text into words, create overlapping 3-grams, hash each n-gram, and combine them into a final SimHash value.
        ◦ Action: When a run is completed, if the mode is Concordant, call this function on the output and store the result in the checkpoints.semantic_digest column.
    2. Task: Update Replay Logic for Concordant Mode
        ◦ File: src-tauri/src/replay.rs
        ◦ Action: Create a new replay_concordant_run(run_id: String) function.
        ◦ Logic:
            1. Fetch the original RunSpec and the original final checkpoint (including its semantic_digest).
            2. Re-execute the run using the orchestrator.
            3. Compute the semantic digest of the new output.
            4. Calculate the semantic distance (e.g., Hamming distance between the two SimHash digests).
            5. Fetch the epsilon value from the receipts table for that run.
            6. Compare: if distance <= epsilon, the replay is a success.
            7. Update the ReplayReport struct to include semantic distance fields.
    3. Task: Update the Main Replay Command
        ◦ File: src-tauri/src/api.rs
        ◦ Action: Modify the replay_run command. It should now query the runs table to find the kind of run. Based on the kind, it will call either replay_exact_run or replay_concordant_run.
Phase 3: Update Frontend for New Features
    1. Task: Enhance the "Run Configuration" UI
        ◦ File: app/src/components/
        ◦ Action: Activate the "Proof Mode" radio buttons.
        ◦ Action: When "Concordant" is selected, display a new input field or slider for the user to set the epsilon value (e.g., a slider from 0.0 to 1.0).
        ◦ Action: Update the Model dropdown to include real, locally available models alongside the "stub-model".
    2. Task: Improve the Project Tree
        ◦ File: app/src/components/
        ◦ Action: Modify the component to be a true tree. When a project is clicked, fetch and display its associated Runs as child nodes.
        ◦ Action: Add small badges next to each run's name to indicate its proof mode ([E], [C], [I]).
    3. Task: Update the Inspector Panel
        ◦ File: app/src/components/
        ◦ Action: When displaying the results of a Concordant replay, show the detailed report: "Concordant Proof: PASS (Distance: 0.08 <= ε: 0.15)".
3. Acceptance Criteria (Definition of "Done")
    • LLM-01: Executing a run with a real local model (e.g., one served by Ollama) successfully generates text output, and the run's token usage is correctly recorded in a checkpoint.
    • CONCORD-01 (Hashing): Completing a run in Concordant mode correctly calculates and stores a semantic_digest in the final checkpoint.
    • CONCORD-02 (Replay): Replaying a Concordant run successfully re-executes the LLM prompt and compares the old and new semantic digests, producing a correct PASS/FAIL status in the ReplayReport.
    • UI-02 (Run Config): The EditorPanel now allows the user to select Concordant mode and set an epsilon value, which is correctly saved in the RunSpec.
    • UI-03 (Project Tree): The project list now functions as a tree, successfully listing past runs under each project, complete with their proof-mode badges.



**Intelexta - Sprint 2B Milestone Plan (Checkpoint CRUD & Reruns)**
Based on: PROJECT_CONTEXT.md v4
Date: September 23, 2025 (Assumes start after S2A completion)

1. Sprint Goal
"Deliver the schema, orchestrator plumbing, and UI needed to author checkpoint sequences, edit them safely, and re-run sealed workflows."
This sprint turns the EditorPanel into a full workflow builder with checkpoint CRUD support while establishing the API surface for deterministic reruns under governance.
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

**Intelexta - Sprint 2C Milestone Plan (Chaining, Interactive Checkpoint, Inspector Detail)**
Based on: PROJECT_CONTEXT.md v4
Date: September 30, 2025 (Assumes start after S2B completion)

1. Sprint Goal
"Activate checkpoint chaining, deliver the interactive checkpoint type, and deepen inspector visibility so multi-step workflows remain explainable."
This sprint connects the workflow builder to a chaining-capable orchestrator, introduces a conversational checkpoint mode, and equips inspectors with rich detail views.
2. Actionable Steps & Tasks
Phase 1: Orchestrator Chaining Enhancements
    1. Task: Expand dag_json to support ordered node identifiers, dependency lists, and checkpoint templates for each node.
    2. Task: Update orchestrator.rs to execute nodes in dependency order, emitting checkpoints (or incident checkpoints) per node and honoring resumable execution points.
    3. Task: Persist chain metadata on the run so replay and exports can reproduce the authored structure.
Phase 2: Interactive Checkpoint Experience
    1. Task: Define a checkpoint_kind "interactive" with schema support for transcript turns, participant attribution, and policy references per exchange.
    2. Task: Implement UI affordances within the EditorPanel to launch an interactive checkpoint session, capture each turn as a sub-checkpoint entry, and finalize it into the sequence.
    3. Task: Ensure governance hooks (budgets, policy overrides) are invoked for each interactive turn before it is committed.
Phase 3: Inspector Detail View
    1. Task: Build an Inspector detail panel that displays the selected checkpoint's configuration JSON, policy snapshot metadata, budget usage, and any interactive transcript content.
    2. Task: Add per-checkpoint replay status and rerun lineage indicators so reviewers can see when and how steps were re-verified.
    3. Task: Support chained navigation (previous/next, dependency graph) from the Inspector to help reviewers follow multi-step flows.
3. Acceptance Criteria (Definition of "Done")
    • CHAIN-01: Runs may define chained checkpoints with explicit dependencies, and the orchestrator executes them in order, producing checkpoints (or incidents) per node.
    • INT-01: Interactive checkpoints capture conversational turns as part of the run, respect governance checks per exchange, and appear alongside other checkpoints in the timeline.
    • INSP-01: The Inspector detail view surfaces configuration, policy, budget, and replay context for any checkpoint, including interactive transcripts.

**Intelexta - Sprint 3A Milestone Plan (Governance, Portability & Verification)**
Based on: PROJECT_CONTEXT.md v4
Date: October 7, 2025 (Assumes start after S2C completion)

1. Sprint Goal
"Harden project-level governance, make checkpointed workflows portable, and deliver independent CAR verification."
This sprint enforces policy governance across the project lifecycle, packages artifacts for export/import, and equips auditors with robust verification tools.
2. Actionable Steps & Tasks
Phase 1: Governance Reinforcement
    1. Task: Implement policy versioning with effective_from timestamps and ensure every checkpoint references the correct policy revision.
    2. Task: Add governance validation that blocks sealing or rerunning if cumulative budgets exceed project allowances or mandatory controls are unmet.
    3. Task: Surface governance incidents in the UI with remediation actions and ensure they propagate into CARs and exports.
Phase 2: Project Portability
    1. Task: Expand export_project to package project.json, policy history, runs, checkpoints (including interactive metadata), inspector layouts, and CARs into the .ixp archive with manifest signatures.
    2. Task: Implement import_project with schema validation, policy/budget reconciliation, and optional dry-run replay to confirm integrity before committing.
    3. Task: Provide UI previews that highlight inbound policy differences, checkpoint conflicts, and CAR verification status prior to import.
Phase 3: CAR Verification Tooling
    1. Task: Ship intelexta-verify CLI v1 that validates CAR signatures, policy linkage, checkpoint budgets, and rerun references offline.
    2. Task: Add a "Verify Externally" affordance in the Inspector that exports the selected run's CAR and invokes the CLI with captured output.
    3. Task: Publish documentation describing the verification flow, canonical_json expectations, and how policy governance is represented in CARs.
3. Acceptance Criteria (Definition of "Done")
    • GOV-01: Governance checks prevent sealing or rerunning when project policies or budgets would be violated, and the resulting incidents are recorded and viewable.
    • PORT-01: Exporting a project produces an .ixp that, when inspected, contains runs, checkpoints, policies, inspector metadata, and CARs with consistent digests and manifests.
    • VERIFY-01: The intelexta-verify CLI validates a CAR's checkpoint chain, policy linkage, budget compliance, and rerun references, returning actionable diagnostics on mismatch.
