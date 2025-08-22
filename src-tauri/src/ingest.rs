use anyhow::{Context, Result};
use std::path::Path;

pub fn extract_text(file_path: &Path) -> Result<String> {
    let extension = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match extension {
        "pdf" => {
            let bytes = std::fs::read(file_path)?;
            let text = pdf_extract::extract_text_from_mem(&bytes)
                .context("Failed to extract text from PDF")?;
            Ok(text)
        }
        "md" | "txt" => {
            let text = std::fs::read_to_string(file_path)
                .context("Failed to read text file")?;
            Ok(text)
        }
        // Add more handlers for .tex, .docx etc. later
        _ => Err(anyhow::anyhow!("Unsupported file type: {}", extension)),
    }
}