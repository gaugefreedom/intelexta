// DOCX (and DOC via fallback) extractor
use crate::document_processing::schemas::{DocumentMetadata, PdfIntermediate};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

pub struct DocxExtractor;

impl DocxExtractor {
    /// Extract text from a DOCX file
    ///
    /// DOCX files are ZIP archives containing XML files.
    /// The main content is in word/document.xml
    pub fn extract(docx_path: impl AsRef<Path>) -> Result<PdfIntermediate> {
        let docx_path = docx_path.as_ref();

        // Open the DOCX file as a ZIP archive
        let file = File::open(docx_path)
            .with_context(|| format!("Failed to open DOCX file: {}", docx_path.display()))?;

        let mut archive = ZipArchive::new(file)
            .with_context(|| format!("Failed to read DOCX as ZIP: {}", docx_path.display()))?;

        // Extract text from document.xml
        let content = if let Ok(mut document_xml) = archive.by_name("word/document.xml") {
            let mut xml_content = String::new();
            document_xml.read_to_string(&mut xml_content)?;

            // Simple XML parsing to extract text from <w:t> tags
            Self::extract_text_from_xml(&xml_content)
        } else {
            return Err(anyhow::anyhow!(
                "Invalid DOCX file: word/document.xml not found"
            ));
        };

        // Try to extract metadata from core.xml if available
        let mut metadata = DocumentMetadata {
            title: None,
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

        if let Ok(mut core_xml) = archive.by_name("docProps/core.xml") {
            let mut xml_content = String::new();
            core_xml.read_to_string(&mut xml_content)?;
            metadata = Self::extract_metadata_from_core_xml(&xml_content);
        }

        // Fallback: use filename as title if no metadata
        if metadata.title.is_none() {
            let file_name = docx_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            metadata.title = Some(file_name);
        }

        // Get relative path
        let relative_path = docx_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.docx")
            .to_string();

        Ok(PdfIntermediate {
            source_file_relative_path: relative_path,
            category_path_tags: vec![],
            extracted_metadata_guess: metadata,
            auto_cleaned_text: content,
            status: "auto_extracted".to_string(),
        })
    }

    /// Extract text content from Office Open XML
    ///
    /// Looks for <w:t>text</w:t> tags and extracts their content
    fn extract_text_from_xml(xml: &str) -> String {
        let mut result = String::new();
        let mut in_text_tag = false;
        let mut current_text = String::new();

        // Simple state machine to extract text from <w:t> tags
        for line in xml.lines() {
            // Look for <w:t> tags
            let mut chars = line.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '<' {
                    // Check if this is a text tag
                    let rest: String = chars.clone().collect();
                    if rest.starts_with("w:t") || rest.starts_with("w:t>") {
                        in_text_tag = true;
                        // Skip to after the >
                        while let Some(c) = chars.next() {
                            if c == '>' {
                                break;
                            }
                        }
                    } else if rest.starts_with("/w:t>") {
                        in_text_tag = false;
                        if !current_text.is_empty() {
                            result.push_str(&current_text);
                            current_text.clear();
                        }
                        // Skip to after the >
                        while let Some(c) = chars.next() {
                            if c == '>' {
                                break;
                            }
                        }
                    } else {
                        // Some other tag, skip it
                        while let Some(c) = chars.next() {
                            if c == '>' {
                                break;
                            }
                        }
                    }
                } else if in_text_tag {
                    current_text.push(ch);
                }
            }

            // Add newline for paragraph breaks
            if !current_text.is_empty() {
                result.push(' ');
            }
        }

        // Basic cleanup
        result
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    }

    /// Extract metadata from core.xml (Dublin Core metadata)
    fn extract_metadata_from_core_xml(xml: &str) -> DocumentMetadata {
        let mut metadata = DocumentMetadata {
            title: None,
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

        // Extract title from <dc:title>
        if let Some(title) = Self::extract_xml_tag_content(xml, "dc:title") {
            metadata.title = Some(title);
        }

        // Extract creator (author) from <dc:creator>
        if let Some(author) = Self::extract_xml_tag_content(xml, "dc:creator") {
            metadata.authors = vec![author];
        }

        // Extract keywords from <cp:keywords>
        if let Some(keywords) = Self::extract_xml_tag_content(xml, "cp:keywords") {
            metadata.keywords_from_source = keywords.split(',').map(|s| s.trim().to_string()).collect();
        }

        metadata
    }

    /// Simple helper to extract content between XML tags
    fn extract_xml_tag_content(xml: &str, tag_name: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag_name);
        let end_tag = format!("</{}>", tag_name);

        if let Some(start) = xml.find(&start_tag) {
            let content_start = start + start_tag.len();
            if let Some(end) = xml[content_start..].find(&end_tag) {
                let content = &xml[content_start..content_start + end];
                return Some(content.trim().to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_from_simple_xml() {
        let xml = r#"
            <w:p>
                <w:r>
                    <w:t>Hello</w:t>
                </w:r>
                <w:r>
                    <w:t>World</w:t>
                </w:r>
            </w:p>
        "#;

        let text = DocxExtractor::extract_text_from_xml(xml);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn test_extract_metadata() {
        let xml = r#"
            <dc:title>Test Document</dc:title>
            <dc:creator>John Doe</dc:creator>
            <cp:keywords>test, document, sample</cp:keywords>
        "#;

        let metadata = DocxExtractor::extract_metadata_from_core_xml(xml);
        assert_eq!(metadata.title, Some("Test Document".to_string()));
        assert_eq!(metadata.authors, vec!["John Doe".to_string()]);
        assert!(!metadata.keywords_from_source.is_empty());
    }
}
