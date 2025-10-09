# Intelexta Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

---

## [0.2.0] - 2025-10-09

### ✅ Phase 1 MVP Complete: Cryptographic Integrity Verification

This release completes Phase 1 of the Intelexta roadmap, establishing a full cryptographic integrity verification system for AI workflows.

### Added

#### Standalone Verification Tool (`intelexta-verify`)
- **Trustless CAR Verification**: CLI tool that verifies CAR files without requiring the full application or database
  - Works completely offline (no network or database needed)
  - Reads both `.car.json` (plain JSON) and `.car.zip` (compressed archives) formats
  - Auto-detects file format
  - Human-readable colored terminal output with ✓/✗ indicators
  - JSON output format for automation and CI/CD pipelines
  - Proper exit codes: 0 (verified), 1 (failed)

- **Hash Chain Verification**:
  - Computes `SHA256(prev_chain || canonical_json(checkpoint_body))` for each checkpoint
  - Uses JCS (JSON Canonicalization Scheme) via `serde_jcs` for deterministic hashing
  - Verifies `curr_chain` matches computed hash for tamper detection
  - **Detects**: Any modification to checkpoint metadata, timestamps, token counts, or execution order

- **Cryptographic Signature Verification**:
  - Ed25519 signature verification on every checkpoint
  - Verifies each signature against checkpoint's `curr_chain` hash
  - Base64-encoded keys and signatures (matching provenance implementation)
  - **Detects**: Forged checkpoints or unauthorized modifications

- **Content Integrity Verification**:
  - **Config hash**: Computes `SHA256(canonical_json(run.steps))` and verifies against provenance claim #0
    - **Detects**: Modified prompts, changed models, altered step names, tampered workflow configuration
  - **Attachment verification**: Self-verifying content-addressed files in `attachments/` directory
    - Each file is named by its SHA256 hash: `attachments/{hash}.txt`
    - Verifies content matches filename hash (self-verifying property)
    - **Detects**: Modified outputs, tampered attachments, substituted files

#### CAR Export Enhancements
- **Always Include Process Proof**: CAR files now always include `proof.process.sequential_checkpoints` for all workflow types
  - Previously only included for interactive workflows
  - Required for signature verification
  - Backward compatible (old CARs still readable, just prompt for re-export)

- **Full Checkpoint Body Export**: Checkpoints now include all fields needed for verification:
  - `run_id`, `kind`, `timestamp` (for hash chain computation)
  - `inputs_sha256`, `outputs_sha256` (for content integrity)
  - `usage_tokens`, `prompt_tokens`, `completion_tokens` (for cost tracking)
  - Matches orchestrator checkpoint body structure exactly

- **Fixed Duplicate CAR Exports**: Changed `INSERT INTO receipts` to `INSERT OR REPLACE INTO receipts`
  - Prevents "UNIQUE constraint failed: receipts.id" errors when re-exporting
  - CAR IDs are deterministic (based on run content), so re-exports generate same ID

### Fixed

#### Verification Implementation Fixes
- **Corrected Hash Chain Computation**:
  - Was: `SHA256(prev_chain || checkpoint_id)` ❌
  - Now: `SHA256(prev_chain || canonical_json(checkpoint_body))` ✅
  - Added `CheckpointBody` struct matching `orchestrator.rs` implementation

- **Fixed Signature Encoding**:
  - Was: Hex decoding ❌
  - Now: Base64 decoding ✅
  - Added `base64` crate dependency
  - Matches provenance signing implementation

- **Canonical JSON Implementation**:
  - Added `serde_jcs` dependency for JSON Canonicalization Scheme
  - Ensures deterministic hashing across platforms
  - Matches `provenance.rs` canonical JSON function

### Documentation

- **Comprehensive Verification README**: `src-tauri/crates/intelexta-verify/README.md`
  - Installation instructions
  - Usage examples (human and JSON output)
  - Detailed explanation of verification stages
  - CAR file format documentation
  - Phase 2 roadmap (Graded Replay)

- **Project Roadmap**: `ROADMAP.md`
  - Phase 1 (Complete): Cryptographic Integrity ✅
  - Phase 2 (Next): Graded Replay (reproducibility verification)
  - Phase 3 (Future): Visualization & Insights
  - Phase 4 (Future): Advanced Governance
  - Phase 5 (Future): Ecosystem Integration
  - Includes timelines, success metrics, and technical details

