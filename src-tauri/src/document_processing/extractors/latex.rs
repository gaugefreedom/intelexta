// LaTeX extraction module
// Simplified Rust implementation inspired by Python's latex_extractor.py
// Uses regex-based parsing for common LaTeX patterns

use std::path::Path;
use std::fs;
use anyhow::{Result, Context};
use regex::Regex;

use crate::document_processing::schemas::{DocumentMetadata, LatexIntermediate};

pub struct LatexExtractor;

impl LatexExtractor {
    /// Extract content and metadata from a LaTeX file
    pub fn extract(latex_path: impl AsRef<Path>) -> Result<LatexIntermediate> {
        let latex_path = latex_path.as_ref();

        // Read the LaTeX file
        let latex_content = fs::read_to_string(latex_path)
            .with_context(|| format!("Failed to read LaTeX file: {}", latex_path.display()))?;

        // Extract metadata
        let mut metadata = Self::extract_metadata(&latex_content);

        // Derive category tags from path
        let category_path_tags = Self::derive_category_tags(latex_path);
        metadata.category_path_tags = category_path_tags.clone();

        // Convert to Markdown with preserved LaTeX math
        let body_markdown_with_latex = Self::convert_to_markdown(&latex_content);

        Ok(LatexIntermediate {
            source_file_relative_path: latex_path.to_string_lossy().to_string(),
            category_path_tags,
            extracted_metadata_guess: metadata,
            body_markdown_with_latex,
            status: "auto_extracted".to_string(),
        })
    }

