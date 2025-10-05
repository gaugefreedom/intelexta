# Document Processing Integration Guide (Updated)

## Current Integration Status

Document processing is **fully integrated** as a first-class workflow step type in Intelexta.

### What Changed Since Initial Conversion

#### ✅ Completed Integration

1. **Database Schema** (Sprint 2B+):
   - Added `step_type` field to distinguish LLM prompts from document ingestion
   - Document ingestion steps store format, source path, and privacy status in `config_json`
   - Run executions properly tracked with foreign key relationships

2. **Orchestrator Integration** (src-tauri/src/orchestrator.rs):
   - `execute_document_ingestion_checkpoint()` function handles document workflow steps
   - Supports PDF, LaTeX, TXT, and DOCX formats
   - Generates canonical JSON output stored in checkpoints
   - Located at: `src-tauri/src/orchestrator.rs:1770-1840`

3. **UI Integration** (app/src/components/):
   - CheckpointEditor supports "Document Ingestion" step type
   - Native file picker for document selection via `@tauri-apps/plugin-dialog`
   - Format dropdown: PDF, LaTeX, TXT, DOCX
   - Privacy status controls
   - Browse button for easy file selection

4. **Format Support**:
   - **PDF**: Full extraction via pdf-extract crate
   - **LaTeX**: Metadata + content parsing to Markdown
   - **TXT**: Plain text with basic metadata (NEW)
   - **DOCX**: Office Open XML parsing with metadata extraction (NEW)

5. **Export/Import Enhancements**:
   - Native save dialogs for CAR and project exports
   - Direct-path exports (no nested directories)
   - Backward compatibility with legacy project formats
   - Proper run_execution_id tracking

#### 📋 Workflow Example

1. Create a new project in Intelexta
2. Add a "Document Ingestion" step in the Workflow Builder
3. Click "Browse" to select a PDF/DOCX/TXT/LaTeX file
4. Choose format from dropdown
5. Set privacy status (public/private)
6. Execute the full workflow
7. Inspect the canonical JSON output in the Inspector
8. Export as CAR for third-party verification

### Architecture

```
User Uploads Document
    ↓
CheckpointEditor (React)
    ↓ (invoke Tauri command)
execute_run_checkpoint (API)
    ↓
Orchestrator::execute_document_ingestion_checkpoint
    ↓
[Format Detection] → PdfExtractor | LatexExtractor | TxtExtractor | DocxExtractor
    ↓
[Intermediate Format] (PdfIntermediate | LatexIntermediate)
    ↓
CanonicalProcessor::process_*_intermediate
    ↓
[Canonical Document] (JSON serialized)
    ↓
Store in checkpoints table with signed provenance
    ↓
Display in Inspector + Export to CAR
```

### Database Schema

Document ingestion checkpoints are stored with:

```sql
-- In run_steps table
step_type = "document_ingestion"
config_json = {
  "format": "pdf|latex|txt|docx",
  "source_path": "/absolute/path/to/document.pdf",
  "privacy_status": "public|private|consent_obtained_anonymized"
}

-- In checkpoints table
-- checkpoint_type references the step
-- outputs_sha256 contains hash of canonical JSON
-- signature provides cryptographic proof
```

### Code Locations

**Backend**:
- **Extractors**: `src-tauri/src/document_processing/extractors/`
  - `pdf.rs` - PDF extraction using pdf-extract crate
  - `latex.rs` - LaTeX parsing with Markdown conversion
  - `txt.rs` - Plain text extraction (NEW)
  - `docx.rs` - DOCX/Office Open XML parsing (NEW)
- **Schemas**: `src-tauri/src/document_processing/schemas.rs`
  - `CanonicalDocument` - Main document structure
  - `DocumentMetadata` - Title, authors, dates, keywords, etc.
  - `ProcessingLog` - Audit trail for extraction steps
- **Processors**: `src-tauri/src/document_processing/processors/canonical.rs`
  - Convert intermediate formats to canonical
  - JSONL serialization/deserialization
  - Deduplication and corpus preparation
