// Canonical processor - converts intermediate formats to canonical JSONL
// Rust implementation inspired by Python's structure_*_to_canonical.py scripts

use std::path::Path;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use anyhow::{Result, Context};

use crate::document_processing::schemas::{
    CanonicalDocument, PdfIntermediate, LatexIntermediate, ProcessingLog,
};

pub struct CanonicalProcessor;

impl CanonicalProcessor {
    /// Process PDF intermediate to canonical format
    pub fn process_pdf_intermediate(
        intermediate: PdfIntermediate,
        source_path_absolute: impl AsRef<Path>,
        privacy_status: Option<String>,
    ) -> Result<CanonicalDocument> {
        let source_path_absolute = source_path_absolute.as_ref();

        // Generate document ID from content
        let document_id = CanonicalDocument::generate_id(&intermediate.auto_cleaned_text);

        // Create processing log
        let mut processing_log = ProcessingLog::new(Some("pdf-extract".to_string()));
        processing_log.add_cleaning_step("auto_clean_pdf");

        Ok(CanonicalDocument {
            document_id,
            source_type: "paper".to_string(), // Default, can be customized
            source_path_absolute: source_path_absolute.to_string_lossy().to_string(),
            source_file_relative_path: intermediate.source_file_relative_path,
            original_format: "pdf".to_string(),
            processing_log,
            privacy_status: privacy_status.unwrap_or_else(|| "public".to_string()),
            consent_details: None,
            metadata: intermediate.extracted_metadata_guess,
            cleaned_text_with_markdown_structure: intermediate.auto_cleaned_text,
            language: "en".to_string(),
            schema_version: "1.0.0".to_string(),
        })
    }

    /// Process LaTeX intermediate to canonical format
    pub fn process_latex_intermediate(
        intermediate: LatexIntermediate,
        source_path_absolute: impl AsRef<Path>,
        privacy_status: Option<String>,
    ) -> Result<CanonicalDocument> {
        let source_path_absolute = source_path_absolute.as_ref();

        // Generate document ID from content
        let document_id = CanonicalDocument::generate_id(&intermediate.body_markdown_with_latex);

        // Create processing log
        let mut processing_log = ProcessingLog::new(Some("latex-extractor".to_string()));
        processing_log.add_cleaning_step("latex_to_markdown_conversion");

        Ok(CanonicalDocument {
            document_id,
            source_type: "paper".to_string(), // Default, can be customized
            source_path_absolute: source_path_absolute.to_string_lossy().to_string(),
            source_file_relative_path: intermediate.source_file_relative_path,
            original_format: "latex".to_string(),
            processing_log,
            privacy_status: privacy_status.unwrap_or_else(|| "public".to_string()),
            consent_details: None,
            metadata: intermediate.extracted_metadata_guess,
            cleaned_text_with_markdown_structure: intermediate.body_markdown_with_latex,
            language: "en".to_string(),
            schema_version: "1.0.0".to_string(),
        })
    }

    /// Write canonical document to JSONL file
    pub fn write_to_jsonl(
        documents: &[CanonicalDocument],
        output_path: impl AsRef<Path>,
        overwrite: bool,
    ) -> Result<()> {
        let output_path = output_path.as_ref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Open file for writing
        let file = if overwrite {
            File::create(output_path)?
        } else {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(output_path)?
        };

        let mut writer = BufWriter::new(file);

        // Write each document as a JSON line
        for doc in documents {
            let json_line = doc.to_jsonl_string()
                .with_context(|| format!("Failed to serialize document: {}", doc.document_id))?;
            writeln!(writer, "{}", json_line)?;
        }

        writer.flush()?;

        Ok(())
    }

    /// Read JSONL file into canonical documents
    pub fn read_from_jsonl(input_path: impl AsRef<Path>) -> Result<Vec<CanonicalDocument>> {
        let input_path = input_path.as_ref();
        let content = fs::read_to_string(input_path)
            .with_context(|| format!("Failed to read JSONL file: {}", input_path.display()))?;

        let mut documents = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let doc: CanonicalDocument = serde_json::from_str(line)
                .with_context(|| format!("Failed to parse line {} in {}", line_num + 1, input_path.display()))?;
            documents.push(doc);
        }

        Ok(documents)
    }

    /// Deduplicate canonical corpus by document ID
    pub fn deduplicate(documents: Vec<CanonicalDocument>) -> Vec<CanonicalDocument> {
        use std::collections::HashSet;

        let mut seen_ids = HashSet::new();
        let mut deduplicated = Vec::new();

        for doc in documents {
            if seen_ids.insert(doc.document_id.clone()) {
                deduplicated.push(doc);
            }
        }

        deduplicated
    }

    /// Prepare DAPT (Domain-Adaptive Pre-Training) corpus from canonical documents
    /// Extracts just the cleaned text for language model training
    pub fn prepare_dapt_corpus(
        documents: &[CanonicalDocument],
        output_path: impl AsRef<Path>,
    ) -> Result<()> {
        let output_path = output_path.as_ref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let mut file = File::create(output_path)?;
        let mut writer = BufWriter::new(&mut file);

        for doc in documents {
            // Write the cleaned text with double newlines between documents
            writeln!(writer, "{}\n", doc.cleaned_text_with_markdown_structure)?;
        }

        writer.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_and_read_jsonl() {
        let temp_dir = TempDir::new().unwrap();
        let jsonl_path = temp_dir.path().join("test.jsonl");

        // Create test documents
        let doc = CanonicalDocument {
            document_id: "test123".to_string(),
            source_type: "paper".to_string(),
            source_path_absolute: "/test/paper.pdf".to_string(),
            source_file_relative_path: "paper.pdf".to_string(),
            original_format: "pdf".to_string(),
            processing_log: ProcessingLog::new(Some("test".to_string())),
            privacy_status: "public".to_string(),
            consent_details: None,
            metadata: DocumentMetadata::default(),
            cleaned_text_with_markdown_structure: "# Test\n\nContent".to_string(),
            language: "en".to_string(),
            schema_version: "1.0.0".to_string(),
        };

        // Write
        CanonicalProcessor::write_to_jsonl(&[doc.clone()], &jsonl_path, true).unwrap();

        // Read
        let read_docs = CanonicalProcessor::read_from_jsonl(&jsonl_path).unwrap();
        assert_eq!(read_docs.len(), 1);
        assert_eq!(read_docs[0].document_id, doc.document_id);
    }

    #[test]
    fn test_deduplicate() {
        let doc1 = CanonicalDocument {
            document_id: "id1".to_string(),
            source_type: "paper".to_string(),
            source_path_absolute: "/test/paper1.pdf".to_string(),
            source_file_relative_path: "paper1.pdf".to_string(),
            original_format: "pdf".to_string(),
            processing_log: ProcessingLog::new(Some("test".to_string())),
            privacy_status: "public".to_string(),
            consent_details: None,
            metadata: DocumentMetadata::default(),
            cleaned_text_with_markdown_structure: "Content 1".to_string(),
            language: "en".to_string(),
            schema_version: "1.0.0".to_string(),
        };

        let doc2 = doc1.clone();
        let mut doc3 = doc1.clone();
        doc3.document_id = "id2".to_string();

        let deduplicated = CanonicalProcessor::deduplicate(vec![doc1, doc2, doc3]);
        assert_eq!(deduplicated.len(), 2); // id1 and id2
    }
}