- **Updated Main README**: Added verification tool section and roadmap link

### Technical Details

**Files Modified**:
- `src-tauri/src/car.rs` (lines 76-97, 194-208, 266-392) - Enhanced CAR export with full checkpoint bodies
- `src-tauri/src/api.rs` (lines 1355, 1390) - Fixed duplicate CAR export handling
- `src-tauri/crates/intelexta-verify/src/main.rs` - Complete verification implementation
- `src-tauri/crates/intelexta-verify/Cargo.toml` - Added crypto dependencies

**Dependencies Added to `intelexta-verify`**:
- `serde_jcs = "0.1"` - JSON Canonicalization Scheme (JCS)
- `base64 = "0.22"` - Base64 encoding/decoding
- `ed25519-dalek = "2.1"` - Ed25519 signature verification
- `sha2 = "0.10"` - SHA-256 hashing
- `hex = "0.4"` - Hex encoding for hash display
- `colored = "2.1"` - Terminal color output

**Database Schema**: No changes

### Verification Coverage

The verification system now detects ALL forms of tampering:
- ✅ Modified prompts or models in workflow specification (config hash mismatch)
- ✅ Changed attachment files/outputs (attachment content mismatch)
- ✅ Altered checkpoint metadata (hash chain broken)
- ✅ Modified timestamps or token counts (hash chain broken)
- ✅ Forged or invalid signatures (signature verification failed)
- ✅ Broken hash chains (hash chain verification failed)

**Test Results**:
- Untampered CAR: ✅ All checks pass
- Prompt tampering: ❌ Config hash mismatch detected
- Attachment tampering: ❌ Attachment content mismatch detected
- Checkpoint tampering: ❌ Hash chain broken detected

### Breaking Changes

**None** - This release is backward compatible with existing CAR files.

**Note**: CARs exported before v0.2 may not include the `proof.process` field required for signature verification. If you encounter an error about missing process proof, re-export the CAR from Intelexta to include the latest cryptographic evidence.

### Migration Guide

No migration needed. To take advantage of new verification features:

1. **Build the verification tool**:
   ```bash
   cd src-tauri
   cargo build --release --package intelexta-verify
   ```

2. **Re-export workflows** (optional, but recommended):
   - Open existing workflows in Intelexta
   - Export as CAR (File → Export CAR or via API)
   - New CARs will include full cryptographic proof

3. **Verify CARs**:
   ```bash
   ./target/release/intelexta-verify path/to/proof.car.zip
   ```

### What's Next

See [ROADMAP.md](ROADMAP.md) for detailed plans.

**Phase 2 Priority**: Graded Replay
- Re-execute workflows to verify reproducibility
- Semantic similarity scoring for outputs
- Graded reports (A+, A, B, C, F)
- Integration with model adapters

---

## [0.1.x] - Document Processing & Portability

### Added

#### Document Processing
- **Document Ingestion Workflow Steps**: Process documents (PDF, LaTeX, TXT, DOCX) as first-class verifiable workflow steps
  - Full integration with Intelexta's provenance and signature system
  - Canonical document schema with comprehensive metadata extraction
  - Output stored as signed checkpoints in workflow execution history
- **TXT Format Support**: Extract plain text files with basic metadata
  - Simple file reading via `fs::read_to_string()`
  - Title derived from filename
  - No transformation or cleaning (preserves original text)
  - Best for: pre-cleaned text, notes, transcripts
- **DOCX Format Support**: Parse Microsoft Word documents with Office Open XML
  - ZIP archive parsing for Office Open XML structure
  - Text extraction from `word/document.xml`
  - Dublin Core metadata extraction from `docProps/core.xml`
  - Supports title, author, and keywords metadata
  - Custom state machine parser for `<w:t>` text tags

#### Native File Dialogs
- **Browse Button for Documents**: Native OS file picker in workflow editor
  - Select documents when creating/editing workflow steps
  - Filters for supported formats: PDF, LaTeX, TXT, DOCX
  - Integration via `@tauri-apps/plugin-dialog` v2.4.0
- **Save Dialog for CAR Export**: Choose export location for Content-Addressable Receipts
  - Default filename based on run ID
  - Native save dialog with .car.json filter
