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
    • Human+AI Symbiosis (Agency over Autonomy): Intelexta is a control plane, not an autonomous agent. The human is the strategic director, defining the rules of engagement. This is achieved through local-first keys, user-defined Policies & Budgets, and the ability to verify every step. The AI is a powerful cognitive partner, operating within a provable envelope of human-defined policy.
    • Consciousness & Culture Preservation (Integrity of Knowledge): Intelexta is a provenance engine. By creating a permanent, tamper-evident record of how information is generated (Signed Checkpoints), it preserves the context and integrity of knowledge, solving the modern crisis of attribution. The Content-Addressable Receipt (CAR) acts as a digital provenance card for intellectual work.
    • Energy & Climate Impact (Accountable Efficiency): Intelexta makes energy consumption a first-class design parameter. Carbon emissions (gCO₂e) are a budget that can be set and enforced. The system provides transparent reporting, turning an abstract externality into a concrete, manageable metric.

3. Default Guardrails & Posture
Intelexta is designed with a "secure by default" posture to protect the user and their work.
    • Default Posture: Network egress is disabled by default. Local models are preferred. Cryptographic keys are stored in the OS keychain, not the main database. A content-addressed cache is enabled to prevent redundant computation.
    • Audit Trail: Every policy change, budget denial, or rule violation becomes a signed, verifiable "incident" checkpoint in the project's history.
    • Privacy: No data leaves the user's machine unless explicitly exported by the user in a portable format (.ixp or .car.json).

4. User Experience Philosophy: Control Plane, Not Chat
Intelexta deliberately trades the raw velocity of a conventional chat UI for the rigor of a verifiable control plane.
    • VeVelocity vs. Verifiability: Tools like Cursor are optimized for developer velocity. Intelexta is optimized for process verifiability. Every UI element is designed to support the goal of producing a provable, auditable output.
    • Sprint 1A Interface ("Launch Control"): The EditorPanel is a structured form, not a chat box. This is fundamental, as the form's explicit fields (RunSpec) are the evidence that gets canonicalized, hashed, and signed to initiate a verifiable run.
    • The Path to Intelligent Orchestration: The current orchestrator.rs module and the dag_json field in the RunSpec are the foundational seeds for the advanced "Intelligent Orchestration" capability. The V1 product focuses on perfecting single-step ("single-node DAG") execution across all proof modes. Future versions will expand the orchestrator's capabilities to manage complex, multi-node DAGs for intelligent pipelines.
    • Future Symbiosis (Interactive Mode): The vision for true symbiosis is realized in the Interactive proof mode. This UI will resemble a notebook or chat, but each turn (human prompt, AI response) is captured as a distinct, signed checkpoint in the hash-chain. The transcript itself becomes the auditable artifact of co-creation.

5. Artifact Taxonomy
    • Checkpoint: The atomic unit of proof. A signed, hash-chained record of a single step in a workflow.
    • Run: A sequence of checkpoints that captures a complete workflow, defined by a specific RunSpec and proof mode.
    • CAR (Content-Addressable Receipt): A portable, self-contained JSON file that serves as a verifiable receipt for a single Run.
    • IXP (Intelexta Project Export): A compressed archive (.zip) containing the entire project state: project.json, policy.json, and all associated runs, checkpoints, and CARs.

 6. Product Roadmap & Future Capabilities

(This new section formally documents the path from MVP to the full vision.)

Intelexta is developed through a phased roadmap. The V1.0 release focuses on establishing the core "proof engine" as a robust, usable tool. Post-V1 releases will expand its power and ecosystem connectivity.

    • V1.0 (Core Engine): Sprints 1-3

        • Goal: Ship a polished, local-first control plane for verifiable AI workflows.

        • Key Features: Exact, Concordant, and Interactive proof modes; portable, signed CAR generation; project export/import (.ixp); and integration with local and online AI models under strict governance.

    • V1.X (Post-MVP): Sprints 4+

        • Goal: Evolve from a control plane into an intelligent workflow builder and a trusted hub for verifiable knowledge.

        • Intelligent Orchestration (DAG Engine): The orchestrator.rs will be enhanced to support multi-step, branching workflows defined in dag_json. This will enable users to chain AI calls, run comparisons, and build complex, reproducible research pipelines.

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



**Intelexta - Sprint 2B Implementation Plan**
Based on: PROJECT_CONTEXT.md v3
Date: September 23, 2025 (Assumes start after S2A completion)

