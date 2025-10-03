// Plain text extractor
use crate::document_processing::schemas::{DocumentMetadata, PdfIntermediate};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct TxtExtractor;

impl TxtExtractor {
    /// Extract text from a plain text file
    ///
    /// Returns a PdfIntermediate structure (reusing the same format as PDF
    /// since plain text is similar - just cleaned text content)
    pub fn extract(txt_path: impl AsRef<Path>) -> Result<PdfIntermediate> {
        let txt_path = txt_path.as_ref();

        // Read the entire file as text
        let content = fs::read_to_string(txt_path)
            .with_context(|| format!("Failed to read text file: {}", txt_path.display()))?;

        // Basic metadata from filename
        let file_name = txt_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let metadata = DocumentMetadata {
            title: Some(file_name.clone()),
            authors: Vec::new(),
            date_published: None,
            date_accessed_utc: None,
            abstract_text: None,
            keywords_from_source: Vec::new(),
            category_path_tags: Vec::new(),
            domain_tags_ml: Vec::new(),
            journal_ref: None,
            book_title: None,
            publisher: None,
            doi: None,
            arxiv_id: None,
            email_subject: None,
            email_sender_display: None,
            email_recipients_display: Vec::new(),
        };

        // Get relative path (just filename if no parent)
        let relative_path = txt_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.txt")
            .to_string();

        Ok(PdfIntermediate {
            source_file_relative_path: relative_path,
            category_path_tags: vec![],
            extracted_metadata_guess: metadata,
            auto_cleaned_text: content,
            status: "auto_extracted".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_extract_simple_text() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "Hello, world!")?;
        writeln!(temp_file, "This is a test document.")?;
        writeln!(temp_file, "It has multiple lines.")?;

        let result = TxtExtractor::extract(temp_file.path())?;

        assert!(result.auto_cleaned_text.contains("Hello, world!"));
        assert!(result.auto_cleaned_text.contains("This is a test document."));
        assert_eq!(result.status, "auto_extracted");

        Ok(())
    }
}