- **Save Dialog for Project Export**: Choose export location for project archives
  - Default filename based on project ID
  - Native save dialog with .ixp filter
- **Tauri v2 Capabilities System**: Proper permission configuration for dialog plugin
  - Created `capabilities/default.json` with dialog permissions
  - Updated security configuration in `tauri.conf.json`

#### Export/Import Enhancements
- **Direct-Path Exports**: Project exports now save directly to user-chosen location
  - No more nested directory structures (project_id/exports/file.ixp)
  - Created `write_project_archive_to_path()` helper function
  - Modified `export_project()` API to accept optional output path
- **Backward Compatibility for Legacy Exports**:
  - Handle missing `run_execution_id` field in old project exports
  - Auto-generate `run_executions` entries during import
  - Populate missing fields for old checkpoint formats
  - Use `#[serde(default)]` for optional fields
- **Proper Run Execution Tracking**:
  - Create `run_execution` entries before checkpoints during import
  - Maintain foreign key relationships for reproducible execution history
  - Support for both new and legacy project formats

### Fixed

#### Import/Export Issues
- **FOREIGN KEY constraint errors during project import**
  - Root cause: Checkpoints referenced `run_execution_id` without corresponding entry
  - Solution: Auto-create `run_execution` entries during import process
  - Now supports importing both new projects and legacy formats
- **Missing `run_execution_id` field in legacy exports**
  - Root cause: Old exports don't have `run_execution_id` in JSON
  - Solution: Made field optional with `#[serde(default)]`
  - Auto-populate with generated UUID for backward compatibility
- **Export path nesting issue**
  - Root cause: Exports created nested folders (project_id/exports/file.ixp)
  - Solution: Save directly to user-chosen folder location
  - Simplified export logic while maintaining backward compatibility

#### Compilation Issues
- **DocumentMetadata schema alignment across all extractors**
  - Fixed field name mismatches (e.g., `publication_date` → `date_published`)
  - Fixed type mismatches (e.g., `Option<Vec<String>>` → `Vec<String>`)
  - Updated `keywords` → `keywords_from_source`
  - Added all required fields for consistency

### Changed

#### UI Improvements
- **Format Dropdown Updates**: Removed "(not yet supported)" labels
  - TXT and DOCX now shown as fully supported formats
  - Dropdown shows: PDF, LaTeX, TXT, DOCX
- **File Picker Extensions**: Updated to accept all supported document formats
  - Accepts: .pdf, .tex, .latex, .docx, .txt
  - Organized into logical filter groups

#### Technical Improvements
- **Enhanced Portability Module**: Refactored for direct-path exports
  - Made `load_project()` and `load_runs_for_export()` public (crate-visible)
  - Made `RunExport` and `CarAttachment` structs `pub(crate)`
  - Added `write_project_archive_to_path()` for custom export locations
- **Updated Run Step Schema**: Include `step_type` and `config_json` in exports
  - Properly serialize document ingestion configuration
  - Maintain format, source path, and privacy status

### Dependencies

#### Added
- `@tauri-apps/plugin-dialog` ^2.4.0 (npm)
- `tauri-plugin-dialog` 2.0.0-rc.5 (Rust)
- `zip` crate (for DOCX extraction)

#### Updated
- Enhanced `DocumentMetadata` schema with comprehensive field set

---

## [0.1.0] - Previous Work

### Added
- Initial Intelexta desktop application
- Local SQLite database
- Ed25519 signatures and SHA-256 hash chains
- Project management
- Workflow builder (Runs and Steps)
- Inspector panel for execution history
- CAR (Content-Addressable Receipt) generation
- PDF and LaTeX document processing
- Provenance tracking
- Governance policies and budgets
- Replay and verification functionality

---

## Notes

### Versioning
This project is currently in active development (v0.x). Breaking changes may occur between minor versions until v1.0 is released.

### Upgrade Path
When upgrading from older versions:
1. **Export your projects** before upgrading (just in case)
2. After upgrade, **import projects** work automatically with backward compatibility
3. Old exports missing `run_execution_id` will be auto-populated during import

### Future Milestones
- v0.3: Enhanced document processing (ODT, RTF, OCR)
- v0.4: Batch document processing workflows
- v0.5: Multi-document comparison and merging
- v1.0: Production-ready verifiable workflow system
