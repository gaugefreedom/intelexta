# How to Contribute to Intelexta

We're excited you're here to help build a new foundation for trustworthy AI work.

## Development Setup

1.  Ensure you have Rust and Node.js installed.
2.  Follow the steps in the `README.md` to get the application running.
3.  On some Linux systems using Wayland, the app may fail to launch due to graphics permissions. The current workaround is to run the backend with:
    ```bash
    LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 WINIT_UNIX_BACKEND=x11 GDK_BACKEND=x11 cargo tauri dev
    ```

## Pull Request Process

1.  Fork the repository and create your branch from `main`.
2.  Make sure your code lints and any new features have tests.
3.  Submit your pull request with a clear description of the changes.
4.  Update relevant documentation (README.md, CHANGELOG.md, etc.)

## Working with Document Processing

The document processing module (`src-tauri/src/document_processing/`) is designed to be extensible. Here's how to add support for a new document format.

### Adding a New Document Format

#### Step 1: Create the Extractor

Create a new file in `src-tauri/src/document_processing/extractors/`:

```rust
// src-tauri/src/document_processing/extractors/myformat.rs
use crate::document_processing::schemas::{DocumentMetadata, PdfIntermediate};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct MyFormatExtractor;

impl MyFormatExtractor {
    /// Extract content from MyFormat file
    pub fn extract(path: impl AsRef<Path>) -> Result<PdfIntermediate> {
        let path = path.as_ref();

        // Read and parse your format
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Extract metadata (customize based on your format)
        let metadata = DocumentMetadata {
            title: Some("Extracted Title".to_string()),
            authors: vec!["Author Name".to_string()],
            // ... fill in other fields
            ..Default::default()
        };

        // Get relative path
        let relative_path = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.myformat")
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

    #[test]
    fn test_extraction() {
        // Add your tests here
    }
}
```

#### Step 2: Export the Extractor

Update `src-tauri/src/document_processing/extractors/mod.rs`:

```rust
pub mod myformat;
pub use myformat::MyFormatExtractor;
```

#### Step 3: Add High-Level API

Update `src-tauri/src/document_processing/mod.rs`:

```rust
// Add to re-exports
pub use extractors::MyFormatExtractor;

// Add processing function
pub fn process_myformat_to_canonical(
    path: impl AsRef<Path>,
    privacy_status: Option<String>,
) -> Result<CanonicalDocument> {
    let path = path.as_ref();

    // Extract from MyFormat
    let intermediate = MyFormatExtractor::extract(path)?;

    // Convert to canonical (reuse PDF processor for simple formats)
    let canonical = CanonicalProcessor::process_pdf_intermediate(
        intermediate,
        path,
        privacy_status,
    )?;

    Ok(canonical)
}
```

#### Step 4: Update the Orchestrator

Update the match statement in `src-tauri/src/orchestrator.rs` (around line 1797):

```rust
let canonical_doc = match ingestion_config.format.to_lowercase().as_str() {
    "pdf" => { /* existing */ }
    "tex" | "latex" => { /* existing */ }
    "txt" => { /* existing */ }
    "docx" | "doc" => { /* existing */ }
    "myformat" => {
        document_processing::process_myformat_to_canonical(
            &ingestion_config.source_path,
            Some(ingestion_config.privacy_status.clone())
        )?
    }
    unsupported => {
        return Err(anyhow!(
            "Unsupported document format: {}. Supported formats: pdf, latex, txt, docx, myformat",
            unsupported
        ));
    }
};
```

#### Step 5: Update the UI

Update `app/src/components/CheckpointEditor.tsx`:

```tsx
// Find the format dropdown (around line 352)
<select value={format} onChange={(event) => setFormat(event.target.value)}>
  <option value="pdf">PDF</option>
  <option value="latex">LaTeX</option>
  <option value="txt">TXT</option>
  <option value="docx">DOCX</option>
  <option value="myformat">My Format</option>  {/* Add this */}
</select>

// Update file picker filters (around line 176)
filters: [
  { name: 'Documents', extensions: ['pdf', 'tex', 'latex', 'docx', 'txt', 'myformat'] },
  // ... other filters
]
```

#### Step 6: Add Tests

Create comprehensive tests for your extractor:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_myformat_extraction() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "Test content")?;

        let result = MyFormatExtractor::extract(temp_file.path())?;

        assert!(result.auto_cleaned_text.contains("Test content"));
        assert_eq!(result.status, "auto_extracted");

        Ok(())
    }
}
```

#### Step 7: Update Documentation

1. Add your format to `CHANGELOG.md`
2. Update format table in `src-tauri/src/document_processing/README.md`
3. Add format-specific notes if needed

### Schema Consistency Guidelines

When creating extractors, follow these guidelines:

1. **Return Type**: Return either `PdfIntermediate` or `LatexIntermediate`
   - Use `PdfIntermediate` for simple text-based formats (PDF, TXT, DOCX, RTF)
   - Use `LatexIntermediate` for structured formats (LaTeX, HTML, Markdown)

2. **DocumentMetadata**: Always populate as many fields as possible
   - Use `Vec::new()` for empty vector fields (not `None`)
   - Use `None` for truly unknown optional fields
   - Use `..Default::default()` to fill remaining fields

3. **Error Handling**: Use `anyhow::Context` for informative errors
   ```rust
   .with_context(|| format!("Failed to parse file: {}", path.display()))?
   ```

4. **Testing**: Always include unit tests with actual file samples

### Code Style

- Follow Rust standard formatting: `cargo fmt`
- Check for linting issues: `cargo clippy`
- Ensure all tests pass: `cargo test`
- Frontend: Follow existing TypeScript patterns

## Testing

### Backend Tests
```bash
cd src-tauri
cargo test
```

### Frontend Tests
```bash
cd app
npm test
```

### Integration Testing
Manual integration testing checklist:
1. Create a project
2. Add your new format step
3. Browse for a test file
4. Execute the workflow
5. Verify canonical JSON in Inspector
6. Export as CAR
7. Verify signature

## Documentation Updates

When making significant changes, update:
- `README.md` - User-facing features
- `CHANGELOG.md` - Version history
- `docs/DOCUMENT_PROCESSING_INTEGRATION.md` - Integration details
- Module-specific READMEs - Technical documentation

## Questions?

- Check existing issues on GitHub
- Review `docs/PROJECT_CONTEXT.md` for strategic context
- Ask in pull request comments
