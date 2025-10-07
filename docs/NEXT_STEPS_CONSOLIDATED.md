# Intelexta Next Steps - Consolidated Roadmap

**Date**: October 7, 2025
**Status**: Phase 1 Complete ✅ | Working Solo
**Previous Work**: Model Catalog Implementation Complete

---

## ✅ Phase 1 Complete: Verifiable Model Governance

**Completed Tasks:**
- ✅ Created signed `config/model_catalog.toml` with 15+ models
- ✅ Implemented `model_catalog.rs` module with signature verification
- ✅ Added catalog initialization to `main.rs`
- ✅ Refactored `governance.rs` to use catalog for cost estimation
- ✅ Added `model_catalog_hash` and `model_catalog_version` to CAR exports
- ✅ Built and tested integration (6 tests passing)
- ✅ Documented implementation

**Impact:**
- Per-model pricing now accurate (local = $0, cloud = real pricing)
- Nature Cost tracking per model
- Energy consumption tracking (kWh)
- CAR exports include catalog hash for verification
- Fallback catalog ensures system reliability

---

## 🎯 Phase 2: Robust Verification & Governance

**Goal**: Make verification meaningful for AI workflows and complete portability features.

### Priority Order (High to Low)

#### **Task 2.1: Full Output Persistence (Attachment Store)** ⚠️ **CRITICAL**

**Problem**: Currently only output hashes are stored, not the actual text. This blocks:
- CAR export completeness
- Replay verification
- Output inspection
- Any meaningful verification

**Current State**:
```rust
// In orchestrator.rs - checkpoints are created with output_text
let checkpoint = Checkpoint {
    output_text: result_output.clone(),  // ✅ Generated
    // ...
};

// But in store.rs persist_checkpoint:
output_payload: Some(serde_json::to_string(&checkpoint.output_text)?), // ⚠️ Limited storage
```

**Database Schema** (already exists):
```sql
CREATE TABLE checkpoints (
    -- ...
    output_payload TEXT,  -- ✅ Column exists but needs enhancement
    -- ...
);
```

**Implementation Plan**:

1. **Backend - Create Attachment Store** (1-2 days)
   ```rust
   // New module: src-tauri/src/attachments.rs

   pub struct AttachmentStore {
       base_path: PathBuf,
   }

   impl AttachmentStore {
       pub fn save_full_output(
           &self,
           checkpoint_id: &str,
           output: &str
       ) -> Result<String> {
           // Hash the output
           let hash = sha256_hex(output.as_bytes());

           // Save to attachments/{hash[0..2]}/{hash}.txt
           let path = self.base_path
               .join(&hash[0..2])
               .join(format!("{}.txt", hash));

           std::fs::create_dir_all(path.parent().unwrap())?;
           std::fs::write(&path, output)?;

           Ok(hash)
       }

       pub fn load_full_output(&self, hash: &str) -> Result<String> {
           let path = self.base_path
               .join(&hash[0..2])
               .join(format!("{}.txt", hash));
           Ok(std::fs::read_to_string(&path)?)
       }
   }
   ```

2. **Backend - Update persist_checkpoint** (1 day)
   ```rust
   // In store.rs
   pub fn persist_checkpoint(
       conn: &Connection,
       checkpoint: &Checkpoint,
       attachment_store: &AttachmentStore,
   ) -> Result<()> {
       // Save full output to attachment store
       let full_hash = attachment_store.save_full_output(
           &checkpoint.id,
           &checkpoint.output_text
       )?;

       // Save preview (first 1000 chars) to database
       let preview = checkpoint.output_text
           .chars()
           .take(1000)
           .collect::<String>();

       conn.execute(
           "INSERT INTO checkpoints (..., output_payload, full_output_hash)
            VALUES (..., ?1, ?2)",
           params![preview, full_hash],
       )?;

       Ok(())
   }
   ```

3. **Database Migration** (30 min)
   ```sql
   ALTER TABLE checkpoints ADD COLUMN full_output_hash TEXT;
   ```

4. **Frontend - Add Download Button** (1 day)
   ```typescript
   // In CheckpointDetailsPanel.tsx

   async function downloadFullOutput(checkpointId: string) {
       const fullOutput = await invoke('download_checkpoint_full_output', {
           checkpointId
       });

       // Trigger browser download
       const blob = new Blob([fullOutput], { type: 'text/plain' });
       const url = URL.createObjectURL(blob);
       const a = document.createElement('a');
       a.href = url;
       a.download = `checkpoint-${checkpointId}.txt`;
       a.click();
   }

   // In the UI:
   <button onClick={() => downloadFullOutput(checkpoint.id)}>
       Download Full Output
   </button>
   ```

