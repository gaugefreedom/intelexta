// Example: Process documents to canonical JSONL format
//
// This example demonstrates how to use the document processing module
// to convert PDFs and LaTeX files to the canonical format.
//
// Usage:
//   cargo run --example process_documents

use anyhow::Result;
use std::path::Path;

use intelexta::document_processing::{
    process_pdf_to_canonical,
    process_latex_to_canonical,
    process_directory_to_jsonl,
    CanonicalProcessor,
};

fn main() -> Result<()> {
    println!("Document Processing Example\n");

    // Example 1: Process a single PDF
    println!("=== Example 1: Single PDF ===");
    example_single_pdf()?;

    // Example 2: Process a single LaTeX file
    println!("\n=== Example 2: Single LaTeX ===");
    example_single_latex()?;

    // Example 3: Process a directory of PDFs
    println!("\n=== Example 3: Batch Processing ===");
    example_batch_processing()?;

    // Example 4: Read and process JSONL
    println!("\n=== Example 4: JSONL Processing ===");
    example_jsonl_processing()?;

    println!("\n✓ All examples completed successfully!");
    Ok(())
}

fn example_single_pdf() -> Result<()> {
    let pdf_path = "data/papers/example.pdf";

    if !Path::new(pdf_path).exists() {
        println!("⚠ PDF file not found: {}", pdf_path);
        println!("  Skipping this example. Create the file to test.");
        return Ok(());
    }

    let canonical = process_pdf_to_canonical(pdf_path, Some("public".to_string()))?;

    println!("Document ID: {}", canonical.document_id);
    println!("Title: {}", canonical.metadata.title.unwrap_or_else(|| "N/A".to_string()));
    println!("Source: {}", canonical.source_file_relative_path);
    println!("Format: {}", canonical.original_format);
    println!("Text length: {} chars", canonical.cleaned_text_with_markdown_structure.len());

    Ok(())
}

fn example_single_latex() -> Result<()> {
    let latex_path = "data/latex/example.tex";

    if !Path::new(latex_path).exists() {
        println!("⚠ LaTeX file not found: {}", latex_path);
        println!("  Skipping this example. Create the file to test.");
        return Ok(());
    }

    let canonical = process_latex_to_canonical(latex_path, Some("public".to_string()))?;

    println!("Document ID: {}", canonical.document_id);
    println!("Title: {}", canonical.metadata.title.unwrap_or_else(|| "N/A".to_string()));
    println!("Authors: {}", canonical.metadata.authors.join(", "));
    println!("Abstract length: {} chars",
        canonical.metadata.abstract_text.as_ref().map(|s| s.len()).unwrap_or(0));

    Ok(())
}

fn example_batch_processing() -> Result<()> {
    let input_dir = "data/papers";
    let output_jsonl = "data/processed/canonical_corpus.jsonl";

    if !Path::new(input_dir).exists() {
        println!("⚠ Input directory not found: {}", input_dir);
        println!("  Skipping this example. Create the directory with PDF files to test.");
        return Ok(());
    }

    let count = process_directory_to_jsonl(
        input_dir,
        output_jsonl,
        "pdf",
        true  // overwrite
    )?;

    println!("Processed {} PDF documents", count);
    println!("Output: {}", output_jsonl);

    Ok(())
}

fn example_jsonl_processing() -> Result<()> {
    let jsonl_path = "data/processed/canonical_corpus.jsonl";

    if !Path::new(jsonl_path).exists() {
        println!("⚠ JSONL file not found: {}", jsonl_path);
        println!("  Skipping this example. Run batch processing first.");
        return Ok(());
    }

    // Read documents
    let documents = CanonicalProcessor::read_from_jsonl(jsonl_path)?;
    println!("Loaded {} documents", documents.len());

    // Deduplicate
    let deduplicated = CanonicalProcessor::deduplicate(documents);
    println!("After deduplication: {} documents", deduplicated.len());

    // Prepare DAPT corpus
    let dapt_output = "data/processed/dapt_corpus.txt";
    CanonicalProcessor::prepare_dapt_corpus(&deduplicated, dapt_output)?;
    println!("DAPT corpus written to: {}", dapt_output);

    Ok(())
}
