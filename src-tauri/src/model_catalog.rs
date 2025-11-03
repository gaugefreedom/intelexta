// src-tauri/src/model_catalog.rs
//!
//! Model Catalog: Verifiable, signed pricing and environmental impact data
//!
//! This module loads and verifies the `config/model_catalog.toml` file, which
//! contains authoritative pricing, energy consumption, and environmental impact
//! metrics for all available models.
//!
//! Key Features:
//! - Cryptographic signature verification (Ed25519)
//! - SHA256 hash of catalog for provenance
//! - Fallback to safe defaults if catalog is missing/invalid
//! - Support for multiple nature cost algorithms

use anyhow::{anyhow, Context, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Model definition with pricing and environmental metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDef {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_name: Option<String>,

    pub description: String,

    /// USD cost per million tokens (blended input/output)
    pub cost_per_million_tokens: f64,

    /// Nature cost per million tokens (gCO2e or other units)
    pub nature_cost_per_million_tokens: f64,

    /// Energy consumption in kWh per million tokens
    pub energy_kwh_per_million_tokens: f64,

    /// Whether this model is enabled for use
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,

    /// Context window size (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,

    /// Maximum output tokens (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    /// Whether this model requires network access
    #[serde(default)]
    pub requires_network: bool,

    /// Whether this model requires an API key
    #[serde(default)]
    pub requires_api_key: bool,
}

fn default_true() -> bool {
    true
}

/// Metadata about the catalog itself
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMetadata {
    pub version: String,
    pub created_at: String,
    pub description: String,
}

/// Default settings for cost calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogDefaults {
    pub nature_cost_algorithm: String,
    pub fallback_cost_per_million_tokens: f64,
    pub fallback_nature_cost_per_million_tokens: f64,
}

/// Nature cost algorithm definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatureCostAlgorithm {
    pub formula: String,
    pub description: String,
    #[serde(flatten)]
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Provider metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub requires_network: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base_url: Option<String>,
    #[serde(default)]
    pub requires_api_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_endpoint: Option<String>,
}

/// Signature block for catalog verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogSignature {
    pub public_key: String,
    pub signature: String,
    pub signed_at: String,
}

/// The complete model catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawModelCatalog {
    pub metadata: CatalogMetadata,
    pub defaults: CatalogDefaults,
    pub nature_cost_algorithms: HashMap<String, NatureCostAlgorithm>,
    pub models: Vec<ModelDef>,
    pub providers: HashMap<String, ProviderInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<CatalogSignature>,
}

/// Verified model catalog with computed hash
#[derive(Debug, Clone)]
pub struct ModelCatalog {
    pub raw: RawModelCatalog,
    pub catalog_sha256: String,
    pub signature_verified: bool,
    models_by_id: HashMap<String, ModelDef>,
}

impl ModelCatalog {
    /// Load catalog from default location (config/model_catalog.toml)
    pub fn load_default() -> Result<Self> {
        let catalog_path = Self::default_catalog_path()?;
        Self::load_from_path(&catalog_path)
    }

    /// Load catalog from a specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        let toml_str = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read catalog from {:?}", path))?;

