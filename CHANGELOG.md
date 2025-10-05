# Intelexta Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