1. Sprint Goal
"Implement the 'Interactive' proof mode, transforming the EditorPanel into an auditable conversational interface and establishing the foundation for negotiated co-agency."
By the end of this sprint, a user will be able to engage in a multi-turn dialogue with a local AI, with each turn being captured as a verifiable, signed checkpoint. This makes the collaborative process itself the primary, auditable artifact.
2. Actionable Steps & Tasks
This sprint heavily focuses on evolving the EditorPanel UI and enhancing the orchestrator to handle stateful, turn-by-turn interactions.
Phase 1: Backend for Interactive Runs
    1. Task: Evolve the Run & Checkpoint Schema
        ◦ File: src-tauri/src/store/schema.sql
        ◦ Action: Add a parent_checkpoint_id column to the checkpoints table. This allows us to explicitly chain turns in a conversation.
        ◦ Action: Add a turn_index column (INTEGER) to the checkpoints table to maintain strict conversational order.
    2. Task: Refactor Orchestrator for Statefulness
        ◦ File: src-tauri/src/orchestrator.rs
        ◦ Action: Modify the start_run command. If the RunKind is Interactive, it should create the initial run entry and a "Run Started" checkpoint, but it will not execute an AI model immediately. Instead, it prepares for the first human turn.
        ◦ Action: Create a new Tauri command: submit_turn(run_id: String, prompt_text: String) -> Result<AIMessage, ApiError>.
        ◦ Logic for submit_turn:
            1. Create a "Human Input" checkpoint containing the prompt_text, turn_index, and linking to the previous checkpoint.
            2. Call the local LLM with the prompt_text (and potentially a summary of the conversation history).
            3. Track the usage metrics for this turn.
            4. Create an "AI Output" checkpoint containing the AI's response text, usage data, turn_index, and linking to the "Human Input" checkpoint.
            5. Return the AI's response and its checkpoint ID to the frontend.
    3. Task: Implement Replay for Interactive Mode
        ◦ File: src-tauri/src/replay.rs
        ◦ Action: Implement replay_interactive_run(run_id: String).
        ◦ Logic (Process Proof): This replay is simpler than the others. It does not re-execute the LLM. It queries the database for all checkpoints associated with the run_id, verifies the cryptographic hash-chain and signatures for every checkpoint in order, and confirms the turn_index is sequential. If the entire chain is valid, the process proof is successful.
        ◦ Action: Update the main replay_run command in api.rs to call this new function when the run kind is Interactive.
Phase 2: Frontend for Conversational UI
    1. Task: Transform the EditorPanel
        ◦ File: app/src/components/
        ◦ Action: Use a state variable (e.g., runMode) to conditionally render the UI.
        ◦ If runMode is 'Configuration' (default): Show the existing "Launch Control" form.
        ◦ If runMode is 'Interactive': Show a new conversational UI.
        ◦ Action: When a user configures and starts a new run with kind: 'Interactive', switch the EditorPanel's state to runMode: 'Interactive'.
    2. Task: Build the Conversational Interface
        ◦ File: app/src/components/
        ◦ Action: Design and build the conversational UI. This should include:
            ▪ A message display area that shows the history of human prompts and AI responses.
            ▪ A text input area for the user to type their next prompt.
            ▪ A "Send" button.
        ◦ Logic:
            ▪ Use a useState array (e.g., const [messages, setMessages] = useState([])) to hold the conversation history.
            ▪ When a run begins, fetch all existing checkpoints for that run and populate the messages state.
            ▪ The "Send" button's onClick handler will call the new submit_turn Tauri command.
            ▪ When submit_turn returns, append both the user's new prompt and the AI's response to the messages array to update the display.
    3. Task: Enhance UI with Attribution
        ◦ File: app/src/components/
        ◦ Action: Style the message display to clearly distinguish between "Human" and "AI" turns.
        ◦ Action: For each message bubble, add a small, clickable icon or link that reveals the underlying checkpoint_id. This visually connects the conversation to its verifiable proof on the backend.
    4. Task: Visual feedback during the AI's turn. When the user's message is sent, immediately show a "placeholder" AI message with a typing indicator or spinner. Replace it with the real message when the submit_turn call returns. This dramatically improves the user experience of the Interactive mode, making it feel responsive and alive, just like a modern chat application.
3. Acceptance Criteria (Definition of "Done")
    • INTERACTIVE-01 (Backend): Calling submit_turn correctly creates two new, linked checkpoints (one for human input, one for AI output) in the database with sequential turn_index values.
    • INTERACTIVE-02 (UI): Starting a new Interactive run correctly switches the EditorPanel from the configuration form to the conversational UI.
    • INTERACTIVE-03 (E2E Loop): Typing a message in the conversational UI and clicking "Send" successfully triggers the submit_turn command, and the AI's response is displayed correctly in the message history.
    • REPLAY-03 (Process Proof): Replaying an Interactive run successfully verifies the entire checkpoint chain and signature integrity, returning a "PASS" status for the process proof.
    • UI-04 (Attribution): Each message in the conversation view is clearly marked as either "Human" or "AI," and provides a way to view its associated checkpoint ID.