        Self::load_from_str(&toml_str)
    }

    /// Load catalog from TOML string
    pub fn load_from_str(toml_str: &str) -> Result<Self> {
        let raw: RawModelCatalog = toml::from_str(toml_str)
            .context("Failed to parse model catalog TOML")?;

        // Compute SHA256 hash of catalog (excluding signature block)
        let catalog_sha256 = Self::compute_catalog_hash(toml_str);

        // Verify signature if present
        let signature_verified = if let Some(ref sig_block) = raw.signature {
            Self::verify_signature(toml_str, sig_block)?
        } else {
            // No signature present - catalog is unverified
            eprintln!("⚠️  Warning: Model catalog has no signature - using unverified data");
            false
        };

        // Build models lookup map
        let models_by_id = raw
            .models
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        Ok(ModelCatalog {
            raw,
            catalog_sha256,
            signature_verified,
            models_by_id,
        })
    }

    /// Get default catalog path
    fn default_catalog_path() -> Result<PathBuf> {
        // Try multiple locations to find the catalog
        let mut candidates = Vec::new();

        // 1. Try current directory + config/model_catalog.toml
        if let Ok(cwd) = std::env::current_dir() {
            let path = cwd.join("config").join("model_catalog.toml");
            eprintln!("[model_catalog] Trying: {}", path.display());
            candidates.push(path);
        }

        // 2. Try parent directory + config/model_catalog.toml (for Tauri dev mode)
        if let Ok(cwd) = std::env::current_dir() {
            let path = cwd.join("..").join("config").join("model_catalog.toml");
            eprintln!("[model_catalog] Trying: {}", path.display());
            candidates.push(path);
        }

        // 3. Try executable directory + config/model_catalog.toml (production builds)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                let path = exe_dir.join("config").join("model_catalog.toml");
                eprintln!("[model_catalog] Trying: {}", path.display());
                candidates.push(path);
            }
        }

        // 4. Try executable directory + ../Resources/config/model_catalog.toml (macOS app bundle)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                let path = exe_dir.join("..").join("Resources").join("config").join("model_catalog.toml");
                eprintln!("[model_catalog] Trying: {}", path.display());
                candidates.push(path);
            }
        }

        // 5. Try executable directory + resources/config/model_catalog.toml (Windows/Linux bundle)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                let path = exe_dir.join("resources").join("config").join("model_catalog.toml");
                eprintln!("[model_catalog] Trying: {}", path.display());
                candidates.push(path);
            }
        }

        // 6. Try AppImage/Linux bundle: /path/to/usr/share/config/model_catalog.toml
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                // For AppImage: exe is at /tmp/.mount_XXX/usr/bin/intelexta
                // Resources are at /tmp/.mount_XXX/usr/share/...
                let path = exe_dir.join("..").join("share").join("config").join("model_catalog.toml");
                eprintln!("[model_catalog] Trying: {}", path.display());
                candidates.push(path);
            }
        }

        // 7. Try AppImage/Linux bundle: /path/to/usr/config/model_catalog.toml
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                // Another common location: /tmp/.mount_XXX/usr/config/...
                let path = exe_dir.join("..").join("config").join("model_catalog.toml");
                eprintln!("[model_catalog] Trying: {}", path.display());
                candidates.push(path);
            }
        }

        // 8. Try direct sibling to binary: ../config/model_catalog.toml
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                if let Some(parent_dir) = exe_dir.parent() {
                    let path = parent_dir.join("config").join("model_catalog.toml");
                    eprintln!("[model_catalog] Trying: {}", path.display());
                    candidates.push(path);
                }
            }
        }

        // 9. Try Tauri AppImage resource location: ../lib/ProductName/_up_/config/model_catalog.toml
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                if let Some(parent_dir) = exe_dir.parent() {
                    // Tauri bundles resources with _up_ for parent directory
                    let path = parent_dir.join("lib").join("Intelexta").join("_up_").join("config").join("model_catalog.toml");
                    eprintln!("[model_catalog] Trying: {}", path.display());
                    candidates.push(path);
                }
            }
        }

        // Return the first path that exists
        for path in &candidates {
            if path.exists() {
                eprintln!("[model_catalog] ✓ Found catalog at: {}", path.display());
                return Ok(path.clone());
            }
        }

        // If none exist, return the first candidate and let the error be handled upstream
        eprintln!("[model_catalog] ✗ Could not find model_catalog.toml in any of these locations:");
        for path in &candidates {
            eprintln!("[model_catalog]   - {}", path.display());
        }

        candidates.into_iter().next()
            .ok_or_else(|| anyhow!("Could not determine catalog path"))
    }

    /// Compute SHA256 hash of the catalog content (excluding signature block)
    fn compute_catalog_hash(toml_str: &str) -> String {
        // For simplicity, hash the full TOML (including signature if present)
        // In production, you'd want to exclude the [signature] section
        // and compute a canonical hash
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(toml_str.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verify the Ed25519 signature on the catalog
    fn verify_signature(toml_str: &str, sig_block: &CatalogSignature) -> Result<bool> {
        // Parse public key
        let public_key_bytes = hex::decode(&sig_block.public_key)
            .context("Invalid public key hex in signature block")?;
        let public_key = VerifyingKey::from_bytes(
            &public_key_bytes
                .try_into()
                .map_err(|_| anyhow!("Public key must be 32 bytes"))?,
        )
        .context("Invalid Ed25519 public key")?;

        // Parse signature
        let sig_bytes = hex::decode(&sig_block.signature)
            .context("Invalid signature hex in signature block")?;
        let signature = Signature::from_bytes(
            &sig_bytes
                .try_into()
                .map_err(|_| anyhow!("Signature must be 64 bytes"))?,
        );

        // For verification, we need the canonical content (without signature block)
        // For now, we'll use the full toml_str as a placeholder
        // TODO: Implement proper canonical form extraction
        let message = toml_str.as_bytes();

        // Verify signature
        match public_key.verify(message, &signature) {
            Ok(_) => Ok(true),
            Err(_) => {
                eprintln!("❌ Model catalog signature verification FAILED");
                Ok(false)
            }
        }
    }

    /// Get a model by ID
    pub fn get_model(&self, model_id: &str) -> Option<&ModelDef> {
        self.models_by_id.get(model_id)
    }

    /// Get all enabled models
    pub fn get_enabled_models(&self) -> Vec<&ModelDef> {
        self.raw
            .models
            .iter()
            .filter(|m| m.enabled)
            .collect()
    }

    /// Get all models for a specific provider
    pub fn get_models_by_provider(&self, provider: &str) -> Vec<&ModelDef> {
        self.raw
            .models
            .iter()
            .filter(|m| m.provider == provider)
            .collect()
    }

    /// Calculate USD cost for a given model and token count
    pub fn calculate_usd_cost(&self, model_id: &str, tokens: u64) -> f64 {
        let cost_per_million = self
            .get_model(model_id)
            .map(|m| m.cost_per_million_tokens)
            .unwrap_or(self.raw.defaults.fallback_cost_per_million_tokens);

        (tokens as f64 / 1_000_000.0) * cost_per_million
    }

    /// Calculate nature cost for a given model and token count
    pub fn calculate_nature_cost(&self, model_id: &str, tokens: u64) -> f64 {
        let nature_cost_per_million = self
            .get_model(model_id)
            .and_then(|m| {
                let value = m.nature_cost_per_million_tokens;
                if value.is_finite() && value > 0.0 {
                    Some(value)
                } else {
                    None
                }
            })
            .unwrap_or(self.raw.defaults.fallback_nature_cost_per_million_tokens);

        (tokens as f64 / 1_000_000.0) * nature_cost_per_million
    }

    /// Calculate energy consumption for a given model and token count
    pub fn calculate_energy_kwh(&self, model_id: &str, tokens: u64) -> f64 {
        let energy_per_million = self
            .get_model(model_id)
            .map(|m| m.energy_kwh_per_million_tokens)
            .unwrap_or(0.0);

        (tokens as f64 / 1_000_000.0) * energy_per_million
    }

    /// Get the nature cost algorithm definition
    pub fn get_nature_cost_algorithm(&self, algorithm_name: &str) -> Option<&NatureCostAlgorithm> {
        self.raw.nature_cost_algorithms.get(algorithm_name)
    }

    /// Get the default nature cost algorithm
    pub fn get_default_nature_cost_algorithm(&self) -> Option<&NatureCostAlgorithm> {
        let default_name = &self.raw.defaults.nature_cost_algorithm;
        self.get_nature_cost_algorithm(default_name)
    }

    /// Check if a model requires network access
    pub fn requires_network(&self, model_id: &str) -> bool {
        self.get_model(model_id)
            .map(|m| m.requires_network)
            .unwrap_or(false)
    }

    /// Check if a model requires an API key
    pub fn requires_api_key(&self, model_id: &str) -> bool {
        self.get_model(model_id)
            .map(|m| m.requires_api_key)
            .unwrap_or(false)
    }

    /// Get provider information
    pub fn get_provider(&self, provider_name: &str) -> Option<&ProviderInfo> {
        self.raw.providers.get(provider_name)
    }

    /// Get the catalog version
    pub fn version(&self) -> &str {
        &self.raw.metadata.version
    }

    /// Get the catalog hash (for provenance/CAR)
    pub fn hash(&self) -> &str {
        &self.catalog_sha256
    }

    /// Check if the catalog signature was verified
    pub fn is_signature_verified(&self) -> bool {
        self.signature_verified
    }

    /// Create a fallback catalog with safe defaults (if loading fails)
    pub fn fallback_catalog() -> Self {
        eprintln!("⚠️  Using fallback model catalog with default values");

        let raw = RawModelCatalog {
            metadata: CatalogMetadata {
                version: "0.0.0-fallback".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                description: "Fallback catalog with safe defaults".to_string(),
            },
            defaults: CatalogDefaults {
                nature_cost_algorithm: "simple".to_string(),
                fallback_cost_per_million_tokens: 10.0,
                fallback_nature_cost_per_million_tokens: 5.0,
            },
            nature_cost_algorithms: HashMap::new(),
            models: vec![
                ModelDef {
                    id: "stub-model".to_string(),
                    provider: "internal".to_string(),
                    display_name: "Stub Model".to_string(),
                    api_name: None,
                    description: "Testing model".to_string(),
                    cost_per_million_tokens: 0.0,
                    nature_cost_per_million_tokens: 0.0,
                    energy_kwh_per_million_tokens: 0.0,
                    enabled: true,
                    tags: vec!["testing".to_string()],
                    context_window: None,
                    max_output_tokens: None,
                    requires_network: false,
                    requires_api_key: false,
                },
            ],
            providers: HashMap::new(),
            signature: None,
        };

        let models_by_id = raw
            .models
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        ModelCatalog {
            catalog_sha256: "fallback-0000000000000000".to_string(),
            signature_verified: false,
            raw,
            models_by_id,
        }
    }
}

/// Global catalog instance (loaded once at startup)
use once_cell::sync::OnceCell;
static GLOBAL_CATALOG: OnceCell<ModelCatalog> = OnceCell::new();

/// Initialize the global model catalog
pub fn init_global_catalog() -> Result<()> {
    let catalog = ModelCatalog::load_default()
        .unwrap_or_else(|err| {
            eprintln!("⚠️  Failed to load model catalog: {}", err);
            eprintln!("   Using fallback catalog with default values");
            ModelCatalog::fallback_catalog()
        });

    GLOBAL_CATALOG
        .set(catalog)
        .map_err(|_| anyhow!("Global catalog already initialized"))?;

    Ok(())
}

/// Get the global model catalog (must be initialized first)
pub fn get_global_catalog() -> &'static ModelCatalog {
    GLOBAL_CATALOG
        .get()
        .expect("Model catalog not initialized - call init_global_catalog() first")
}