5. **Backend - New API Command** (30 min)
   ```rust
   #[tauri::command]
   pub fn download_checkpoint_full_output(
       pool: State<DbPool>,
       checkpoint_id: String,
   ) -> Result<String, Error> {
       let conn = pool.get()?;

       let hash: String = conn.query_row(
           "SELECT full_output_hash FROM checkpoints WHERE id = ?1",
           params![checkpoint_id],
           |row| row.get(0),
       )?;

       let attachment_store = get_global_attachment_store();
       Ok(attachment_store.load_full_output(&hash)?)
   }
   ```

**Estimated Time**: 2-3 days
**Files to Modify**:
- `src-tauri/src/attachments.rs` (new)
- `src-tauri/src/store.rs`
- `src-tauri/src/api.rs`
- `src-tauri/migrations/XXX_add_full_output_hash.sql` (new)
- `app/src/components/CheckpointDetailsPanel.tsx`

**Success Criteria**:
- ✅ Full outputs saved to `attachments/` directory
- ✅ Previews in database for quick display
- ✅ Download button works in Inspector
- ✅ CAR export includes full outputs

---

#### **Task 2.2: CAR Export UI Button** 📦

**Problem**: CAR export exists but has no UI button.

**Implementation Plan**:

1. **Frontend - Add Export Button** (2 hours)
   ```typescript
   // In InspectorPanel.tsx

   async function exportCAR(runId: string, executionId: string) {
       try {
           const carPath = await invoke('emit_car', {
               runId,
               runExecutionId: executionId,
           });

           // Show success message
           toast.success(`CAR exported to: ${carPath}`);
       } catch (err) {
           toast.error(`Failed to export CAR: ${err}`);
       }
   }

   // In the UI (Run Execution view):
   <button onClick={() => exportCAR(run.id, execution.id)}>
       Export CAR
   </button>
   ```

2. **Backend - Return file path** (30 min)
   ```rust
   // In api.rs
   #[tauri::command]
   pub fn emit_car(
       pool: State<DbPool>,
       run_id: String,
       run_execution_id: Option<String>,
   ) -> Result<String, Error> {
       let conn = pool.get()?;
       let car = car::emit_car(&conn, &run_id, run_execution_id.as_deref())?;

       // Write to file
       let path = std::env::temp_dir()
           .join(format!("car-{}.json", run_id));

       let json = serde_json::to_string_pretty(&car)?;
       std::fs::write(&path, json)?;

       Ok(path.to_string_lossy().to_string())
   }
   ```

3. **Enhancement - Bundle with Attachments** (1 day)
   ```rust
   pub fn emit_car_bundle(
       conn: &Connection,
       run_id: &str,
       attachment_store: &AttachmentStore,
   ) -> Result<PathBuf> {
       let car = emit_car(conn, run_id, None)?;

       // Create zip with CAR + all attachments
       let zip_path = std::env::temp_dir()
           .join(format!("car-{}.zip", run_id));

       let file = File::create(&zip_path)?;
       let mut zip = ZipWriter::new(file);

       // Add car.json
       zip.start_file("car.json", FileOptions::default())?;
       zip.write_all(serde_json::to_string_pretty(&car)?.as_bytes())?;

       // Add attachments/
       for checkpoint_id in &car.checkpoints {
           let hash = get_checkpoint_output_hash(conn, checkpoint_id)?;
           let output = attachment_store.load_full_output(&hash)?;

           zip.start_file(
               format!("attachments/{}.txt", checkpoint_id),
               FileOptions::default()
           )?;
           zip.write_all(output.as_bytes())?;
       }

       zip.finish()?;
       Ok(zip_path)
   }
   ```

**Estimated Time**: 1 day
**Files to Modify**:
- `app/src/components/InspectorPanel.tsx`
- `src-tauri/src/api.rs`
- `src-tauri/src/car.rs`

**Success Criteria**:
- ✅ "Export CAR" button visible in Inspector
- ✅ Clicking button generates CAR file
- ✅ File path shown to user
- ✅ CAR includes all attachments in zip bundle

---

#### **Task 2.3: Graded Replay for LLMs** 🎯

**Problem**: Current replay is PASS/FAIL which doesn't work for stochastic LLMs.

**Implementation Plan**:

