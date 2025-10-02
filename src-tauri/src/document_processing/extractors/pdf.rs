// PDF extraction module
// Rust implementation inspired by Python's pdf_extractor.py

use std::path::Path;
use anyhow::{Result, Context};
use pdf_extract::extract_text;

use crate::document_processing::schemas::{DocumentMetadata, PdfIntermediate};

pub struct PdfExtractor;

impl PdfExtractor {
    /// Extract text and metadata from a PDF file
    pub fn extract(pdf_path: impl AsRef<Path>) -> Result<PdfIntermediate> {
        let pdf_path = pdf_path.as_ref();

        // Extract text using pdf-extract crate
        let extracted_text = extract_text(pdf_path)
            .with_context(|| format!("Failed to extract text from PDF: {}", pdf_path.display()))?;

        // Auto-clean the extracted text
        let auto_cleaned_text = Self::auto_clean_text(&extracted_text);

        // Derive category tags from path
        let category_path_tags = Self::derive_category_tags(pdf_path);

        // Extract metadata (basic implementation - can be enhanced)
        let extracted_metadata_guess = Self::guess_metadata(&extracted_text, pdf_path);

        Ok(PdfIntermediate {
            source_file_relative_path: pdf_path.to_string_lossy().to_string(),
            category_path_tags,
            extracted_metadata_guess,
            auto_cleaned_text,
            status: "auto_extracted".to_string(),
        })
    }

    /// Auto-clean extracted text
    /// Applies basic cleaning rules similar to Python's pdf_cleaner.py
    fn auto_clean_text(text: &str) -> String {
        let mut cleaned = text.to_string();

        // Remove excessive whitespace
        cleaned = cleaned
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        // Remove excessive newlines (more than 2)
        let re_newlines = regex::Regex::new(r"\n{3,}").unwrap();
        cleaned = re_newlines.replace_all(&cleaned, "\n\n").to_string();

        // Remove page numbers (simple heuristic - standalone numbers)
        let re_page_nums = regex::Regex::new(r"^\d+$").unwrap();
        cleaned = cleaned
            .lines()
            .filter(|line| !re_page_nums.is_match(line.trim()))
            .collect::<Vec<_>>()
            .join("\n");

        // Remove header/footer artifacts (lines with all caps, very short)
        cleaned = cleaned
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                if trimmed.len() < 40 && trimmed.chars().all(|c| c.is_uppercase() || c.is_whitespace()) {
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        cleaned.trim().to_string()
    }

    /// Derive category tags from file path
    fn derive_category_tags(path: &Path) -> Vec<String> {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|name| name.to_str())
            .map(|s| vec![s.to_string()])
            .unwrap_or_default()
    }

    /// Guess metadata from content (basic implementation)
    fn guess_metadata(text: &str, path: &Path) -> DocumentMetadata {
        let mut metadata = DocumentMetadata::default();

        // Try to extract title (first non-empty line that looks like a title)
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        if let Some(first_line) = lines.first() {
            let first_line = first_line.trim();
            // Heuristic: if first line is not too long and doesn't start with lowercase
            if first_line.len() < 200 && !first_line.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
                metadata.title = Some(first_line.to_string());
            }
        }

        // Try to extract abstract (look for "Abstract" section)
        if let Some(abstract_start) = text.to_lowercase().find("abstract") {
            let after_abstract = &text[abstract_start..];
            if let Some(next_section) = after_abstract.find("\n\n") {
                let abstract_text = after_abstract[..next_section].trim();
                // Remove "Abstract" label
                let abstract_text = abstract_text.trim_start_matches("Abstract").trim();
                let abstract_text = abstract_text.trim_start_matches("ABSTRACT").trim();
                if !abstract_text.is_empty() && abstract_text.len() < 2000 {
                    metadata.abstract_text = Some(abstract_text.to_string());
                }
            }
        }

        // Add path-based tags
        metadata.category_path_tags = Self::derive_category_tags(path);

        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_clean_text() {
        let input = "  Title  \n\n\n\n  Content  \n\n\n123\n\n  More  ";
        let cleaned = PdfExtractor::auto_clean_text(input);
        assert!(!cleaned.contains("   "));
        assert!(!cleaned.contains("\n\n\n"));
    }

    #[test]
    fn test_derive_category_tags() {
        let path = Path::new("/data/raw/papers/physics/paper.pdf");
        let tags = PdfExtractor::derive_category_tags(path);
        assert_eq!(tags, vec!["physics".to_string()]);
    }
}