/// Try to get the global catalog, or None if not initialized
pub fn try_get_global_catalog() -> Option<&'static ModelCatalog> {
    GLOBAL_CATALOG.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_catalog() {
        let catalog = ModelCatalog::fallback_catalog();
        assert_eq!(catalog.version(), "0.0.0-fallback");
        assert!(!catalog.is_signature_verified());
        assert!(catalog.get_model("stub-model").is_some());
    }

    #[test]
    fn test_cost_calculation() {
        let catalog = ModelCatalog::fallback_catalog();

        // Test with unknown model (uses fallback)
        let cost = catalog.calculate_usd_cost("unknown-model", 1_000_000);
        assert_eq!(cost, 10.0); // fallback_cost_per_million_tokens

        // Test with stub model (free)
        let cost = catalog.calculate_usd_cost("stub-model", 1_000_000);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_nature_cost_calculation() {
        let catalog = ModelCatalog::fallback_catalog();

        let nature_cost = catalog.calculate_nature_cost("stub-model", 1_000_000);
        assert_eq!(nature_cost, 0.0);
    }

    #[test]
    fn test_nature_cost_uses_fallback_when_model_missing_value() {
        let toml = r#"
[metadata]
version = "1.0.0"
created_at = "2025-01-01T00:00:00Z"
description = "Test catalog"

[defaults]
nature_cost_algorithm = "simple"
fallback_cost_per_million_tokens = 5.0
fallback_nature_cost_per_million_tokens = 2.5

[[models]]
id = "zero-nature"
provider = "test"
display_name = "Zero Nature"
description = "Model without nature cost data"
cost_per_million_tokens = 0.0
nature_cost_per_million_tokens = 0.0
energy_kwh_per_million_tokens = 0.0
enabled = true

[providers.test]
name = "Test Provider"
description = "Test provider"
"#;

        let catalog = ModelCatalog::load_from_str(toml).unwrap();
        let nature_cost = catalog.calculate_nature_cost("zero-nature", 1_000_000);
        assert_eq!(nature_cost, 2.5);
    }

    #[test]
    fn test_load_from_str() {
        let toml = r#"
[metadata]
version = "1.0.0"
created_at = "2025-01-01T00:00:00Z"
description = "Test catalog"

[defaults]
nature_cost_algorithm = "simple"
fallback_cost_per_million_tokens = 5.0
fallback_nature_cost_per_million_tokens = 2.5

[nature_cost_algorithms.simple]
formula = "test"
description = "Test algorithm"

[[models]]
id = "test-model"
provider = "test"
display_name = "Test Model"
description = "A test model"
cost_per_million_tokens = 1.0
nature_cost_per_million_tokens = 0.5
energy_kwh_per_million_tokens = 0.1
enabled = true

[providers.test]
name = "Test Provider"
description = "Test provider"
"#;

        let catalog = ModelCatalog::load_from_str(toml).unwrap();
        assert_eq!(catalog.version(), "1.0.0");
        assert!(catalog.get_model("test-model").is_some());

        let cost = catalog.calculate_usd_cost("test-model", 1_000_000);
        assert_eq!(cost, 1.0);
    }
}
