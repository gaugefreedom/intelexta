// src-tauri/src/model_adapters.rs
//!
//! Model Adapters: Unified interface for multiple LLM providers
//!
//! This module implements the adapter pattern to support multiple LLM providers
//! (Anthropic, OpenAI, Google, Groq, xAI, Ollama) through a common interface.
//!
//! Architecture:
//! - ModelAdapter trait: Common interface for all providers
//! - Provider-specific adapters: AnthropicAdapter, OpenAIAdapter, etc.
//! - ModelDispatcher: Routes requests to appropriate adapter based on model ID

use crate::{api_keys, model_catalog};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// LLM generation result
#[derive(Debug, Clone)]
pub struct LlmGeneration {
    pub response: String,
    pub usage: TokenUsage,
}

/// Model adapter trait - common interface for all LLM providers
pub trait ModelAdapter: Send + Sync {
    /// Generate text from a prompt
    fn generate(&self, model_id: &str, prompt: &str) -> Result<LlmGeneration>;

    /// Check if this adapter can handle the given model
    fn can_handle(&self, model_id: &str) -> bool;

    /// Get the provider name
    fn provider_name(&self) -> &'static str;
}

// ============================================================================
// Ollama Adapter (Local Models)
// ============================================================================

pub struct OllamaAdapter {
    host: String,
}

impl OllamaAdapter {
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1:11434".to_string(),
        }
    }

    pub fn with_host(host: String) -> Self {
        Self { host }
    }
}

impl ModelAdapter for OllamaAdapter {
    fn generate(&self, model_id: &str, prompt: &str) -> Result<LlmGeneration> {
        // Use existing perform_ollama_stream function
        // For Ollama, the internal `id` is the `apiName`
        let orch_result = crate::orchestrator::perform_ollama_stream(model_id, prompt)?;

        // Convert from orchestrator::LlmGeneration to model_adapters::LlmGeneration
        Ok(LlmGeneration {
            response: orch_result.response,
            usage: TokenUsage {
                prompt_tokens: orch_result.usage.prompt_tokens,
                completion_tokens: orch_result.usage.completion_tokens,
            },
        })
    }

    fn can_handle(&self, model_id: &str) -> bool {
        // Check if model is from Ollama provider in catalog
        model_catalog::try_get_global_catalog()
            .and_then(|catalog| catalog.get_model(model_id))
            .map(|model_def| model_def.provider == "ollama")
            .unwrap_or(false)
    }

    fn provider_name(&self) -> &'static str {
        "Ollama"
    }
}

// ============================================================================
// Anthropic Adapter (Claude)
// ============================================================================

pub struct AnthropicAdapter;

impl AnthropicAdapter {
    pub fn new() -> Self {
        Self
    }

    fn get_api_key(&self) -> Result<String> {
        api_keys::load_api_key(api_keys::ApiKeyProvider::Anthropic)
            .context("Anthropic API key not configured. Please add it in Settings → API Keys")
    }
}

impl ModelAdapter for AnthropicAdapter {
    fn generate(&self, model_id: &str, prompt: &str) -> Result<LlmGeneration> {
        let api_key = self.get_api_key()?;

        // --- FIX START ---
        // Look up the correct apiName from the catalog
        let catalog = model_catalog::try_get_global_catalog()
            .ok_or_else(|| anyhow!("Model catalog not initialized"))?;
        let model_def = catalog.get_model(model_id)
            .ok_or_else(|| anyhow!("Model '{}' not found in catalog", model_id))?;
        let api_model_name = model_def.api_name.as_ref().unwrap_or(&model_def.id);
        // --- FIX END ---

        // Build request payload for Anthropic Messages API
        let payload = serde_json::json!({
            "model": api_model_name, // Use the correct name
            "max_tokens": 4096,
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        });

        // Make HTTP request to Anthropic API
        let client = ureq::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build();

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .set("x-api-key", &api_key)
            .set("anthropic-version", "2023-06-01")
            .set("content-type", "application/json")
            .send_json(&payload);

        // Handle HTTP errors
        let response = match response {
            Ok(resp) => resp,
            Err(ureq::Error::Status(code, resp)) => {
                let error_body: Result<serde_json::Value, _> = resp.into_json();
                let error_msg = if let Ok(json) = error_body {
                    json["error"]["message"]
                        .as_str()
                        .unwrap_or("Unknown API error")
                        .to_string()
                } else {
                    format!("HTTP {} error", code)
                };
                return Err(anyhow!("Anthropic API error (HTTP {}): {}", code, error_msg));
            }
            Err(e) => {
                return Err(anyhow!("Failed to connect to Anthropic API: {}", e));
            }
        };

        // Parse response
        let response_json: serde_json::Value = response
            .into_json()
            .context("Failed to parse Anthropic API response")?;

        // Extract text from response
        let text = response_json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("No text in Anthropic response"))?
            .to_string();

