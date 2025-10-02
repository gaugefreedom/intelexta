// Canonical schema for scientific document processing
// Rust implementation of the Python Pydantic models

use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Processing log for tracking extraction and cleaning steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingLog {
    pub extraction_tool: Option<String>,
    pub extraction_timestamp_utc: Option<String>,
    pub processing_timestamp_utc: String,
    #[serde(default)]
    pub cleaning_steps_applied: Vec<String>,
    pub quality_heuristic_score: Option<f64>,
}

impl ProcessingLog {
    pub fn new(extraction_tool: Option<String>) -> Self {
        Self {
            extraction_tool,
            extraction_timestamp_utc: Some(Utc::now().to_rfc3339()),
            processing_timestamp_utc: Utc::now().to_rfc3339(),
            cleaning_steps_applied: Vec::new(),
            quality_heuristic_score: None,
        }
    }

    pub fn add_cleaning_step(&mut self, step: impl Into<String>) {
        self.cleaning_steps_applied.push(step.into());
    }
}

/// Consent details for privacy tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentDetails {
    pub consent_form_id: Option<String>,
    pub consent_date: Option<String>, // YYYY-MM-DD
    pub anonymization_level: Option<String>,
}

/// Comprehensive metadata for documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    pub date_published: Option<String>, // YYYY-MM-DD, YYYY-MM, or YYYY
    pub date_accessed_utc: Option<String>, // ISO 8601
    pub abstract_text: Option<String>,
    #[serde(default)]
    pub keywords_from_source: Vec<String>,
    #[serde(default)]
    pub category_path_tags: Vec<String>, // From folder structure
    #[serde(default)]
    pub domain_tags_ml: Vec<String>, // ML-assigned or broader
    pub journal_ref: Option<String>,
    pub book_title: Option<String>,
    pub publisher: Option<String>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub email_subject: Option<String>,
    pub email_sender_display: Option<String>, // Anonymized/Pseudonymized
    #[serde(default)]
    pub email_recipients_display: Vec<String>, // Anonymized/Pseudonymized
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            title: None,
            authors: Vec::new(),
            date_published: None,
            date_accessed_utc: Some(Utc::now().to_rfc3339()),
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
        }
    }
}

/// Canonical document structure - the core schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalDocument {
    pub document_id: String, // Hash of content or unique path-based ID
    pub source_type: String, // "paper", "book_chapter", "email", "note", etc.
    pub source_path_absolute: String,
    pub source_file_relative_path: String,
    pub original_format: String, // "pdf", "latex", "text", "email_mime", etc.
    pub processing_log: ProcessingLog,
    pub privacy_status: String, // "public", "consent_obtained_anonymized", etc.
    pub consent_details: Option<ConsentDetails>,
    pub metadata: DocumentMetadata,
    pub cleaned_text_with_markdown_structure: String, // Full text, Markdown for structure, LaTeX for math
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
}

fn default_language() -> String {
    "en".to_string()
}

fn default_schema_version() -> String {
    "1.0.0".to_string()
}

impl CanonicalDocument {
    /// Convert to JSONL string (one line JSON)
    pub fn to_jsonl_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Generate document ID from content hash
    pub fn generate_id(content: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Intermediate extraction result for PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfIntermediate {
    pub source_file_relative_path: String,
    #[serde(default)]
    pub category_path_tags: Vec<String>,
    pub extracted_metadata_guess: DocumentMetadata,
    pub auto_cleaned_text: String,
    pub status: String, // "auto_extracted", "human_llm_refined", etc.
}

/// Intermediate extraction result for LaTeX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatexIntermediate {
    pub source_file_relative_path: String,
    #[serde(default)]
    pub category_path_tags: Vec<String>,
    pub extracted_metadata_guess: DocumentMetadata,
    pub body_markdown_with_latex: String,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_document_serialization() {
        let doc = CanonicalDocument {
            document_id: "test123".to_string(),
            source_type: "paper".to_string(),
            source_path_absolute: "/path/to/paper.pdf".to_string(),
            source_file_relative_path: "papers/paper.pdf".to_string(),
            original_format: "pdf".to_string(),
            processing_log: ProcessingLog::new(Some("pdf-extract".to_string())),
            privacy_status: "public".to_string(),
            consent_details: None,
            metadata: DocumentMetadata::default(),
            cleaned_text_with_markdown_structure: "# Test\n\nContent".to_string(),
            language: "en".to_string(),
            schema_version: "1.0.0".to_string(),
        };

        let json = doc.to_jsonl_string().unwrap();
        assert!(json.contains("test123"));
        assert!(!json.contains('\n')); // Should be single line
    }

    #[test]
    fn test_generate_id() {
        let id1 = CanonicalDocument::generate_id("test content");
        let id2 = CanonicalDocument::generate_id("test content");
        let id3 = CanonicalDocument::generate_id("different content");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }
}