**Intelexta - Sprint 3A Implementation Plan (V1 Polish)**
Based on: PROJECT_CONTEXT.md v3
Date: September 30, 2025 (Assumes start after S2B completion)

1. Sprint Goal
"Transition from a feature-complete prototype to a polished and shippable V1.0 by implementing project portability (export/import), robust asynchronous feedback, comprehensive error handling, and the first version of negotiated co-agency."
This sprint focuses on the critical "last mile" features that make a tool truly usable and trustworthy. It ensures users can manage their data, understand what the application is doing, and recover gracefully from errors.
2. Actionable Steps & Tasks
This sprint is focused on hardening the application and adding key features that enable real-world collaboration and long-term use.
Phase 1: Implement Project Portability
    1. Task: Implement Project Export (IXP)
        ◦ File: src-tauri/src/api.rs
        ◦ Action: Implement a new Tauri command: export_project(project_id: String) -> Result<String, ApiError>.
        ◦ Logic:
            1. Create a temporary directory.
            2. Fetch and write project.json and policy.json for the given project.
            3. Iterate through all associated runs, checkpoints, and receipts, writing each to a structured directory (e.g., runs/<run_id>/spec.json, runs/<run_id>/checkpoints/<, etc.).
            4. Use a crate like zip to compress the entire directory into a single <project_name>.ixp file.
            5. Save the .ixp file to the user's "Downloads" directory.
            6. Return the final path of the exported file to the frontend.
    2. Task: Implement Project Import (Stretch Goal)
        ◦ File: src-tauri/src/api.rs
        ◦ Action: Create a new command: import_project(file_path: String) -> Result<String, ApiError>.
        ◦ Logic: This involves unzipping the .ixp file, validating its contents, and carefully inserting the data into the database, ensuring no ID conflicts. This is a complex task and can be considered a stretch goal for the sprint.
Phase 2: Enhance User Experience & Robustness
    1. Task: Implement Asynchronous Task Handling
        ◦ Problem: AI runs can be slow, and the UI currently freezes during execution.
        ◦ File: src-tauri/src/orchestrator.rs and api.rs
        ◦ Action: Refactor long-running commands like start_run and replay_run to be fully asynchronous. Use tauri::async_runtime::spawn to run the core logic in a background thread.
        ◦ File: app/src/components/ & InspectorPanel.tsx
        ◦ Action: While a run is in progress, disable the "Start Run" and "Replay" buttons and display a prominent loading indicator or spinner. The UI must remain responsive.
    2. Task: Implement a Global Notification System
        ◦ Problem: Errors from the backend (e.g., budget exceeded, LLM failure) are not clearly communicated in the UI.
        ◦ File: app/src/App.tsx or a new context provider
        ◦ Action: Implement a simple "toast" or notification system. Create a global state (e.g., using React Context or Zustand) to manage a list of notifications.
        ◦ File: app/src/lib/api.ts
        ◦ Action: Wrap all invoke calls in a utility that can catch Rust Err results and automatically push a user-friendly error message to the notification state.
    3. Task: Create the "Onboarding" / Empty State
        ◦ File: app/src/components/
        ◦ Action: When the app loads and there are no projects, display a helpful message and a prominent "Create New Project" button instead of a blank panel.
Phase 3: Implement Negotiated Co-Agency
    1. Task: Backend for AI-Proposed Actions
        ◦ File: src-tauri/src/orchestrator.rs
        ◦ Action: In the submit_turn function for Interactive runs, add logic for the AI to "propose" an action. For this sprint, focus on one action: requesting a budget increase.
        ◦ Logic: The AI's response can include a special, structured block (e.g., [ACTION:REQUEST_BUDGET:{"). The orchestrator will parse this, halt execution, and return a special message type to the frontend indicating a user decision is required.
    2. Task: Frontend for Human Approval
        ◦ File: app/src/components/
        ◦ Action: When the frontend receives the "decision required" message, it should render a modal dialog or a special message bubble: "The AI partner requests a 500 token budget increase to continue. [Approve] [Deny]".
        ◦ Action: Create a new Tauri command, resolve_action(run_id: String, decision: bool), that the "Approve/Deny" buttons call. The backend will then record the human's decision as a signed checkpoint and either continue the run or terminate it.
3. Acceptance Criteria (Definition of "Done")
    • EXPORT-01: The "Export Project" feature successfully creates a .ixp zip file containing all the project's data in the correct folder structure.
    • UX-01 (Async): Starting a multi-second AI run displays a loading indicator, keeps the UI responsive, and the indicator disappears upon completion.
    • UX-02 (Errors): A deliberate error in the backend (e.g., trying to read a non-existent file) results in a clear, non-crashing error toast appearing in the UI.
    • UX-03 (Onboarding): Launching the app for the first time (with an empty database) displays a welcoming "empty state" UI that guides the user to create their first project.
    • GOV-02 (Negotiation): The AI can request a budget increase during an Interactive run, the user is prompted in the UI to approve or deny it, and their choice is recorded as a new, signed checkpoint in the run's history.