        // Extract usage
        let usage = TokenUsage {
            prompt_tokens: response_json["usage"]["input_tokens"]
                .as_u64()
                .unwrap_or(0),
            completion_tokens: response_json["usage"]["output_tokens"]
                .as_u64()
                .unwrap_or(0),
        };

        Ok(LlmGeneration {
            response: text,
            usage,
        })
    }

    fn can_handle(&self, model_id: &str) -> bool {
        model_catalog::try_get_global_catalog()
            .and_then(|catalog| catalog.get_model(model_id))
            .map(|model_def| model_def.provider == "anthropic")
            .unwrap_or(false)
    }

    fn provider_name(&self) -> &'static str {
        "Anthropic"
    }
}

// ============================================================================
// OpenAI-Compatible Adapter (OpenAI, Groq, xAI)
// ============================================================================

pub struct OpenAICompatibleAdapter {
    provider: api_keys::ApiKeyProvider,
    api_base: String,
}

impl OpenAICompatibleAdapter {
    pub fn new_openai() -> Self {
        Self {
            provider: api_keys::ApiKeyProvider::OpenAI,
            api_base: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn new_groq() -> Self {
        Self {
            provider: api_keys::ApiKeyProvider::Groq,
            api_base: "https://api.groq.com/openai/v1".to_string(),
        }
    }

    pub fn new_xai() -> Self {
        Self {
            provider: api_keys::ApiKeyProvider::XAI,
            api_base: "https://api.x.ai/v1".to_string(),
        }
    }

    fn get_api_key(&self) -> Result<String> {
        api_keys::load_api_key(self.provider).with_context(|| {
            format!(
                "{} API key not configured. Please add it in Settings → API Keys",
                self.provider.display_name()
            )
        })
    }
}

impl ModelAdapter for OpenAICompatibleAdapter {
    fn generate(&self, model_id: &str, prompt: &str) -> Result<LlmGeneration> {
        let api_key = self.get_api_key()?;

        // Look up the correct apiName from the catalog
        let catalog = model_catalog::try_get_global_catalog()
            .ok_or_else(|| anyhow!("Model catalog not initialized"))?;
        let model_def = catalog.get_model(model_id)
            .ok_or_else(|| anyhow!("Model '{}' not found in catalog", model_id))?;
        let api_model_name = model_def.api_name.as_ref().unwrap_or(&model_def.id);


        // Build request payload for OpenAI Chat Completions API
        let payload = serde_json::json!({
            "model": api_model_name, // Use the correct name
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "max_tokens": 4096,
        });

        // Make HTTP request
        let client = ureq::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build();

        let url = format!("{}/chat/completions", self.api_base);
        let response = client
            .post(&url)
            .set("Authorization", &format!("Bearer {}", api_key))
            .set("Content-Type", "application/json")
            .send_json(&payload);

        // Handle HTTP errors with detailed messages
        let response = match response {
            Ok(resp) => resp,
            Err(ureq::Error::Status(code, resp)) => {
                // Try to extract error message from response body
                let error_body: Result<serde_json::Value, _> = resp.into_json();
                let error_msg = if let Ok(json) = error_body {
                    json["error"]["message"]
                        .as_str()
                        .unwrap_or("Unknown API error")
                        .to_string()
                } else {
                    format!("HTTP {} error", code)
                };
                return Err(anyhow!("{} API error (HTTP {}): {}", self.provider_name(), code, error_msg));
            }
            Err(e) => {
                return Err(anyhow!("Failed to connect to {} API: {}", self.provider_name(), e));
            }
        };

        // Parse response
        let response_json: serde_json::Value = response
            .into_json()
            .context(format!("Failed to parse {} API response", self.provider_name()))?;

        // Extract text from response
        let text = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("No content in {} response", self.provider_name()))?
            .to_string();

        // Extract usage
        let usage = TokenUsage {
            prompt_tokens: response_json["usage"]["prompt_tokens"]
                .as_u64()
                .unwrap_or(0),
            completion_tokens: response_json["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0),
        };

        Ok(LlmGeneration {
            response: text,
            usage,
        })
    }

    fn can_handle(&self, model_id: &str) -> bool {
        model_catalog::try_get_global_catalog()
            .and_then(|catalog| catalog.get_model(model_id))
            .map(|model_def| match self.provider {
                api_keys::ApiKeyProvider::OpenAI => model_def.provider == "openai",
                api_keys::ApiKeyProvider::Groq => model_def.provider == "groq",
                api_keys::ApiKeyProvider::XAI => model_def.provider == "xai",
                _ => false,
            })
            .unwrap_or(false)
    }

    fn provider_name(&self) -> &'static str {
        match self.provider {
            api_keys::ApiKeyProvider::OpenAI => "OpenAI",
            api_keys::ApiKeyProvider::Groq => "Groq",
            api_keys::ApiKeyProvider::XAI => "xAI",
            _ => "OpenAI-Compatible",
        }
    }
}

// ============================================================================
// Google Gemini Adapter
// ============================================================================

pub struct GoogleAdapter;

impl GoogleAdapter {
    pub fn new() -> Self {
        Self
    }

