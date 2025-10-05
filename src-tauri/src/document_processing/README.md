# Document Processing Module

This module provides functionality for extracting, cleaning, and structuring scientific documents (PDFs, LaTeX) into a canonical JSONL format. It's a Rust conversion of the Python [sci-llm-data-prep](https://github.com/YOUR_USERNAME/sci-llm-data-prep) project.

## Overview

The document processing pipeline converts various scientific document formats into a standardized canonical schema, enabling:
- Consistent data representation across different source formats
- Integration into verifiable workflows
- Preparation for downstream tasks (DAPT, RAG, fine-tuning)

## Architecture

```
document_processing/
├── schemas.rs           # Canonical document schema
├── extractors/          # Format-specific extraction
│   ├── pdf.rs          # PDF extraction
│   └── latex.rs        # LaTeX extraction
├── processors/          # Conversion to canonical format
│   └── canonical.rs    # JSONL output and processing
└── utils/              # Utility functions
    └── file_utils.rs   # File handling
```

## Canonical Schema

The `CanonicalDocument` structure represents the standardized format:

```rust
pub struct CanonicalDocument {
    pub document_id: String,              // SHA-256 hash of content
    pub source_type: String,              // "paper", "book_chapter", etc.
    pub source_path_absolute: String,
    pub source_file_relative_path: String,
    pub original_format: String,          // "pdf", "latex", etc.
    pub processing_log: ProcessingLog,
    pub privacy_status: String,           // "public", etc.
    pub metadata: DocumentMetadata,       // Title, authors, abstract, etc.
    pub cleaned_text_with_markdown_structure: String,
    pub language: String,
    pub schema_version: String,
}
```

## Usage Examples

### Process a Single PDF

```rust
use intelexta::document_processing::process_pdf_to_canonical;

let canonical_doc = process_pdf_to_canonical(
    "data/papers/example.pdf",
    Some("public".to_string())
)?;

println!("Extracted: {}", canonical_doc.metadata.title.unwrap_or_default());
```

### Process a LaTeX File

```rust
use intelexta::document_processing::process_latex_to_canonical;

let canonical_doc = process_latex_to_canonical(
    "data/latex/paper.tex",
    Some("public".to_string())
)?;
```

### Process a Directory to JSONL

```rust
use intelexta::document_processing::process_directory_to_jsonl;

// Process all PDFs in a directory
let count = process_directory_to_jsonl(
    "data/raw/papers",
    "data/processed/canonical_corpus.jsonl",
    "pdf",
    true  // overwrite existing file
)?;

println!("Processed {} documents", count);
```

### Read and Process JSONL

```rust
use intelexta::document_processing::{CanonicalProcessor};

// Read canonical documents
let documents = CanonicalProcessor::read_from_jsonl(
    "data/processed/canonical_corpus.jsonl"
)?;

// Deduplicate by document ID
let deduplicated = CanonicalProcessor::deduplicate(documents);

// Prepare DAPT corpus (plain text for LLM training)
CanonicalProcessor::prepare_dapt_corpus(
    &deduplicated,
    "data/processed/dapt_corpus.txt"
)?;
```

## Pipeline Workflow

### Phase 1: Extraction

Extract content from source files:

```rust
use intelexta::document_processing::extractors::{PdfExtractor, LatexExtractor};

// PDF
let pdf_intermediate = PdfExtractor::extract("paper.pdf")?;

// LaTeX
let latex_intermediate = LatexExtractor::extract("paper.tex")?;
```

### Phase 2: Conversion to Canonical

Convert intermediate format to canonical:

```rust
use intelexta::document_processing::CanonicalProcessor;

let canonical = CanonicalProcessor::process_pdf_intermediate(
    pdf_intermediate,
    "/absolute/path/to/paper.pdf",
    Some("public".to_string())
)?;
```

### Phase 3: Output to JSONL

Write canonical documents:

```rust
CanonicalProcessor::write_to_jsonl(
    &[canonical],
    "output.jsonl",
    false  // append
)?;
```

