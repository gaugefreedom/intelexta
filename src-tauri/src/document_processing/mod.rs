// Document Processing Module
//
// This module provides functionality for extracting, cleaning, and structuring
// scientific documents (PDFs, LaTeX) into a canonical JSONL format.
//
// Inspired by and converted from the Python sci-llm-data-prep project.
//
// Main components:
// - schemas: Canonical document schema definitions
// - extractors: PDF and LaTeX content extraction
// - processors: Convert to canonical format and JSONL output
// - utils: File handling and utility functions
//
// Usage:
//   1. Extract content from PDFs or LaTeX files using extractors
//   2. Process to canonical format using processors
//   3. Output to JSONL for downstream tasks (DAPT, RAG, etc.)

pub mod schemas;
pub mod extractors;
pub mod processors;
pub mod utils;

// Re-export commonly used types
pub use schemas::{
    CanonicalDocument,
    DocumentMetadata,
    ProcessingLog,
    ConsentDetails,
    PdfIntermediate,
    LatexIntermediate,
};

pub use extractors::{PdfExtractor, LatexExtractor, TxtExtractor, DocxExtractor};
pub use processors::CanonicalProcessor;
pub use utils::{find_files_by_extension, get_relative_path, ensure_dir_exists};

use std::path::Path;
use anyhow::Result;

/// High-level API for processing PDFs to canonical format
pub fn process_pdf_to_canonical(
    pdf_path: impl AsRef<Path>,
    privacy_status: Option<String>,
) -> Result<CanonicalDocument> {
    let pdf_path = pdf_path.as_ref();

    // Extract from PDF
    let intermediate = PdfExtractor::extract(pdf_path)?;

    // Convert to canonical
    let canonical = CanonicalProcessor::process_pdf_intermediate(
        intermediate,
        pdf_path,
        privacy_status,
    )?;

    Ok(canonical)
}

/// High-level API for processing LaTeX to canonical format
pub fn process_latex_to_canonical(
    latex_path: impl AsRef<Path>,
    privacy_status: Option<String>,
) -> Result<CanonicalDocument> {
    let latex_path = latex_path.as_ref();

    // Extract from LaTeX
    let intermediate = LatexExtractor::extract(latex_path)?;

    // Convert to canonical
    let canonical = CanonicalProcessor::process_latex_intermediate(
        intermediate,
        latex_path,
        privacy_status,
    )?;

    Ok(canonical)
}

/// High-level API for processing plain text to canonical format
pub fn process_txt_to_canonical(
    txt_path: impl AsRef<Path>,
    privacy_status: Option<String>,
) -> Result<CanonicalDocument> {
    let txt_path = txt_path.as_ref();

    // Extract from TXT (returns PdfIntermediate format)
    let intermediate = TxtExtractor::extract(txt_path)?;

    // Convert to canonical (reuse PDF processor since format is the same)
    let canonical = CanonicalProcessor::process_pdf_intermediate(
        intermediate,
        txt_path,
        privacy_status,
    )?;

    Ok(canonical)
}

/// High-level API for processing DOCX to canonical format
pub fn process_docx_to_canonical(
    docx_path: impl AsRef<Path>,
    privacy_status: Option<String>,
) -> Result<CanonicalDocument> {
    let docx_path = docx_path.as_ref();

    // Extract from DOCX (returns PdfIntermediate format)
    let intermediate = DocxExtractor::extract(docx_path)?;

    // Convert to canonical (reuse PDF processor since format is the same)
    let canonical = CanonicalProcessor::process_pdf_intermediate(
        intermediate,
        docx_path,
        privacy_status,
    )?;

    Ok(canonical)
}

/// Process a directory of documents to canonical JSONL
pub fn process_directory_to_jsonl(
    input_dir: impl AsRef<Path>,
    output_jsonl: impl AsRef<Path>,
    file_extension: &str, // "pdf" or "tex"
    overwrite: bool,
) -> Result<usize> {
    let input_dir = input_dir.as_ref();
    let output_jsonl = output_jsonl.as_ref();

    // Find all files with the given extension
    let files = find_files_by_extension(input_dir, file_extension)?;

    let mut documents = Vec::new();

    for file_path in &files {
        let result = match file_extension {
            "pdf" => process_pdf_to_canonical(file_path, Some("public".to_string())),
            "tex" => process_latex_to_canonical(file_path, Some("public".to_string())),
            _ => continue,
        };

        match result {
            Ok(doc) => documents.push(doc),
            Err(e) => eprintln!("Failed to process {}: {}", file_path.display(), e),
        }
    }

    // Write to JSONL
    let count = documents.len();
    CanonicalProcessor::write_to_jsonl(&documents, output_jsonl, overwrite)?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Just verify that the main types are accessible
        let _doc: Option<CanonicalDocument> = None;
        let _metadata: Option<DocumentMetadata> = None;
    }
}