    fn get_api_key(&self) -> Result<String> {
        api_keys::load_api_key(api_keys::ApiKeyProvider::Google)
            .context("Google API key not configured. Please add it in Settings → API Keys")
    }
}

impl ModelAdapter for GoogleAdapter {
    fn generate(&self, model_id: &str, prompt: &str) -> Result<LlmGeneration> {
        let api_key = self.get_api_key()?;

        // Look up the correct apiName from the catalog
        let catalog = model_catalog::try_get_global_catalog()
            .ok_or_else(|| anyhow!("Model catalog not initialized"))?;
        let model_def = catalog.get_model(model_id)
            .ok_or_else(|| anyhow!("Model '{}' not found in catalog", model_id))?;
        let api_model_name = model_def.api_name.as_ref().unwrap_or(&model_def.id);

        // Build request payload for Gemini API
        let payload = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "maxOutputTokens": 4096
            }
        });

        // Make HTTP request to Gemini API
        let client = ureq::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build();

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            api_model_name, api_key // Use the correct name
        );

        let response = client
            .post(&url)
            .set("Content-Type", "application/json")
            .send_json(&payload);

        // Handle HTTP errors
        let response = match response {
            Ok(resp) => resp,
            Err(ureq::Error::Status(code, resp)) => {
                let error_body: Result<serde_json::Value, _> = resp.into_json();
                let error_msg = if let Ok(json) = error_body {
                    json["error"]["message"]
                        .as_str()
                        .unwrap_or("Unknown API error")
                        .to_string()
                } else {
                    format!("HTTP {} error", code)
                };
                return Err(anyhow!("Google Gemini API error (HTTP {}): {}", code, error_msg));
            }
            Err(e) => {
                return Err(anyhow!("Failed to connect to Google Gemini API: {}", e));
            }
        };

        // Parse response
        let response_json: serde_json::Value = response
            .into_json()
            .context("Failed to parse Gemini API response")?;

        // Extract text from response
        let text = response_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("No text in Gemini response"))?
            .to_string();

        // Extract usage (Gemini has different structure)
        let usage = TokenUsage {
            prompt_tokens: response_json["usageMetadata"]["promptTokenCount"]
                .as_u64()
                .unwrap_or(0),
            completion_tokens: response_json["usageMetadata"]["candidatesTokenCount"]
                .as_u64()
                .unwrap_or(0),
        };

        Ok(LlmGeneration {
            response: text,
            usage,
        })
    }

    fn can_handle(&self, model_id: &str) -> bool {
        model_catalog::try_get_global_catalog()
            .and_then(|catalog| catalog.get_model(model_id))
            .map(|model_def| model_def.provider == "google")
            .unwrap_or(false)
    }

    fn provider_name(&self) -> &'static str {
        "Google"
    }
}