    /// Extract metadata from LaTeX content
    fn extract_metadata(content: &str) -> DocumentMetadata {
        let mut metadata = DocumentMetadata::default();

        // Extract title
        if let Some(title) = Self::extract_command(content, "title") {
            metadata.title = Some(title);
        }

        // Extract authors
        if let Some(authors_str) = Self::extract_command(content, "author") {
            // Split by "and" for multiple authors
            metadata.authors = authors_str
                .split(" and ")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // Extract date
        if let Some(date) = Self::extract_command(content, "date") {
            metadata.date_published = Some(date);
        }

        // Extract abstract
        if let Some(abstract_text) = Self::extract_environment(content, "abstract") {
            metadata.abstract_text = Some(abstract_text);
        }

        metadata
    }

    /// Extract a LaTeX command like \title{...}
    fn extract_command(content: &str, command: &str) -> Option<String> {
        let pattern = format!(r"\\{}\{{([^}}]+)\}}", regex::escape(command));
        let re = Regex::new(&pattern).ok()?;
        re.captures(content)
            .and_then(|cap| cap.get(1))
            .map(|m| Self::clean_latex_text(m.as_str()))
    }

    /// Extract a LaTeX environment like \begin{abstract}...\end{abstract}
    fn extract_environment(content: &str, env: &str) -> Option<String> {
        let pattern = format!(
            r"\\begin\{{{}\}}(.*?)\\end\{{{}}}",
            regex::escape(env),
            regex::escape(env)
        );
        let re = Regex::new(&pattern).ok()?;
        re.captures(content)
            .and_then(|cap| cap.get(1))
            .map(|m| Self::clean_latex_text(m.as_str()))
    }

    /// Convert LaTeX to Markdown with preserved math
    fn convert_to_markdown(content: &str) -> String {
        let mut result = content.to_string();

        // Extract document body (between \begin{document} and \end{document})
        if let Some(body) = Self::extract_environment(&result, "document") {
            result = body;
        }

        // Convert sections to Markdown headings
        result = Self::convert_sections(&result);

        // Convert text formatting
        result = Self::convert_formatting(&result);

        // Handle citations and references (preserve them)
        result = Self::preserve_citations(&result);

        // Remove common LaTeX commands that don't need conversion
        result = Self::remove_noise_commands(&result);

        // Clean up excessive whitespace
        result = Self::clean_whitespace(&result);

        result
    }

    /// Convert LaTeX sections to Markdown headings
    fn convert_sections(text: &str) -> String {
        let conversions = vec![
            (r"\\section\*?\{([^}]+)\}", "## $1"),
            (r"\\subsection\*?\{([^}]+)\}", "### $1"),
            (r"\\subsubsection\*?\{([^}]+)\}", "#### $1"),
            (r"\\paragraph\{([^}]+)\}", "##### $1"),
            (r"\\subparagraph\{([^}]+)\}", "###### $1"),
        ];

        let mut result = text.to_string();
        for (pattern, replacement) in conversions {
            if let Ok(re) = Regex::new(pattern) {
                result = re.replace_all(&result, replacement).to_string();
            }
        }
        result
    }

    /// Convert LaTeX text formatting to Markdown
    fn convert_formatting(text: &str) -> String {
        let conversions = vec![
            (r"\\textbf\{([^}]+)\}", "**$1**"),
            (r"\\textit\{([^}]+)\}", "*$1*"),
            (r"\\emph\{([^}]+)\}", "*$1*"),
            (r"\\texttt\{([^}]+)\}", "`$1`"),
        ];

        let mut result = text.to_string();
        for (pattern, replacement) in conversions {
            if let Ok(re) = Regex::new(pattern) {
                result = re.replace_all(&result, replacement).to_string();
            }
        }
        result
    }

    /// Preserve citations and references in LaTeX format
    fn preserve_citations(text: &str) -> String {
        // Citations are already in LaTeX format, just ensure they're preserved
        // \cite{key}, \ref{label}, etc. stay as-is
        text.to_string()
    }

    /// Remove noise LaTeX commands
    fn remove_noise_commands(text: &str) -> String {
        let commands_to_remove = vec![
            r"\\maketitle",
            r"\\tableofcontents",
            r"\\newpage",
            r"\\clearpage",
            r"\\noindent",
            r"\\centering",
            r"\\vspace\{[^}]+\}",
            r"\\hspace\{[^}]+\}",
            r"\\smallskip",
            r"\\medskip",
            r"\\bigskip",
        ];

        let mut result = text.to_string();
        for pattern in commands_to_remove {
            if let Ok(re) = Regex::new(pattern) {
                result = re.replace_all(&result, "").to_string();
            }
        }
        result
    }

    /// Clean LaTeX text (remove extra braces, commands)
    fn clean_latex_text(text: &str) -> String {
        let mut cleaned = text.to_string();

        // Remove simple formatting commands and keep their content
        let simple_commands = vec![
            (r"\\textbf\{([^}]+)\}", "$1"),
            (r"\\textit\{([^}]+)\}", "$1"),
            (r"\\emph\{([^}]+)\}", "$1"),
        ];

        for (pattern, replacement) in simple_commands {
            if let Ok(re) = Regex::new(pattern) {
                cleaned = re.replace_all(&cleaned, replacement).to_string();
            }
        }

        // Remove comments
        if let Ok(re) = Regex::new(r"%.*") {
            cleaned = re.replace_all(&cleaned, "").to_string();
        }

        cleaned.trim().to_string()
    }

    /// Clean excessive whitespace
    fn clean_whitespace(text: &str) -> String {
        let mut cleaned = text.to_string();

        // Remove excessive newlines
        if let Ok(re) = Regex::new(r"\n{3,}") {
            cleaned = re.replace_all(&cleaned, "\n\n").to_string();
        }

        // Trim lines
        cleaned = cleaned
            .lines()
            .map(|line| line.trim_end())
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_command() {
        let content = r"\title{Test Document Title}";
        let title = LatexExtractor::extract_command(content, "title");
        assert_eq!(title, Some("Test Document Title".to_string()));
    }

    #[test]
    fn test_extract_environment() {
        let content = r"\begin{abstract}This is an abstract.\end{abstract}";
        let abstract_text = LatexExtractor::extract_environment(content, "abstract");
        assert_eq!(abstract_text, Some("This is an abstract.".to_string()));
    }

    #[test]
    fn test_convert_sections() {
        let input = r"\section{Introduction}";
        let output = LatexExtractor::convert_sections(input);
        assert!(output.contains("## Introduction"));
    }

    #[test]
    fn test_convert_formatting() {
        let input = r"\textbf{bold} and \textit{italic}";
        let output = LatexExtractor::convert_formatting(input);
        assert!(output.contains("**bold**"));
        assert!(output.contains("*italic*"));
    }
}
