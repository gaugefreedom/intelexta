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

Of course. The current document is good, but it still contains remnants of the older, more confusing "draft-then-sealed" model.

I've rewritten the necessary sections to fully align with our new, clearer strategy: the "Workflow as a Reusable Template" and the "Program vs. Execution" model. The changes make the entire document more consistent and powerful.

Here is the revised context, with significant changes highlighted.

Project Master Context: Intelexta (v5 - Revised)

(Sections 1, 2, and 3 are excellent and require no changes.)

4. User Experience Philosophy: The Control Plane Loop

Intelexta is built around a clear, two-part loop: building a Workflow Definition (the program) and generating an Execution Record (the proof). This deliberately trades the raw velocity of a chat UI for the rigor of a verifiable control plane.

    • The Workflow Builder is Your Program: The main editor is for creating and refining a Run, which is a reusable workflow template. A Run is composed of an ordered sequence of configurable Steps (run_steps). This is the mutable "source code" for your work, and it is always editable.
    • The Inspector is Your Execution History: The Inspector displays the immutable Execution Records. Every time you click "Execute Full Run," you create a new, timestamped execution record in the Inspector. This provides a complete, auditable history of every time your workflow program was run.
    • Verifiability Over Velocity: Every UI element is designed to support this loop. The Workflow Builder helps you define a reproducible process, and the Inspector allows you to inspect and verify the historical proof of each execution.
    • Chaining-Ready Foundation: The orchestrator.rs module and the dag_json field in the RunSpec prepare the system for chained execution. V1 focuses on trustworthy single- and multi-step checkpoint sequencing while the engine for richer DAG flows is hardened.
    • Interactive Chat Post-V1: Conversational checkpoints are explicitly deferred until after V1. They will return as a specialized post-V1 workflow mode so interactive collaboration inherits the same audited timeline once the core proof engine is stable.

5. Artifact & Governance Taxonomy

    • Run (The Program 📝): A Run is a reusable workflow definition composed of an ordered list of configurable Steps. It is the primary document the user creates and edits in the Workflow Builder. It does not store execution results, only the plan for execution.
    • Execution Record (The Proof 🧾): An Execution Record is the result of a single, complete execution of a Run. It is an immutable, timestamped log that appears in the Inspector. Each record is composed of one or more executed Checkpoints.
    • Checkpoint: The atomic unit of proof. A Checkpoint is the immutable record of a single Step's execution, containing its cryptographic hashes, usage metrics, and signature. It is the core component of an Execution Record.
    • CAR (Content-Addressable Receipt): A portable, self-contained JSON file that packages a single Execution Record. A CAR allows a third party to inspect the human-readable details of an execution and cryptographically verify its integrity and reproducibility.
    • IXP (Intelexta Project Export): A signed archive (.zip) containing a project's full state, including its workflow definitions (Runs), governance policies, and all historical Execution Records.


 6. Product Roadmap & Future Capabilities

    • V1.0 (Core Engine): Sprints 2B-3A
        • Goal: Ship a polished, local-first control plane for creating, executing, and verifying multi-step AI workflows.
        • Key V1 Features:
            • Workflow Editor: Create and manage Runs as editable sequences of Steps.
            • Core Step Types: Exact and Concordant proof modes with epsilon controls.
            • Execution History: Generate a new, immutable Execution Record in the Inspector for every run.
            • Replay & Verification: Verify any past Execution Record.
            • Governance: Enforce project-wide policies (budgets, network access) during execution.
            • Portability: Export/import projects (.ixp) and Execution Records (.car.json).

    • V1.X (Post-MVP): Sprints 4+
        • Goal: Evolve from a control plane into an intelligent workflow builder and a trusted hub for verifiable knowledge.
        • Key V1.X Features:
            • Interactive Checkpoints: Reintroduce stateful chat sessions as a powerful, verifiable step type within a workflow.
            • Intelligent Orchestration: Support branching workflows, comparisons, and reusable templates.
            • Optional Blockchain Anchoring, Collaboration & CLI tools.


    • The Verification Ecosystem: The intelexta-verify CLI

        • To complete the trust chain and enable third-party validation, the Intelexta project includes a crucial companion: intelexta-verify. This lightweight, open-source CLI verifies exported CARs without requiring the desktop application.
        • Core Functions:
            1. Integrity Check: Verifies the CAR's internal signatures and hash-chain to prove it is tamper-evident.
            2. Reproducibility Check: Reads the RunSpec from the CAR and re-executes the workflow to prove the results are reproducible.
            3. Governance Check: Confirms policy linkage, budget reconciliation, and rerun references to certify that exported runs honored project rules.
            4. Timestamp Check (Optional): For anchored CARs, query public blockchains to confirm immutable timestamps.