- **Orchestrator**: `src-tauri/src/orchestrator.rs:1770-1840`
  - `execute_document_ingestion_checkpoint()` function
- **API**: `src-tauri/src/api.rs`
  - `execute_run_checkpoint()` command

**Frontend**:
- **Step Editor**: `app/src/components/CheckpointEditor.tsx:320-370`
  - Document ingestion UI controls
  - File picker integration
  - Format selection dropdown
- **File Dialog Integration**: Uses `@tauri-apps/plugin-dialog`
  - `handleBrowseDocument()` function for file selection
  - Native OS file picker

### Recent Additions (Latest Session)

#### 1. TXT Extractor (`extractors/txt.rs`)
- Simple plain text file reading via `fs::read_to_string()`
- Basic metadata derived from filename
- Reuses `PdfIntermediate` format for consistency
- No cleaning or transformation (preserves original text)
- Best for: pre-cleaned text, notes, transcripts

#### 2. DOCX Extractor (`extractors/docx.rs`)
- ZIP archive parsing of Office Open XML format
- Text extraction from `word/document.xml` using custom XML parser
- Dublin Core metadata extraction from `docProps/core.xml`
- Supports: title, author, keywords
- Custom state machine parser for `<w:t>` text tags
- Handles common Word document structures

#### 3. UI Updates
- Removed "(not yet supported)" labels from TXT and DOCX options
- Format dropdown shows all 4 formats as fully supported
- File picker dialog accepts all extensions: `.pdf`, `.tex`, `.latex`, `.docx`, `.txt`

#### 4. Native File Dialogs
- Added `@tauri-apps/plugin-dialog` npm package v2.4.0
- Added Rust crate `tauri-plugin-dialog` v2.0.0-rc.5
- Implemented Tauri v2 capabilities system
- Created `capabilities/default.json` with dialog permissions
- Browse button for document path in CheckpointEditor
- Save dialogs for CAR emit and project export

#### 5. Export/Import Improvements
- Created `write_project_archive_to_path()` helper function
- Modified `export_project()` to accept optional output path
- Exports save directly to user-selected location (no nested folders)
- Added backward compatibility for legacy exports:
  - Handle missing `run_execution_id` field with `#[serde(default)]`
  - Auto-generate run_execution entries during import
  - Populate missing fields for old project formats

### Testing Document Processing

#### Backend Unit Tests
```bash
cd src-tauri
cargo test document_processing
```

#### Integration Test (Manual)
1. Start Intelexta application
2. Create a new project
3. Add a "Document Ingestion" step
4. Browse for a test DOCX/TXT/PDF/LaTeX file
5. Select appropriate format
6. Execute Full Run
7. Verify canonical JSON appears in Inspector
8. Check that output contains expected metadata and content
9. Export as CAR and verify signature

#### Test Files Needed
- Sample PDF with text
- Sample LaTeX document (.tex)
- Sample plain text file (.txt)
- Sample DOCX file with metadata

### Configuration Schema Example

When a document ingestion step is saved, the `config_json` looks like:

```json
{
  "format": "docx",
  "source_path": "/home/user/documents/research_paper.docx",
  "privacy_status": "public"
}
```

### Canonical Document Output Example

The checkpoint's output is a serialized `CanonicalDocument`:

```json
{
  "document_id": "sha256:abc123...",
  "source_type": "paper",
  "source_path_absolute": "/home/user/documents/research_paper.docx",
  "source_file_relative_path": "research_paper.docx",
  "original_format": "docx",
  "processing_log": {
    "extraction_tool": "DocxExtractor",
    "extraction_timestamp_utc": "2025-10-04T12:34:56Z",
    "processing_timestamp_utc": "2025-10-04T12:34:57Z",
    "cleaning_steps_applied": [],
    "quality_heuristic_score": null
  },
  "privacy_status": "public",
  "metadata": {
    "title": "Research Paper on AI Safety",
    "authors": ["John Doe", "Jane Smith"],
    "date_published": null,
    "date_accessed_utc": "2025-10-04T12:34:56Z",
    "abstract_text": null,
    "keywords_from_source": ["AI", "safety", "verification"],
    "doi": null,
    "arxiv_id": null
  },
  "cleaned_text_with_markdown_structure": "# Introduction\n\nThis paper discusses...",
  "language": "en",
  "schema_version": "0.2.0",
  "consent_details": null
}
```