1. **Backend - Update Report Structs** (1 hour)
   ```rust
   // In replay.rs

   #[derive(Serialize, Deserialize)]
   pub struct CheckpointReplayReport {
       pub checkpoint_id: String,
       pub original_output: String,
       pub replayed_output: String,
       pub similarity_score: f64,  // 0.0 to 1.0
       pub grade: String,          // "A", "B", "C", "D", "F"
       pub passed: bool,
   }

   #[derive(Serialize, Deserialize)]
   pub struct ReplayReport {
       pub run_id: String,
       pub checkpoints: Vec<CheckpointReplayReport>,
       pub overall_grade: String,
       pub overall_similarity: f64,
   }
   ```

2. **Backend - Implement Similarity Scoring** (2 days)
   ```rust
   // Add dependency to Cargo.toml:
   // strsim = "0.11"

   use strsim::normalized_levenshtein;

   fn calculate_similarity_score(original: &str, replayed: &str) -> f64 {
       // Use normalized Levenshtein distance
       // Returns 0.0 (completely different) to 1.0 (identical)
       normalized_levenshtein(original, replayed)
   }

   fn assign_grade(similarity: f64, epsilon: f64) -> String {
       if similarity >= 0.95 { "A" }
       else if similarity >= epsilon { "B" }
       else if similarity >= epsilon * 0.9 { "C" }
       else if similarity >= epsilon * 0.8 { "D" }
       else { "F" }
   }

   pub fn replay_concordant_checkpoint(
       original_output: &str,
       replayed_output: &str,
       epsilon: f64,
   ) -> CheckpointReplayReport {
       let similarity = calculate_similarity_score(original_output, replayed_output);
       let grade = assign_grade(similarity, epsilon);
       let passed = similarity >= epsilon;

       CheckpointReplayReport {
           original_output: original_output.to_string(),
           replayed_output: replayed_output.to_string(),
           similarity_score: similarity,
           grade,
           passed,
       }
   }
   ```

3. **Frontend - Update Replay Display** (1 day)
   ```typescript
   // In InspectorPanel.tsx

   function ReplayGradeBadge({ grade, similarity }: { grade: string, similarity: number }) {
       const colors = {
           'A': 'bg-green-500',
           'B': 'bg-blue-500',
           'C': 'bg-yellow-500',
           'D': 'bg-orange-500',
           'F': 'bg-red-500',
       };

       return (
           <div className={`${colors[grade]} text-white px-3 py-1 rounded`}>
               Grade {grade} ({(similarity * 100).toFixed(1)}% similar)
           </div>
       );
   }

   // In replay results view:
   {replayReport.checkpoints.map(cp => (
       <div key={cp.checkpoint_id}>
           <ReplayGradeBadge grade={cp.grade} similarity={cp.similarity_score} />
           <div className="grid grid-cols-2 gap-4">
               <div>
                   <h4>Original</h4>
                   <pre>{cp.original_output}</pre>
               </div>
               <div>
                   <h4>Replayed</h4>
                   <pre>{cp.replayed_output}</pre>
               </div>
           </div>
       </div>
   ))}
   ```

**Estimated Time**: 3-4 days
**Files to Modify**:
- `src-tauri/Cargo.toml` (add strsim)
- `src-tauri/src/replay.rs`
- `app/src/components/InspectorPanel.tsx`

**Success Criteria**:
- ✅ Replay produces similarity scores (0.0-1.0)
- ✅ Grades assigned (A-F)
- ✅ UI shows grade badges
- ✅ Side-by-side output comparison

---

#### **Task 2.4: Policy Revisioning** 🗂️

**Problem**: Policy changes overwrite history, breaking auditability.

**Implementation Plan**:

1. **Database Migration** (1 hour)
   ```sql
   -- Create policy revisions table
   CREATE TABLE policy_revisions (
       id TEXT PRIMARY KEY,
       project_id TEXT NOT NULL,
       revision_number INTEGER NOT NULL,
       budget_tokens INTEGER NOT NULL,
       budget_usd REAL NOT NULL,
       budget_nature_cost REAL NOT NULL,
       allow_network BOOLEAN NOT NULL,
       created_at TEXT NOT NULL,
       FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
   );

   -- Add policy_revision_id to checkpoints
   ALTER TABLE checkpoints ADD COLUMN policy_revision_id TEXT
       REFERENCES policy_revisions(id);
   ```

