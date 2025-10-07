
### **Architectural Shift: The Dual-Engine and Verifiable Governance Model**

This plan introduces two powerful architectural upgrades that we will adopt:

1.  **Dual-Engine Adapters:** We will abstract all LLM calls behind a `ModelAdapter` trait in the backend. This allows us to support multiple engines (Claude, OpenAI/"Codex", Ollama) side-by-side and choose the right one for each step.
2.  **Signed Model Catalog:** All model information (costs, energy use, network requirements) will live in a cryptographically signed `model_catalog.toml` file. This makes our governance and cost control provably accurate and tamper-evident.

---
### **Unified MVP Roadmap**

#### **Phase 1: Enabling Full, Verifiable Outputs ** ⚠️

**Goal:** Implement the critical blockers that make the "proof" workflow a reality.

* **Task 1.1: Implement the Attachment Store (Full Outputs)**
    * **Action:** Create the `checkpoint_blobs` table or an `attachments/` directory to store the full, untruncated outputs of every step. Update the `persist_checkpoint` logic to save previews in `checkpoint_payloads` and full outputs in the new attachment store.
    * **UI Impact:** The "Checkpoint Details" panel will get a **"Download full payload"** button.

* **Task 1.2: Implement CAR Export with Attachments**
    * **Action:** Update the `emit_car` command to package the full payload attachments along with the rest of the execution record. The CAR must be a self-contained, verifiable proof that includes all the data.

    ### **Phase 1 Action Plan: Critical V1 Blockers**

**Sprint Duration**: 
**Goal**: Implement the foundational features for a complete, verifiable workflow: persisting full outputs, establishing verifiable governance, and enabling portable proof export.

---
### **Task 1: Full Output Persistence (Attachment Store) ⚠️**

**Priority**: Highest
**Goal**: Ensure the full, untruncated output of every step is saved, making true verification possible.

#### Action Items:

1.  **Backend - Create Attachment Store**: Implement a new `checkpoint_blobs` table or an `attachments/` directory to store full payloads (prompts and outputs) as content-addressed files (hashed by their SHA-256 digest).

2.  **Backend - Update Persistence Logic**: Modify the `persist_checkpoint` function. It should continue to save a truncated preview in the `checkpoint_payloads` table for quick UI display, but it must now also save the **full payload** to the new attachment store and record the full payload's hash on the `checkpoints` row.

3.  **Frontend - Enhance the Inspector**: Update the `CheckpointDetailsPanel` to:
    * Display the truncated preview as it does now.
    * Add a **"Download Full Output"** button that uses a new backend command to retrieve the full attachment by its hash.

---
### **Task 2: Verifiable Model Governance 💰**

**Priority**: High
**Goal**: Replace hardcoded cost estimates with a verifiable, signed catalog of model properties.

#### Action Items:

1.  **Backend - Create Signed Catalog**: Create a new configuration file, `config/model_catalog.toml`, and a corresponding signature file, `model_catalog.toml.sig`. The catalog will list each supported model with its `provider`, `pricing` (per-token), and `nature_cost` coefficients.

2.  **Backend - Implement Catalog Loader**: Create a new module that, on application startup, loads the `model_catalog.toml` and **cryptographically verifies its signature**. If valid, cache the catalog for use by the application.

3.  **Backend - Refactor Costing Logic**: Update all cost estimation functions (`estimate_usd_cost`, `estimate_nature_cost`) to take a `model_id` and use the specific rates from the verified catalog instead of a flat heuristic.

---
### **Task 3: CAR Export with Full Provenance 📦**

**Priority**: High
**Goal**: Enable users to export a complete, self-contained, and verifiable CAR file from the UI.

#### Action Items:

1.  **Frontend - Add Export Button**: Add an **"Export CAR"** button to the "Run Execution" view in the Inspector panel.

2.  **Backend - Enhance CAR Generation**: Update the `emit_car` command to be a complete export tool. When generating a CAR, it must now package:
    * The full execution record (all `checkpoints`).
    * The **full payload attachments** for each checkpoint.
    * The **hash of the model catalog** that was used for the execution.
    * The **hash of the policy revision** that governed the execution.

