// src-tauri/src/api_keys.rs
//!
//! API Key Management for LLM Providers
//!
//! This module extends the existing keychain infrastructure to support
//! managing API keys for different LLM providers (Anthropic, OpenAI, Google, etc.)
//!
//! Keys are stored per-user (not per-project) and use the OS keyring when available,
//! with filesystem fallback.

use crate::keychain;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Supported LLM providers that require API keys
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyProvider {
    Anthropic,
    OpenAI,
    Google,
    Groq,
    XAI,
}

impl ApiKeyProvider {
    /// Get the keychain identifier for this provider
    fn keychain_id(&self) -> String {
        match self {
            ApiKeyProvider::Anthropic => "api_key_anthropic".to_string(),
            ApiKeyProvider::OpenAI => "api_key_openai".to_string(),
            ApiKeyProvider::Google => "api_key_google".to_string(),
            ApiKeyProvider::Groq => "api_key_groq".to_string(),
            ApiKeyProvider::XAI => "api_key_xai".to_string(),
        }
    }

    /// Get the display name for this provider
    pub fn display_name(&self) -> &'static str {
        match self {
            ApiKeyProvider::Anthropic => "Anthropic (Claude)",
            ApiKeyProvider::OpenAI => "OpenAI (GPT)",
            ApiKeyProvider::Google => "Google (Gemini)",
            ApiKeyProvider::Groq => "Groq",
            ApiKeyProvider::XAI => "xAI (Grok)",
        }
    }

    /// Get example key format for this provider
    pub fn example_format(&self) -> &'static str {
        match self {
            ApiKeyProvider::Anthropic => "sk-ant-...",
            ApiKeyProvider::OpenAI => "sk-...",
            ApiKeyProvider::Google => "AIza...",
            ApiKeyProvider::Groq => "gsk_...",
            ApiKeyProvider::XAI => "xai-...",
        }
    }

    /// Parse provider from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" => Some(ApiKeyProvider::Anthropic),
            "openai" => Some(ApiKeyProvider::OpenAI),
            "google" => Some(ApiKeyProvider::Google),
            "groq" => Some(ApiKeyProvider::Groq),
            "xai" => Some(ApiKeyProvider::XAI),
            _ => None,
        }
    }

    /// Get all providers
    pub fn all() -> Vec<Self> {
        vec![
            ApiKeyProvider::Anthropic,
            ApiKeyProvider::OpenAI,
            ApiKeyProvider::Google,
            ApiKeyProvider::Groq,
            ApiKeyProvider::XAI,
        ]
    }
}

/// Store an API key for a provider
pub fn store_api_key(provider: ApiKeyProvider, api_key: &str) -> Result<()> {
    let keychain_id = provider.keychain_id();
    keychain::store_secret(&keychain_id, api_key)
        .with_context(|| format!("Failed to store API key for {}", provider.display_name()))
}

/// Load an API key for a provider
pub fn load_api_key(provider: ApiKeyProvider) -> Result<String> {
    let keychain_id = provider.keychain_id();
    keychain::load_secret(&keychain_id)
        .with_context(|| format!("Failed to load API key for {}", provider.display_name()))
}

/// Check if an API key exists for a provider
pub fn has_api_key(provider: ApiKeyProvider) -> bool {
    load_api_key(provider).is_ok()
}

/// Delete an API key for a provider
pub fn delete_api_key(provider: ApiKeyProvider) -> Result<()> {
    let keychain_id = provider.keychain_id();
    keychain::delete_secret(&keychain_id)
        .with_context(|| format!("Failed to delete API key for {}", provider.display_name()))
}

/// Get status of all API keys (which are configured)
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyStatus {
    pub provider: ApiKeyProvider,
    pub display_name: String,
    pub is_configured: bool,
    pub example_format: String,
}

pub fn get_all_api_key_status() -> Vec<ApiKeyStatus> {
    ApiKeyProvider::all()
        .into_iter()
        .map(|provider| ApiKeyStatus {
            display_name: provider.display_name().to_string(),
            is_configured: has_api_key(provider),
            example_format: provider.example_format().to_string(),
            provider,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_str() {
        assert_eq!(
            ApiKeyProvider::from_str("anthropic"),
            Some(ApiKeyProvider::Anthropic)
        );
        assert_eq!(
            ApiKeyProvider::from_str("OPENAI"),
            Some(ApiKeyProvider::OpenAI)
        );
        assert_eq!(ApiKeyProvider::from_str("invalid"), None);
    }

    #[test]
    fn test_provider_display_names() {
        assert_eq!(
            ApiKeyProvider::Anthropic.display_name(),
            "Anthropic (Claude)"
        );
        assert_eq!(ApiKeyProvider::OpenAI.display_name(), "OpenAI (GPT)");
    }

    #[test]
    fn test_keychain_ids_are_unique() {
        let providers = ApiKeyProvider::all();
        let mut ids = std::collections::HashSet::new();

        for provider in providers {
            let id = provider.keychain_id();
            assert!(ids.insert(id), "Duplicate keychain ID found");
        }
    }
}