2. **Backend - Update Policy Management** (1-2 days)
   ```rust
   // In store/policies.rs

   pub fn update_policy(
       conn: &Connection,
       project_id: &str,
       new_policy: &Policy,
   ) -> Result<String> {
       // Get current revision number
       let rev_num: i64 = conn.query_row(
           "SELECT COALESCE(MAX(revision_number), 0) FROM policy_revisions WHERE project_id = ?1",
           params![project_id],
           |row| row.get(0),
       )?;

       let new_rev_num = rev_num + 1;
       let revision_id = format!("policy-rev-{}-{}", project_id, new_rev_num);

       // Insert new revision (immutable)
       conn.execute(
           "INSERT INTO policy_revisions
            (id, project_id, revision_number, budget_tokens, budget_usd,
             budget_nature_cost, allow_network, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
           params![
               revision_id,
               project_id,
               new_rev_num,
               new_policy.budget_tokens,
               new_policy.budget_usd,
               new_policy.budget_nature_cost,
               new_policy.allow_network,
               Utc::now().to_rfc3339(),
           ],
       )?;

       // Update current policy (still keep for backwards compat)
       conn.execute(
           "UPDATE policies SET ... WHERE project_id = ?1",
           params![...],
       )?;

       Ok(revision_id)
   }

   pub fn get_policy_at_revision(
       conn: &Connection,
       revision_id: &str,
   ) -> Result<Policy> {
       // Load specific revision
   }
   ```

3. **Backend - Link Checkpoints to Revisions** (1 day)
   ```rust
   // In orchestrator.rs

   fn execute_step(...) {
       // Get current policy revision
       let policy_revision_id = get_current_policy_revision(&conn, &project_id)?;

       // ... execute step ...

       // Save checkpoint with revision link
       let checkpoint = Checkpoint {
           policy_revision_id: Some(policy_revision_id),
           // ...
       };

       persist_checkpoint(&conn, &checkpoint)?;
   }
   ```

4. **Frontend - Policy History View** (1-2 days)
   ```typescript
   // In ContextPanel.tsx

   function PolicyHistoryView() {
       const [revisions, setRevisions] = useState([]);

       useEffect(() => {
           invoke('list_policy_revisions', { projectId })
               .then(setRevisions);
       }, [projectId]);

       return (
           <div>
               <h3>Policy History</h3>
               {revisions.map(rev => (
                   <div key={rev.id}>
                       <strong>Revision {rev.revision_number}</strong>
                       <span>{rev.created_at}</span>
                       <div>Budget: {rev.budget_tokens} tokens, ${rev.budget_usd}</div>
                   </div>
               ))}
           </div>
       );
   }
   ```

**Estimated Time**: 3-4 days
**Files to Modify**:
- `src-tauri/migrations/XXX_add_policy_revisions.sql` (new)
- `src-tauri/src/store/policies.rs`
- `src-tauri/src/orchestrator.rs`
- `src-tauri/src/api.rs`
- `app/src/components/ContextPanel.tsx`

**Success Criteria**:
- ✅ Policy changes create new revisions
- ✅ Old revisions preserved immutably
- ✅ Checkpoints linked to exact policy revision
- ✅ UI shows policy history
- ✅ Can view policy at any point in time

---

#### **Task 2.5: Project Portability (IXP Export/Import)** 📁

**Problem**: No way to share entire projects with all history.

**Implementation Plan**:

1. **Backend - IXP Export** (2-3 days)
   ```rust
   // In portability.rs

   pub fn export_project(
       conn: &Connection,
       project_id: &str,
       attachment_store: &AttachmentStore,
   ) -> Result<PathBuf> {
       let zip_path = std::env::temp_dir()
           .join(format!("project-{}.ixp", project_id));

       let file = File::create(&zip_path)?;
       let mut zip = ZipWriter::new(file);

       // Export project metadata
       let project = get_project(conn, project_id)?;
       zip.start_file("project.json", FileOptions::default())?;
       zip.write_all(serde_json::to_string_pretty(&project)?.as_bytes())?;

       // Export all runs
       let runs = list_runs(conn, project_id)?;
       zip.start_file("runs.json", FileOptions::default())?;
       zip.write_all(serde_json::to_string_pretty(&runs)?.as_bytes())?;

       // Export all policy revisions
       let policies = list_policy_revisions(conn, project_id)?;
       zip.start_file("policy_revisions.json", FileOptions::default())?;
       zip.write_all(serde_json::to_string_pretty(&policies)?.as_bytes())?;

       // Export all checkpoints
       let checkpoints = list_all_checkpoints(conn, project_id)?;
       zip.start_file("checkpoints.json", FileOptions::default())?;
       zip.write_all(serde_json::to_string_pretty(&checkpoints)?.as_bytes())?;

       // Export all attachments
       for checkpoint in &checkpoints {
           if let Some(hash) = &checkpoint.full_output_hash {
               let output = attachment_store.load_full_output(hash)?;
               zip.start_file(
                   format!("attachments/{}.txt", hash),
                   FileOptions::default()
               )?;
               zip.write_all(output.as_bytes())?;
           }
       }

       zip.finish()?;
       Ok(zip_path)
   }
   ```