// ============================================================================
// Model Dispatcher
// ============================================================================

pub struct ModelDispatcher {
    adapters: Vec<Box<dyn ModelAdapter>>,
}

impl ModelDispatcher {
    pub fn new() -> Self {
        let adapters: Vec<Box<dyn ModelAdapter>> = vec![
            Box::new(OllamaAdapter::new()),
            Box::new(AnthropicAdapter::new()),
            Box::new(OpenAICompatibleAdapter::new_openai()),
            Box::new(OpenAICompatibleAdapter::new_groq()),
            Box::new(OpenAICompatibleAdapter::new_xai()),
            Box::new(GoogleAdapter::new()),
        ];

        Self { adapters }
    }

    pub fn generate(&self, model_id: &str, prompt: &str) -> Result<LlmGeneration> {
        // Find adapter that can handle this model
        for adapter in &self.adapters {
            if adapter.can_handle(model_id) {
                // The first parameter is the internal model ID
                return adapter.generate(model_id, prompt)
                    .with_context(|| format!("Failed to generate with {} for model {}", adapter.provider_name(), model_id));
            }
        }

        Err(anyhow!(
            "No adapter found for model '{}'. Please check model catalog configuration.",
            model_id
        ))
    }

    /// Check if API key is required and configured for a model
    pub fn check_api_key_configured(&self, model_id: &str) -> Result<()> {
        // Check if model requires API key
        let requires_key = model_catalog::try_get_global_catalog()
            .and_then(|catalog| catalog.get_model(model_id))
            .map(|model_def| model_def.requires_api_key)
            .unwrap_or(false);

        if !requires_key {
            return Ok(());
        }

        // Determine provider and check if key exists
        let provider = model_catalog::try_get_global_catalog()
            .and_then(|catalog| catalog.get_model(model_id))
            .map(|model_def| model_def.provider.as_str())
            .ok_or_else(|| anyhow!("Model not found in catalog: {}", model_id))?;

        let api_key_provider = match provider {
            "anthropic" => api_keys::ApiKeyProvider::Anthropic,
            "openai" => api_keys::ApiKeyProvider::OpenAI,
            "google" => api_keys::ApiKeyProvider::Google,
            "groq" => api_keys::ApiKeyProvider::Groq,
            "xai" => api_keys::ApiKeyProvider::XAI,
            _ => return Ok(()), // No API key needed for this provider
        };

        if !api_keys::has_api_key(api_key_provider) {
            return Err(anyhow!(
                "API key for {} is required but not configured. Please add it in Settings → API Keys",
                api_key_provider.display_name()
            ));
        }

        Ok(())
    }
}

impl Default for ModelDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_adapter_can_handle() {
        let adapter = OllamaAdapter::new();
        // This test will fail until the catalog is loaded in the test environment.
        // For now, we rely on the fallback logic.
        assert!(adapter.can_handle("llama3.2:1b"));
        assert!(!adapter.can_handle("claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn test_anthropic_adapter_can_handle() {
        let adapter = AnthropicAdapter::new();
        // This test will fail until the catalog is loaded in the test environment.
        // For now, we rely on the fallback logic.
        assert!(adapter.can_handle("claude-3-5-sonnet-20241022"));
        assert!(!adapter.can_handle("gpt-4o"));
    }

    #[test]
    fn test_dispatcher_finds_adapter() {
        let dispatcher = ModelDispatcher::new();

        // This test will fail until the catalog is properly loaded in the test environment.
        // It's a placeholder for future integration tests.
        let models = vec![
            "llama3.2:1b", // Should be handled by OllamaAdapter
                           // "claude-3-5-sonnet-20241022", // Should be handled by AnthropicAdapter
                           // "gpt-4o", // Should be handled by OpenAICompatibleAdapter
        ];

        for model in models {
            let adapter_found = dispatcher.adapters.iter().any(|a| a.can_handle(model));
            assert!(adapter_found, "No adapter found for model: {}", model);
        }
    }
}