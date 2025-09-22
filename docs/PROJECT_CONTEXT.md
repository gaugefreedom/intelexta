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
    • Policy & Budget Governance: Policies set allowances at the project scope, while each checkpoint records the configuration snapshot it executed under so budget reconciliations and overrides are provable and auditable.
    • Audit Trail: Every policy change, budget denial, or rule violation becomes a signed, verifiable "incident" checkpoint in the project's history.
    • Privacy: No data leaves the user's machine unless explicitly exported by the user in a portable format (.ixp or .car.json).

4. User Experience Philosophy: Control Plane for Checkpointed Runs
Intelexta deliberately trades the raw velocity of a conventional chat UI for the rigor of a verifiable control plane.
    • Checkpointed Workflow Editing: A run is curated as an editable sequence of checkpoints. Until a run is sealed, users can reorder, refine, or replace checkpoints, and every adjustment produces a new canonicalized state that is ready to be signed.
    • Verifiability Over Velocity: Tools like Cursor are optimized for developer velocity. Intelexta is optimized for process verifiability. Every UI element exists to surface checkpoint metadata, budgets, and signatures so that edits remain accountable.
    • Sprint 1A Interface ("Launch Control"): The EditorPanel is a structured workflow builder, not a chat box. Its explicit fields (RunSpec) define the initial checkpoint sequence and provide the evidence that gets canonicalized, hashed, and signed to initiate a verifiable run.
    • Foundations for the Workflow Builder: The current orchestrator.rs module and the dag_json field in the RunSpec are the foundational seeds for the advanced workflow builder capability. The V1 product focuses on perfecting single-step ("single-node DAG") execution and editable checkpoint sequencing. Future versions will expand the orchestrator's capabilities to manage complex, multi-node DAGs for intelligent pipelines.
    • Post-V1 Interactive Symbiosis: Interactive co-creation remains a research track that will arrive after V1. The eventual Interactive proof mode will resemble a notebook or chat, but each turn (human prompt, AI response) will be captured as a distinct, signed checkpoint in the hash-chain.

5. Artifact Taxonomy
    • Checkpoint: The atomic unit of proof and configuration. A signed, hash-chained record of a single step in a workflow that captures its inputs, outputs, policy snapshot, and budget reconciliation against the project-wide allowance.
    • Run: An editable, versioned sequence of checkpoints defined by a specific RunSpec and proof mode. Runs can be iterated until sealed; the final, immutable sequence becomes the reference for replay and receipts.
    • CAR (Content-Addressable Receipt): A portable, self-contained JSON file that serves as a verifiable receipt for a single run, including the ordered checkpoint configurations and their budget compliance.
    • IXP (Intelexta Project Export): A compressed archive (.zip) containing the entire project state: project.json, the governing policy.json, and all associated runs, checkpoints, and CARs so project-wide budgets and checkpoint-level evidence stay linked.

 6. Product Roadmap & Future Capabilities

(This new section formally documents the path from MVP to the full vision.)

Intelexta is developed through a phased roadmap. The V1.0 release focuses on establishing the core "proof engine" as a robust, usable tool. Post-V1 releases will expand its power and ecosystem connectivity.

    • V1.0 (Core Engine): Sprints 1-3

        • Goal: Ship a polished, local-first control plane for verifiable AI workflows.

        • Key Features: Exact and Concordant proof modes, editable checkpoint sequencing for runs, portable signed CAR generation, project export/import (.ixp), and integration with local and online AI models under strict governance.

    • V1.X (Post-MVP): Sprints 4+

        • Goal: Evolve from a control plane into an intelligent workflow builder and a trusted hub for verifiable knowledge.

        • Intelligent Orchestration (DAG Engine): The orchestrator.rs will be enhanced to support multi-step, branching workflows defined in dag_json. This will enable users to chain AI calls, run comparisons, and build complex, reproducible research pipelines.

        • Interactive Co-Agency: The Interactive proof mode, including conversational UX, will be delivered after V1 once the checkpointed workflow builder is stable. Each turn (human prompt, AI response) will materialize as a checkpoint so the transcript itself remains auditable.

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



**Intelexta - Sprint 2B Milestone Plan (Workflow Builder Foundation)**
Based on: PROJECT_CONTEXT.md v4
Date: September 23, 2025 (Assumes start after S2A completion)

1. Sprint Goal
"Establish the workflow builder foundation so runs can be authored as editable checkpoint sequences before sealing."
This sprint transforms the current launch form into a sequencing tool that captures configuration at each checkpoint while honoring project-wide policies.
2. Actionable Steps & Tasks
Phase 1: Data Model & API Support for Editable Sequences
    1. Task: Introduce run revisioning fields (e.g., version INTEGER, sealed_at TIMESTAMP NULL) so drafts and final runs are distinct.
    2. Task: Extend checkpoints to include config_json and budget_snapshot JSON blobs that capture per-checkpoint overrides bound to project budgets.
    3. Task: Add Tauri commands for creating, updating, reordering, and deleting draft checkpoints prior to sealing a run, with guardrails that block edits once sealed.
Phase 2: EditorPanel Workflow Builder UX
    1. Task: Replace the single-submit form with a checkpoint timeline editor that supports adding steps, cloning an existing checkpoint, and editing configuration for each node.
    2. Task: Surface live budget tallies in the builder so editors see remaining project allowances as they adjust checkpoint usage estimates.
    3. Task: Add a "Seal Run" action that snapshots the final sequence, signs the initial checkpoint, and transitions the run into execution.