### Integration with Intelexta's Verifiable Workflow

Document processing integrates seamlessly with Intelexta's core verification model:

1. **Extract**: Process documents through format-specific extractors
2. **Hash**: Generate SHA-256 hash of canonical document (document_id)
3. **Store**: Save canonical JSON in checkpoint with provenance tracking
4. **Sign**: Create Ed25519 signature over checkpoint hash chain
5. **Verify**: Replay workflow to regenerate identical canonical output
6. **Export**: Package in CAR format for third-party verification

### Differences from Python Version

This Rust implementation differs from the original Python `sci-llm-data-prep` in:

1. **LaTeX Processing**:
   - Simplified regex-based approach instead of `pylatexenc`
   - Covers common LaTeX patterns (sections, formatting, metadata)
   - Handles most standard LaTeX documents
   - May need enhancement for complex packages or custom macros

2. **PDF Processing**:
   - Uses `pdf-extract` instead of `unstructured` library
   - Simpler text extraction focused on clean text
   - Basic metadata guessing via heuristics
   - Can be enhanced with more sophisticated extraction

3. **Performance**:
   - Rust provides 10-100x better performance for large-scale processing
   - Lower memory footprint
   - Easy parallelization potential with rayon

4. **Type Safety**:
   - Compile-time guarantees via Rust's type system
   - Runtime validation vs compile-time verification
   - Serde for robust serialization/deserialization

### Future Enhancements

Potential improvements on the roadmap:

- [ ] **ODT Format**: OpenDocument Text support
- [ ] **RTF Format**: Rich Text Format support
- [ ] **Batch Processing**: Process multiple documents in one step
- [ ] **OCR Support**: Extract text from scanned PDFs using Tesseract
- [ ] **Table Extraction**: Preserve table structures from PDFs
- [ ] **Enhanced LaTeX**: Use dedicated crate for complex documents
- [ ] **Multi-document Workflows**: Compare and merge multiple documents
- [ ] **Quality Scoring**: Heuristics for extraction quality assessment
- [ ] **Parallel Processing**: Use rayon for concurrent batch processing
- [ ] **LLM Refinement**: Optional AI-powered extraction improvement

### Troubleshooting

#### Issue: "Failed to open DOCX file"
- **Cause**: File is not a valid ZIP archive or is corrupted
- **Solution**: Verify file is a genuine .docx (not renamed .doc)

#### Issue: "Invalid DOCX file: word/document.xml not found"
- **Cause**: DOCX file has non-standard structure
- **Solution**: Try opening in Word and re-saving, or use different format

#### Issue: PDF extraction returns empty text
- **Cause**: PDF is scanned image without text layer
- **Solution**: OCR support needed (future enhancement)

#### Issue: LaTeX parsing fails on complex documents
- **Cause**: Regex-based parser doesn't handle all LaTeX constructs
- **Solution**: Simplify LaTeX or wait for enhanced parser

### Developer Notes

#### Adding a New Format

To add support for a new document format:

1. Create extractor in `src-tauri/src/document_processing/extractors/`
2. Implement `extract()` method returning `PdfIntermediate` or `LatexIntermediate`
3. Export from `extractors/mod.rs`
4. Add high-level API in `document_processing/mod.rs`
5. Update orchestrator match statement in `orchestrator.rs`
6. Add format option to UI dropdown in `CheckpointEditor.tsx`
7. Update file picker filters to include new extension
8. Add tests

#### Schema Consistency

All extractors must return either:
- `PdfIntermediate` (for simpler formats: PDF, TXT, DOCX)
- `LatexIntermediate` (for structured formats: LaTeX, potentially HTML)

Both are converted to `CanonicalDocument` by `CanonicalProcessor`.

### License

Same as parent Intelexta project.