7. Core Data Structures & Technical Concepts
    • Proof Modes (at the Step/Checkpoint level):
        • Exact: For deterministic tasks. Acceptance requires a bytewise digest match.
        • Concordant: For stochastic tasks. Acceptance requires passing declared invariants (e.g., semantic distance ≤ ε).
        ◦ Interactive Chat (Post-V1): For conversational workflows. Acceptance is a "process proof"—verifying the sequence of turns within the chat.


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


**Intelexta - Sprint 2B Milestone Plan (Workflow Foundation)**
Based on: PROJECT_CONTEXT.md v5
Date: September 23, 2025 (Assumes start after S2A completion)

1. Sprint Goal
"Deliver the schema, orchestrator plumbing, and UI needed to author Steps sequences, edit them safely."
This sprint turns the EditorPanel into a full workflow builder with checkpoint CRUD support while establishing policy-aware persistence.
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


Sprint 2C: The Great Refactor & Core Features

Sprint Goal: Fully refactor the application to adopt the "Reusable Template" model for workflows, and then build the key V1 features of Step Chaining and Interactive Chat Steps.

Phase 1: Foundational Refactoring (Get it Clean)

    Task 1: Refactor the Database Schema

        Action: Create the next migration file to perform two key changes:

            Rename the run_checkpoints table to run_steps for clarity.

            Move the proof configuration to the step level: Remove proof_mode from the runs table and add both proof_mode (TEXT) and epsilon (REAL) columns to the new run_steps table.

    Task 2: Implement the "Reusable Template" Execution Logic

        Action: Rework the logic of the workflow buttons to be non-destructive and intuitive:

            Execute Full Run: Ensure this button is always active. Clicking it must always create a new, separate execution record in the Inspector.

            Reopen Run: This button must be removed from the UI.

            Clone Run: This action must only clone the workflow definition (the run_steps). The new clone must appear in the Workflow Builder only, with no execution history.

    Task 3: Refine the Step Editor UI

        Action: Update the UI for editing a step:

            The controls for setting the Proof Mode (Exact / Concordant) must be present here.

            When Concordant is selected, the control for setting the epsilon value must be a slider ranging from 0.0 to 1.0.

Phase 2: Core V1 Feature Development

    Task 4: Implement the "Interactive Chat" Step 💬

        Backend: Define a new step type for "Interactive Chat." When the orchestrator executes this type, it must initiate a stateful session. Create a new API command to handle submitting turns within this active chat step.

        Frontend: Allow users to add an "Interactive Chat" step to their workflow. When this step is "run" or "opened," it must launch a dedicated conversational UI. A "Finalize Chat" button will conclude the step's execution and save the full transcript as the step's output.

    Task 5: Enable Simple, Linear Chaining →

        Backend: In the orchestrator, implement a simple templating system. Before executing a step, scan its prompt for placeholders like {{step-1.output}} and replace them with the actual output from the specified previous step.

        Frontend: Provide a simple UI helper in the Step Editor for users to easily insert these placeholder references into their prompts.

Acceptance Criteria (Definition of Done)

    The database schema is updated: the run_steps table exists, and the runs table no longer has proof mode columns.

    The Reopen Run button is gone, and the Execute Full Run and Clone Run buttons behave according to the new, non-destructive logic.

    The epsilon slider appears correctly for Concordant steps in the Step Editor.

    A user can successfully create and complete a workflow that includes an "Interactive Chat" step.

    A user can successfully create a two-step workflow where the second step correctly uses the output from the first via the {{...}} syntax.

---old:
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