**Intelexta - Sprint 3B Implementation Plan (Ecosystem & Adoption)**
Based on: PROJECT_CONTEXT.md v3
Date: October 7, 2025 (Assumes start after S3A completion)

1. Sprint Goal
"Expand Intelexta's utility beyond local models by integrating with online AI providers, and accelerate user adoption through a guided onboarding experience and enhanced usability features."
This sprint's focus is on growth. We will break out of the local-only sandbox to connect with the broader AI ecosystem and add features that make the application more intuitive and powerful for daily use, based on anticipated V1 feedback.
2. Actionable Steps & Tasks
This sprint is divided into three key themes: expanding capabilities with online models, improving the new user experience, and adding quality-of-life features for power users.
Phase 1: Ecosystem Expansion (Online Providers)
    1. Task: Implement Secure API Key Management
        ◦ File: src-tauri/src/api.rs (and a new providers.rs module)
        ◦ Action: Create UI and backend logic for managing provider API keys (e.g., OpenAI, Anthropic).
        ◦ Logic: Use the keyring crate to securely store user-provided API keys in the OS keychain, associated with a provider ID (e.g., service: "intelexta-providers", username: "openai_api_key").
    2. Task: Evolve Policy & Governance for Online Models
        ◦ File: src-tauri/src/governance.rs
        ◦ Action: Update the Policy struct to include a list of allowed online providers and their models.
        ◦ Action: Implement a "rate card" system. The governance module must be able to look up the cost-per-token for specific online models to accurately enforce USD budgets.
    3. Task: Update Orchestrator for Network Calls
        ◦ File: src-tauri/src/orchestrator.rs
        ◦ Action: In the execute_llm_run function, add logic to handle online models.
        ◦ Logic: If a run specifies an online model, the orchestrator will fetch the appropriate API key from the keychain, construct an authenticated HTTPS request, and send the prompt to the provider's API. It must also handle network errors gracefully.
    4. Task: Before making any network call, the orchestrator must first check policy.allow_network == true and the list of allowed providers. If false, it must immediately create a signed Incident checkpoint (egress_denied) and stop.
Phase 2: User Onboarding & Experience
    1. Task: Create a First-Launch Tutorial Project
        ◦ Problem: New users are currently met with a blank slate, which can be intimidating.
        ◦ File: src-tauri/src/main.rs (in the setup closure)
        ◦ Action: On the very first launch (e.g., by checking if the database is empty before migration), pre-populate the application with a "Welcome to Intelexta" tutorial project. This project should contain a simple, pre-completed run and a CAR that users can inspect and replay immediately.
    2. Task: Build an In-App Documentation Viewer
        ◦ File: app/src/components/
        ◦ Action: Create a new "Help" or "Guide" panel/view.
        ◦ Content: This view should contain a simple, human-readable explanation of Intelexta's core concepts: What is a Checkpoint? What is a CAR? What are the three Proof Modes? This helps users understand the "why" behind the features.
Phase 3: Quality of Life for Power Users
    1. Task: Implement Search and Filtering
        ◦ Problem: As the number of runs grows, finding specific ones becomes difficult.
        ◦ File: app/src/components/
        ◦ Action: Add a search bar above the project list that allows users to filter runs by name or content within their prompts.
    2. Task: Create a Human-Readable CAR Viewer
        ◦ Problem: Currently, the CAR is a raw JSON file. It's verifiable but not very readable.
        ◦ File: app/src/components/
        ◦ Action: Create a new component, CarViewer.tsx. When a user clicks on a CAR receipt in the UI, instead of just showing the file path, open a dedicated view that renders the CAR's contents as a clean, well-formatted report (e.g., "Run Details," "Budget Consumption," "Provenance Chain," "Replay Result").
    3. Task: Implement intelexta-verify CLI (V1): The goal would be to build a working version of the command-line tool that can perform the Integrity Check and Reproducibility Check for Exact mode runs.
3. Acceptance Criteria (Definition of "Done")
    • ECO-01 (API Keys): A user can navigate to a settings page, enter an OpenAI API key, and have it securely saved in the OS keychain.
    • ECO-02 (Online Run): A run can be configured and successfully executed using an online model (e.g., gpt-4o-mini), with its accurate USD cost (based on the rate card) and token usage recorded in a checkpoint.
    • ONBOARD-01 (Tutorial): On first launch, the app correctly creates and displays a pre-populated "Welcome" project, which includes at least one run that can be successfully replayed.
    • UX-04 (Search): The UI now includes a functional search bar that filters the list of runs in real-time.
    • CAR-05 (Viewer): Clicking on a generated CAR opens a new, dedicated view within the app that presents the receipt's information in a clear, human-readable format.