---
### **Success Criteria for Phase 1**

This phase is complete when:
* ✅ Every step's full output is saved and can be downloaded from the UI.
* ✅ All cost and budget calculations are derived from the signed `model_catalog.toml`.
* ✅ The "Export CAR" button successfully generates a `.car.json` file (or a `.zip` bundle) that contains the complete execution record and all associated full-output attachments.
* ✅ An end-to-end test (Ingest -> Summarize -> Prompt -> Export CAR) is successful.


#### **Phase 2: Implementing Verifiable Governance ** 💰

**Goal:** Build the provable cost-control and policy-auditing engine.

* **Task 2.1: Implement the Signed Model Catalog**
    * **Action:** Create the `config/model_catalog.toml` file with per-model pricing and metadata. Implement the backend logic to load and cryptographically verify this catalog on startup. All cost estimation functions must be updated to use these verifiable rates.

* **Task 2.2: Implement the Dual-Engine `ModelAdapter`**
    * **Action:** Refactor the backend orchestrator to use a `ModelAdapter` trait. Implement adapters for at least two engines (e.g., your local Ollama models and an external one like Claude or OpenAI).

* **Task 2.3: Implement Policy Revisioning**
    * **Action:** Create the `policy_revisions` and `project_contexts` tables. Update the `persist_checkpoint` logic so that every checkpoint is permanently linked to the specific `policy_revision_id` that was active during its execution.


***
### ## **Phase 2: Robust Verification & Governance ** 🎯

**Sprint Goal:** Make the verification process meaningful for modern AI workflows and ensure all governance rules are auditable, while completing the portability feature set.

---
### **Task 2.1: Implement Graded Replay for LLMs**

**Priority**: High
**Goal**: Replace the rigid PASS/FAIL system with a nuanced grading system that is useful for verifying stochastic LLM outputs.

#### Action Items:

1.  **Backend - Enhance Report Structs**: Update the `ReplayReport` and `CheckpointReplayReport` structs. Replace the simple `match_status: bool` with a numeric `similarity_score` (from 0.0 to 1.0) and a `grade` (e.g., "A", "B", "C").

2.  **Backend - Implement Scoring Logic**: Modify the `replay_concordant_checkpoint` function. It must now calculate a similarity score (e.g., using Levenshtein distance or another text similarity metric) between the original and replayed outputs. It will then assign a grade based on how the score compares to the configured `epsilon` bands.

3.  **Frontend - Update Inspector UI**: The Inspector's replay feedback must be updated to display the **grade and similarity score** for concordant steps. This provides a much more informative result than a simple PASS/FAIL badge.

---
### **Task 2.2: Implement Policy & Context Revisioning**

**Priority**: High
**Goal**: Create an auditable history of all policy and project context changes, ensuring every execution can be verified against the exact rules that governed it.

#### Action Items:

1.  **Backend - Add History Tables**: Introduce new `policy_revisions` and `project_contexts` tables to the database. The `update_policy` command must now insert a new, immutable revision instead of overwriting the existing policy.

2.  **Backend - Link Checkpoints to Policy**: Update the `persist_checkpoint` logic. Every checkpoint created during an execution must now store a foreign key (`policy_revision_id`) that permanently links it to the specific policy revision that was active at the moment of execution.

3.  **Frontend - Enhance Context Panel**: The `ContextPanel` should be updated to allow users to view the history of policy changes, providing a clear audit trail.

---
### **Task 2.3: Implement Full Project Portability (IXP)**

**Priority**: Medium
**Goal**: Allow users to export and import their entire workspace, including all workflows, execution histories, and policies.

#### Action Items:

1.  **Backend - Implement IXP Export**: Create the `export_project` command. It must bundle the entire project state—all `runs`, `run_steps`, `policy_revisions`, and all full-payload attachments from the attachment store—into a single, compressed `.ixp` archive.

2.  **Backend - Implement IXP Import**: Create the `import_project` command. This function must safely unpack an `.ixp` archive, validate the integrity of its contents, and insert the data into the user's database without conflicts.

