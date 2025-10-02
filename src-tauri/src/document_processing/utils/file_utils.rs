// File utilities for document processing

use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use walkdir::WalkDir;

/// Find all files with a specific extension in a directory tree
pub fn find_files_by_extension(
    base_dir: impl AsRef<Path>,
    extension: &str,
) -> Result<Vec<PathBuf>> {
    let base_dir = base_dir.as_ref();
    let mut files = Vec::new();

    for entry in WalkDir::new(base_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == extension.to_lowercase() {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}

/// Get relative path from a base directory
pub fn get_relative_path(
    file_path: impl AsRef<Path>,
    base_dir: impl AsRef<Path>,
) -> Result<PathBuf> {
    let file_path = file_path.as_ref();
    let base_dir = base_dir.as_ref();

    file_path
        .strip_prefix(base_dir)
        .map(|p| p.to_path_buf())
        .with_context(|| {
            format!(
                "Failed to get relative path: {} is not relative to {}",
                file_path.display(),
                base_dir.display()
            )
        })
}

/// Create directory if it doesn't exist
pub fn ensure_dir_exists(dir: impl AsRef<Path>) -> Result<()> {
    let dir = dir.as_ref();
    if !dir.exists() {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
    }
    Ok(())
}

/// Save JSON to file
pub fn save_json<T: serde::Serialize>(
    data: &T,
    output_path: impl AsRef<Path>,
    pretty: bool,
) -> Result<()> {
    let output_path = output_path.as_ref();

    // Create parent directory if needed
    if let Some(parent) = output_path.parent() {
        ensure_dir_exists(parent)?;
    }

    let json = if pretty {
        serde_json::to_string_pretty(data)?
    } else {
        serde_json::to_string(data)?
    };

    fs::write(output_path, json)
        .with_context(|| format!("Failed to write JSON file: {}", output_path.display()))?;

    Ok(())
}

/// Load JSON from file
pub fn load_json<T: serde::de::DeserializeOwned>(input_path: impl AsRef<Path>) -> Result<T> {
    let input_path = input_path.as_ref();
    let content = fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read JSON file: {}", input_path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON file: {}", input_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;

    #[test]
    fn test_find_files_by_extension() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create test files
        File::create(base.join("test1.pdf")).unwrap();
        File::create(base.join("test2.pdf")).unwrap();
        File::create(base.join("test.txt")).unwrap();

        let pdf_files = find_files_by_extension(base, "pdf").unwrap();
        assert_eq!(pdf_files.len(), 2);
    }

    #[test]
    fn test_get_relative_path() {
        let base = Path::new("/data/raw");
        let file = Path::new("/data/raw/papers/test.pdf");

        let relative = get_relative_path(file, base).unwrap();
        assert_eq!(relative, PathBuf::from("papers/test.pdf"));
    }

    #[test]
    fn test_save_and_load_json() {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            name: String,
            value: i32,
        }

        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("test.json");

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        // Save
        save_json(&data, &json_path, true).unwrap();

        // Load
        let loaded: TestData = load_json(&json_path).unwrap();
        assert_eq!(loaded, data);
    }
}