## Supported Document Formats

| Format | Extension | Extractor | Metadata | Status |
|--------|-----------|-----------|----------|--------|
| PDF | .pdf | PdfExtractor | Title, abstract (guessed) | ✅ Full |
| LaTeX | .tex, .latex | LatexExtractor | Title, author, date, abstract | ✅ Full |
| Plain Text | .txt | TxtExtractor | Filename as title | ✅ Full |
| DOCX | .docx | DocxExtractor | Title, author, keywords | ✅ Full |
| DOC | .doc | - | - | ⏳ Planned (via DOCX fallback) |
| ODT | .odt | - | - | ⏳ Planned |

### Format-Specific Notes

#### TXT (Plain Text)
- Reads entire file as-is via `fs::read_to_string()`
- No cleaning or transformation
- Title derived from filename
- Best for: pre-cleaned text, notes, transcripts
- **Added**: Latest session

#### DOCX (Microsoft Word)
- Parses Office Open XML structure (ZIP archive)
- Extracts text from `<w:t>` tags in `word/document.xml`
- Metadata from Dublin Core properties in `docProps/core.xml`
- Custom state machine parser for XML text extraction
- **Note**: Binary .doc format not yet supported (requires external converter)
- **Added**: Latest session

#### PDF
- Uses `pdf-extract` crate for text extraction
- Auto-cleans page numbers, headers, footers
- Metadata extraction via heuristics
- **Limitation**: No OCR for scanned documents (planned enhancement)

#### LaTeX
- Regex-based parser converts to Markdown
- Preserves math notation (inline and display)
- Handles common LaTeX commands
- Extracts sectioning, formatting, citations
- **Limitation**: Complex packages may not parse correctly

## Features

### PDF Processing
- Text extraction using `pdf-extract` crate
- Automatic cleaning (whitespace, page numbers, headers/footers)
- Metadata extraction (title, abstract)
- Category tagging from file paths

### LaTeX Processing
- Metadata extraction (title, author, date, abstract)
- Section conversion to Markdown headings
- Math preservation (inline and display)
- Citation and reference preservation
- Text formatting conversion (bold, italic, etc.)

### Plain Text Processing (NEW)
- Direct file reading with no transformation
- Preserves original text exactly
- Minimal metadata (filename-based)
- Fast and simple extraction

### DOCX Processing (NEW)
- Office Open XML parsing
- Text extraction from document body
- Dublin Core metadata extraction
- Handles common Word document structures

### Canonical Processing
- JSONL serialization/deserialization
- Document deduplication by content hash
- DAPT corpus preparation
- Batch processing support

## Integration with Intelexta

This module is designed to integrate seamlessly with Intelexta's verifiable workflow:

1. **Extract**: Process documents through extractors
2. **Verify**: Use Intelexta's provenance tracking
3. **Store**: Save to canonical format with metadata
4. **Audit**: Track processing steps in `ProcessingLog`

## Differences from Python Version

This Rust implementation differs from the original Python version in:

1. **LaTeX Processing**: Simplified regex-based approach instead of pylatexenc
   - Covers common LaTeX patterns
   - Handles sectioning, formatting, and metadata
   - May need enhancement for complex LaTeX documents

2. **PDF Processing**: Uses `pdf-extract` instead of `unstructured`
   - Simpler text extraction
   - Basic metadata guessing
   - Can be enhanced with more sophisticated extraction

3. **Performance**: Rust provides better performance for large-scale processing

4. **Type Safety**: Compile-time guarantees vs runtime validation

## Future Enhancements

Potential improvements:

- [ ] OCR support for scanned PDFs
- [ ] Enhanced LaTeX parser (possibly using a dedicated crate)
- [ ] Table extraction from PDFs
- [ ] Email processing (MIME format)
- [ ] Quality scoring heuristics
- [ ] Parallel batch processing
- [ ] Integration with LLM refinement step

## Testing

Run tests:

```bash
cd src-tauri
cargo test document_processing
```

## License

Same as parent Intelexta project.