3.  **Frontend - Add Portability Buttons**: Wire up the "Export Project" and "Import .ixp" buttons in the `ContextPanel` to the new backend commands.

---
### **Success Criteria for Phase 2**

This phase is complete when:
* ✅ Replaying a workflow with `Concordant` steps produces a graded result (e.g., "Grade A, 98% similarity").
* ✅ Every checkpoint in the database has a permanent, verifiable link to the exact policy revision that was in effect when it was created.
* ✅ A user can successfully export an entire project to an `.ixp` file and another user can successfully import it.


#### **Phase 3: Robust Verification & Final Polish ** ✨

**Goal:** Make verification meaningful for LLMs, complete the portability features, and polish the UX.

* **Task 3.1: Implement Graded Replay**
    * **Action:** Update the `ReplayReport` to use a numeric `similarity_score` and an A-F `grade` instead of a simple boolean. The `replay_concordant_checkpoint` function must be updated to compute these metrics.
    * **UI Impact:** The Inspector will be updated to show the score and grade for replayed steps.

* **Task 3.2: Implement Project Portability (IXP)**
    * **Action:** Implement the `export_project` and `import_project` commands. The `.ixp` file must bundle the project's entire history, including runs, policies, and all full-payload attachments.

* **Task 3.3: Implement Budget & Cost UX**
    * **Action:** Add a real-time **"Estimated run cost"** projection to the Workflow Builder that updates as you add and configure steps.

* **Task 3.4: End-to-End Testing**
    * **Action:** Perform the full suite of tests outlined in the plan: budget blocking, CAR round-trip verification, replay grade consistency, and import validation.



***
### **Phase 3: Final Polish & Demo Prep ** ✨

**Sprint Goal:** Add the final user experience enhancements, complete the verification ecosystem, and prepare the project for a successful launch or demonstration.

---
### **Task 3.1: Implement Budget & Cost UX**

**Priority**: High
**Goal**: Make the "Cost Control" value proposition obvious and useful to the user by providing real-time feedback.

#### Action Items:

1.  **Frontend - Implement Cost Projection**: In the `EditorPanel`, display a real-time **cost projection** for the entire workflow. This estimate should update automatically as the user adds, removes, or edits steps, using the data from the `model_catalog.toml`.

2.  **Frontend - Implement Budget Tracking**: In the `ContextPanel`, display a simple summary of the project's **remaining budgets** (e.g., "Tokens: 5,234 / 10,000 used"). This gives the user immediate feedback on their consumption.

---
### **Task 3.2: Finalize the `intelexta-verify` CLI**

**Priority**: High
**Goal**: Ship a robust, standalone command-line tool that allows third parties to verify a CAR without needing the full Intelexta application.

#### Action Items:

1.  **Backend (CLI) - Enhance Parsing**: Ensure the `intelexta-verify` CLI can fully parse an exported CAR, including its full-output attachments and all governance hashes (policy, model catalog, etc.).

2.  **Backend (CLI) - Implement Verification Logic**: Implement the `verify` command. It must perform a full, graded replay of the workflow described in the CAR and print a clear, human-readable verification report to the terminal, including the final grade and similarity score.

---
### **Task 3.3: Documentation & Demo Preparation**

**Priority**: Medium
**Goal**: Create the materials needed to clearly communicate the project's value and guide new users.

#### Action Items:

1.  **Documentation**: Write clear, step-by-step guides for the primary use cases (e.g., "How to create a verifiable academic paper appendix").

2.  **Demo Workflows**: Create and save several compelling demo projects and workflows that are ready to be presented at a moment's notice.

3.  **Investor Pitch**: Prepare and practice a concise pitch and live demonstration that highlights Intelexta's four unique selling points (Cost Control, Verifiable Provenance, Energy Accountability, Local-First).

---
### **Success Criteria for Phase 3**

This phase, and the V1 MVP, are complete when:
* ✅ The UI clearly shows users the projected cost of a workflow *before* they run it.
* ✅ The `intelexta-verify` CLI can be given a CAR file and successfully output a valid, graded verification report.
* ✅ The project has a polished demo and clear documentation for its key use cases.

Completing these three phases will result in a defensible, feature-complete, and impressive MVP.