Phase 3: Governance Alignment
    1. Task: Ensure enforce_budget validates each checkpoint edit against the latest project budget before it can be persisted.
    2. Task: Record policy_snapshot_id on checkpoints so later audits can prove which policy version governed the configuration.
    3. Task: Update replay logic to reference the sealed checkpoint sequence rather than the mutable draft state.
3. Acceptance Criteria (Definition of "Done")
    • WFB-01: Draft runs support checkpoint insert, reorder, and delete operations until sealed, with all edits logged in the audit trail.
    • WFB-02: The EditorPanel displays project budget consumption projections per checkpoint and prevents sealing if projections exceed allowances.
    • WFB-03: Sealing a run freezes the checkpoint sequence, records the governing policy snapshot, and produces a signed initial checkpoint ready for execution.

**Intelexta - Sprint 2C Milestone Plan (Chaining & Inspector Visualization)**
Based on: PROJECT_CONTEXT.md v4
Date: September 30, 2025 (Assumes start after S2B completion)

1. Sprint Goal
"Enable checkpoint chaining and rich inspector visualization so users can reason about multi-step workflows and their budget impact."
This sprint extends the workflow builder to orchestrate sequential and branching logic while giving inspectors the tools to validate each transition.
2. Actionable Steps & Tasks
Phase 1: Orchestrator Chaining Enhancements
    1. Task: Expand dag_json to support explicit node identifiers, dependencies, and checkpoint templates for each node.
    2. Task: Update orchestrator.rs to execute nodes in dependency order, persisting a checkpoint after each node and emitting incident checkpoints when prerequisites fail.
    3. Task: Add resumable execution that can restart from the last successful checkpoint when a node fails and a human edits its configuration.
Phase 2: Inspector Visualization
    1. Task: Build a graph/timeline hybrid view that displays each checkpoint, its upstream dependencies, and accumulated budget usage.
    2. Task: Allow selecting a checkpoint to reveal its config_json, policy snapshot, and diff against prior revisions.
    3. Task: Surface replay status per checkpoint so reviewers can see which steps have been re-verified.
Phase 3: Governance & Replay Updates
    1. Task: Update enforce_budget to consume projected usage across chained checkpoints, warning when cumulative totals exceed the project budget.
    2. Task: Extend replay_exact_run and replay_concordant_run to iterate over the chained checkpoints and halt on the first failure with actionable diagnostics.
    3. Task: Store inspector layout metadata (e.g., node positions) as part of the run so exported artifacts can reproduce the visualization context.
3. Acceptance Criteria (Definition of "Done")
    • CHAIN-01: Runs may define multi-node chains with dependencies, and the orchestrator executes them in order, producing checkpoints per node.
    • CHAIN-02: The Inspector displays a visual graph/timeline with budget overlays and lets reviewers inspect the configuration of any checkpoint.
    • CHAIN-03: Replay tools operate over chained checkpoints, reporting per-node status and honoring resumable execution points.

**Intelexta - Sprint 3A Milestone Plan (Portability & Verification)**
Based on: PROJECT_CONTEXT.md v4
Date: October 7, 2025 (Assumes start after S2C completion)

1. Sprint Goal
"Deliver portable artifacts and independent verification so checkpointed workflows can travel and be proven anywhere."
This sprint finalizes the packaging of checkpoint-level evidence, strengthens policy/budget provenance, and equips reviewers with verification tooling.
2. Actionable Steps & Tasks
Phase 1: Artifact Packaging
    1. Task: Update CAR generation to embed the sealed checkpoint sequence, inspector layout metadata, and project budget ledger summaries.
    2. Task: Ensure each checkpoint entry within the CAR links to the policy_snapshot_id and includes the budget reconciliation delta for that step.
    3. Task: Add integrity proofs that confirm the sealed run digest covers the workflow builder draft hash, preventing tampering between authoring and execution.
Phase 2: Project Portability
    1. Task: Expand export_project to package project.json, policy history, runs, checkpoints, inspector layouts, and CARs into the .ixp archive.
    2. Task: Implement import_project with validation that reconciles budgets and replays a sample run to confirm the imported data's integrity.
    3. Task: Provide UI affordances to preview an IXP's contents before import, highlighting any budget conflicts or missing checkpoints.
Phase 3: Verification Tooling
    1. Task: Ship intelexta-verify CLI v1 that can validate CAR signatures, policy linkage, and per-checkpoint budgets offline.
    2. Task: Add a "Verify Externally" button in the Inspector that exports the relevant CAR and launches the CLI with the selected run.
    3. Task: Publish documentation describing how auditors can reproduce verification results, including canonical_json expectations.
3. Acceptance Criteria (Definition of "Done")
    • PORT-01: Exporting a project produces an .ixp that, when unpacked, contains checkpoint sequences, policies, CARs, and inspector metadata with consistent digests.
    • PORT-02: Importing an .ixp re-establishes project-wide budgets and successfully replays at least one run without manual fixes.
    • VERIFY-01: The intelexta-verify CLI validates a CAR's checkpoint chain, policy linkage, and budget compliance, returning actionable errors on mismatch.
