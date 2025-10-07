// src-tauri/src/governance.rs
use crate::model_catalog;
use crate::store::policies::Policy;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Budgets {
    pub usd: f64,
    pub tokens: u64,
    pub nature_cost: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Incident {
    pub kind: String, // "budget_exceeded", "network_denied", "nature_cost_warning", ...
    pub severity: String, // "error" | "warn" | "info"
    pub details: String,
}

/// Basic token budget enforcement (used in interactive mode)
pub fn enforce_budget(budget_tokens: u64, usage_tokens: u64) -> Result<(), Incident> {
    if usage_tokens > budget_tokens {
        Err(Incident {
            kind: "budget_exceeded".into(),
            severity: "error".into(),
            details: format!("usage={} > budget={}", usage_tokens, budget_tokens),
        })
    } else {
        Ok(())
    }
}

/// Comprehensive policy enforcement for regular run execution
/// Returns the first BLOCKING violation encountered, or Ok if all checks pass
/// Note: Nature Cost violations return warnings but don't block execution
pub fn enforce_policy(
    policy: &Policy,
    projected_tokens: u64,
    projected_usd: f64,
    projected_nature_cost: f64,
) -> Result<Option<Incident>, Incident> {
    // Check token budget (BLOCKING)
    if projected_tokens > policy.budget_tokens {
        return Err(Incident {
            kind: "budget_exceeded".into(),
            severity: "error".into(),
            details: format!(
                "Projected tokens {} exceeds budget {}",
                projected_tokens, policy.budget_tokens
            ),
        });
    }

    // Check USD budget (BLOCKING)
    if projected_usd > policy.budget_usd {
        return Err(Incident {
            kind: "budget_exceeded".into(),
            severity: "error".into(),
            details: format!(
                "Projected cost ${:.2} exceeds budget ${:.2}",
                projected_usd, policy.budget_usd
            ),
        });
    }

    // Check Nature Cost (WARNING ONLY - non-blocking)
    if projected_nature_cost > policy.budget_nature_cost {
        return Ok(Some(Incident {
            kind: "nature_cost_warning".into(),
            severity: "warn".into(),
            details: format!(
                "Projected Nature Cost {:.2} exceeds budget {:.2} (execution allowed)",
                projected_nature_cost, policy.budget_nature_cost
            ),
        }));
    }

    Ok(None)
}

/// Check if network access is allowed by policy
pub fn enforce_network_policy(policy: &Policy) -> Result<(), Incident> {
    if !policy.allow_network {
        Err(Incident {
            kind: "network_denied".into(),
            severity: "error".into(),
            details: "Network access denied by project policy".into(),
        })
    } else {
        Ok(())
    }
}

/// Estimate USD cost based on token count and model
/// Uses the model catalog for accurate per-model pricing
pub fn estimate_usd_cost(tokens: u64, model_id: Option<&str>) -> f64 {
    if let Some(catalog) = model_catalog::try_get_global_catalog() {
        if let Some(model) = model_id {
            return catalog.calculate_usd_cost(model, tokens);
        }
    }

    // Fallback if catalog not available or model not specified
    const FALLBACK_COST_PER_1K_TOKENS: f64 = 0.01;
    (tokens as f64 / 1000.0) * FALLBACK_COST_PER_1K_TOKENS
}

/// Legacy function for backwards compatibility
#[deprecated(note = "Use estimate_usd_cost with model_id parameter")]
pub fn estimate_usd_cost_legacy(tokens: u64) -> f64 {
    estimate_usd_cost(tokens, None)
}

/// Estimate Nature Cost based on token count and model
/// Uses the model catalog for accurate per-model environmental impact
pub fn estimate_nature_cost(tokens: u64, model_id: Option<&str>) -> f64 {
    if let Some(catalog) = model_catalog::try_get_global_catalog() {
        if let Some(model) = model_id {
            return catalog.calculate_nature_cost(model, tokens);
        }
    }

    // Fallback if catalog not available or model not specified
    const FALLBACK_NATURE_COST_PER_1K_TOKENS: f64 = 1.0;
    (tokens as f64 / 1000.0) * FALLBACK_NATURE_COST_PER_1K_TOKENS
}

/// Legacy function for backwards compatibility
#[deprecated(note = "Use estimate_nature_cost with model_id parameter")]
pub fn estimate_nature_cost_legacy(tokens: u64) -> f64 {
    estimate_nature_cost(tokens, None)
}

/// Estimate energy consumption in kWh for a given model and token count
pub fn estimate_energy_kwh(tokens: u64, model_id: Option<&str>) -> f64 {
    if let Some(catalog) = model_catalog::try_get_global_catalog() {
        if let Some(model) = model_id {
            return catalog.calculate_energy_kwh(model, tokens);
        }
    }

    // Fallback: assume minimal energy for unknown models
    0.0
}
