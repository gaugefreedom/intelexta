// src-tauri/src/attachments.rs
//!
//! Attachment Store: Content-addressable storage for full checkpoint outputs
//!
//! This module provides persistent storage for the full, untruncated outputs
//! of checkpoints. Outputs are stored as content-addressed files using their
//! SHA256 hash, enabling deduplication and efficient retrieval.
//!
//! Storage Structure:
//! ```
//! attachments/
//!   ab/
//!     ab1234...full_hash.txt
//!   cd/
//!     cd5678...full_hash.txt
//! ```
//!
//! The two-character prefix directory helps avoid filesystem limitations
//! on the number of files in a single directory.

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Content-addressable storage for checkpoint outputs
pub struct AttachmentStore {
    base_path: PathBuf,
}

impl AttachmentStore {
    /// Create a new attachment store at the given base path
    pub fn new(base_path: PathBuf) -> Result<Self> {
        // Ensure the base directory exists
        fs::create_dir_all(&base_path)
            .with_context(|| format!("Failed to create attachment store at {:?}", base_path))?;

        Ok(AttachmentStore { base_path })
    }

    /// Save a full output and return its SHA256 hash
    pub fn save_full_output(&self, content: &str) -> Result<String> {
        // Compute SHA256 hash of the content
        let hash = self.compute_hash(content);

        // Get the file path (hash[0..2]/hash.txt)
        let file_path = self.hash_to_path(&hash);

        // Create parent directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        // Only write if file doesn't already exist (deduplication)
        if !file_path.exists() {
            fs::write(&file_path, content).with_context(|| {
                format!("Failed to write attachment to {:?}", file_path)
            })?;
        }

        Ok(hash)
    }

    /// Load a full output by its SHA256 hash
    pub fn load_full_output(&self, hash: &str) -> Result<String> {
        let file_path = self.hash_to_path(hash);

        if !file_path.exists() {
            return Err(anyhow!(
                "Attachment not found: {} at {:?}",
                hash,
                file_path
            ));
        }

        fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read attachment from {:?}", file_path))
    }

    /// Check if an attachment exists for the given hash
    pub fn exists(&self, hash: &str) -> bool {
        self.hash_to_path(hash).exists()
    }

    /// Get the file path for a given hash
    fn hash_to_path(&self, hash: &str) -> PathBuf {
        // Use first 2 characters as subdirectory to avoid too many files in one dir
        let prefix = &hash[0..2.min(hash.len())];
        self.base_path
            .join(prefix)
            .join(format!("{}.txt", hash))
    }

    /// Compute SHA256 hash of content
    fn compute_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Get the total size of all attachments in bytes
    pub fn total_size(&self) -> Result<u64> {
        let mut total = 0u64;

        if !self.base_path.exists() {
            return Ok(0);
        }

        for entry in walkdir::WalkDir::new(&self.base_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total += entry.metadata()?.len();
            }
        }

        Ok(total)
    }

    /// Count the number of attachments
    pub fn count(&self) -> Result<usize> {
        let mut count = 0;

        if !self.base_path.exists() {
            return Ok(0);
        }

        for entry in walkdir::WalkDir::new(&self.base_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Delete an attachment by hash (use with caution!)
    pub fn delete(&self, hash: &str) -> Result<()> {
        let file_path = self.hash_to_path(hash);

        if file_path.exists() {
            fs::remove_file(&file_path)
                .with_context(|| format!("Failed to delete attachment {:?}", file_path))?;
        }

        Ok(())
    }

    /// Get the base path of the attachment store
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

/// Global attachment store instance
use once_cell::sync::OnceCell;
static GLOBAL_ATTACHMENT_STORE: OnceCell<AttachmentStore> = OnceCell::new();

/// Initialize the global attachment store
pub fn init_global_attachment_store(app_data_dir: &Path) -> Result<()> {
    let attachments_path = app_data_dir.join("attachments");
    let store = AttachmentStore::new(attachments_path)?;

    GLOBAL_ATTACHMENT_STORE
        .set(store)
        .map_err(|_| anyhow!("Global attachment store already initialized"))?;

    Ok(())
}

/// Get the global attachment store (must be initialized first)
pub fn get_global_attachment_store() -> &'static AttachmentStore {
    GLOBAL_ATTACHMENT_STORE
        .get()
        .expect("Attachment store not initialized - call init_global_attachment_store() first")
}

/// Try to get the global attachment store, or None if not initialized
pub fn try_get_global_attachment_store() -> Option<&'static AttachmentStore> {
    GLOBAL_ATTACHMENT_STORE.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let store = AttachmentStore::new(temp_dir.path().to_path_buf()).unwrap();

        let content = "This is a test output from a checkpoint.";
        let hash = store.save_full_output(content).unwrap();

        // Hash should be deterministic
        assert_eq!(hash.len(), 64); // SHA256 hex = 64 chars

        // Should be able to load it back
        let loaded = store.load_full_output(&hash).unwrap();
        assert_eq!(loaded, content);
    }

    #[test]
    fn test_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let store = AttachmentStore::new(temp_dir.path().to_path_buf()).unwrap();

        let content = "Same content";

        let hash1 = store.save_full_output(content).unwrap();
        let hash2 = store.save_full_output(content).unwrap();

        // Same content should produce same hash
        assert_eq!(hash1, hash2);

        // Should only have one file
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn test_different_content() {
        let temp_dir = TempDir::new().unwrap();
        let store = AttachmentStore::new(temp_dir.path().to_path_buf()).unwrap();

        let hash1 = store.save_full_output("Content 1").unwrap();
        let hash2 = store.save_full_output("Content 2").unwrap();

        // Different content should produce different hashes
        assert_ne!(hash1, hash2);

        // Should have two files
        assert_eq!(store.count().unwrap(), 2);

        // Both should be loadable
        assert_eq!(store.load_full_output(&hash1).unwrap(), "Content 1");
        assert_eq!(store.load_full_output(&hash2).unwrap(), "Content 2");
    }

    #[test]
    fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let store = AttachmentStore::new(temp_dir.path().to_path_buf()).unwrap();

        let hash = store.save_full_output("Test content").unwrap();

        assert!(store.exists(&hash));
        assert!(!store.exists("nonexistent_hash"));
    }

    #[test]
    fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = AttachmentStore::new(temp_dir.path().to_path_buf()).unwrap();

        let hash = store.save_full_output("Test content").unwrap();
        assert!(store.exists(&hash));

        store.delete(&hash).unwrap();
        assert!(!store.exists(&hash));
    }

    #[test]
    fn test_total_size() {
        let temp_dir = TempDir::new().unwrap();
        let store = AttachmentStore::new(temp_dir.path().to_path_buf()).unwrap();

        let content1 = "Short";
        let content2 = "A much longer piece of content for testing";

        store.save_full_output(content1).unwrap();
        store.save_full_output(content2).unwrap();

        let total = store.total_size().unwrap();
        assert_eq!(total, (content1.len() + content2.len()) as u64);
    }

    #[test]
    fn test_hash_computation() {
        let temp_dir = TempDir::new().unwrap();
        let store = AttachmentStore::new(temp_dir.path().to_path_buf()).unwrap();

        // Known test vector
        let content = "hello world";
        let hash = store.compute_hash(content);

        // SHA256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