2. **Backend - IXP Import** (2-3 days)
   ```rust
   pub fn import_project(
       conn: &Connection,
       ixp_path: &Path,
       attachment_store: &AttachmentStore,
   ) -> Result<String> {
       let file = File::open(ixp_path)?;
       let mut archive = ZipArchive::new(file)?;

       // Extract and validate project.json
       let project: Project = {
           let mut file = archive.by_name("project.json")?;
           let mut contents = String::new();
           file.read_to_string(&mut contents)?;
           serde_json::from_str(&contents)?
       };

       // Check for ID conflicts
       let existing = conn.query_row(
           "SELECT id FROM projects WHERE id = ?1",
           params![project.id],
           |row| row.get::<_, String>(0),
       ).optional()?;

       if existing.is_some() {
           return Err(anyhow!("Project {} already exists", project.id));
       }

       // Import project
       insert_project(conn, &project)?;

       // Import policy revisions
       let policies: Vec<PolicyRevision> = {
           let mut file = archive.by_name("policy_revisions.json")?;
           let mut contents = String::new();
           file.read_to_string(&mut contents)?;
           serde_json::from_str(&contents)?
       };
       for policy in policies {
           insert_policy_revision(conn, &policy)?;
       }

       // Import runs, checkpoints, attachments...

       Ok(project.id)
   }
   ```

3. **Frontend - Export/Import Buttons** (1 day)
   ```typescript
   // In ContextPanel.tsx

   async function exportProject() {
       try {
           const path = await invoke('export_project', {
               projectId: currentProject.id
           });
           toast.success(`Project exported to: ${path}`);
       } catch (err) {
           toast.error(`Export failed: ${err}`);
       }
   }

   async function importProject() {
       try {
           const selected = await open({
               filters: [{ name: 'Intelexta Project', extensions: ['ixp'] }]
           });

           if (selected) {
               const projectId = await invoke('import_project', {
                   ixpPath: selected
               });
               toast.success(`Imported project: ${projectId}`);
               refreshProjects();
           }
       } catch (err) {
           toast.error(`Import failed: ${err}`);
       }
   }
   ```

**Estimated Time**: 5-6 days
**Files to Modify**:
- `src-tauri/src/portability.rs`
- `src-tauri/src/api.rs`
- `app/src/components/ContextPanel.tsx`

**Success Criteria**:
- ✅ Export button creates .ixp file
- ✅ IXP contains all project data
- ✅ Import button accepts .ixp files
- ✅ Imported project fully functional
- ✅ No data loss in export/import cycle

---

## 📊 Phase 2 Summary

**Total Estimated Time**: 3-4 weeks

**Task Breakdown**:
1. Full Output Persistence: 2-3 days ⚠️ **CRITICAL**
2. CAR Export UI Button: 1 day
3. Graded Replay: 3-4 days
4. Policy Revisioning: 3-4 days
5. Project Portability: 5-6 days

**Order of Execution** (Recommended):
1. **Week 1**: Full Output Persistence (Task 2.1) - blocks everything else
2. **Week 1-2**: CAR Export UI (Task 2.2) - quick win
3. **Week 2**: Graded Replay (Task 2.3) - important for verification
4. **Week 3**: Policy Revisioning (Task 2.4) - governance foundation
5. **Week 3-4**: Project Portability (Task 2.5) - sharing capability

**Success Criteria for Phase 2**:
- ✅ Full outputs saved and downloadable
- ✅ CAR export works from UI with attachments
- ✅ Replay produces meaningful grades (A-F)
- ✅ Policy history is auditable
- ✅ Projects can be exported/imported

---

## 🚀 Phase 3: Standalone Verification CLI (Future)

**Goal**: Build `intelexta-verify` - a standalone tool for verifying CAR files.

This is lower priority but important for the "trustless verification" value prop.

**Tasks**:
- Create new Rust crate `crates/intelexta-verify`
- Implement CAR parsing without database
- Verify signatures and hash chains
- Run graded replay
- Generate verification reports
- Publish as standalone binary

**Estimated Time**: 1-2 weeks

---

## 🎯 Recommended Next Action

**Start with Task 2.1: Full Output Persistence**

This is the most critical blocker. Without full outputs:
- CAR export is incomplete
- Replay can't verify anything
- Users can't inspect results
- All other verification features are blocked

Once outputs are persisted, the rest of Phase 2 can proceed in parallel if needed.

**Would you like me to start with Task 2.1?**
