// src-tauri/src/governance.rs
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

/// Estimate USD cost based on token count
/// Using rough industry averages: ~$0.01 per 1000 tokens (adjustable)
pub fn estimate_usd_cost(tokens: u64) -> f64 {
    const COST_PER_1K_TOKENS: f64 = 0.01;
    (tokens as f64 / 1000.0) * COST_PER_1K_TOKENS
}

/// Estimate Nature Cost based on token count
/// PLACEHOLDER: In production, this will use user-configurable algorithms
/// that may incorporate: carbon emissions, water usage, energy consumption,
/// computational resource intensity, and other environmental factors.
///
/// Current implementation uses a simple baseline:
/// - Base carbon: ~0.5g CO₂e per 1000 tokens
/// - Additional factors can be layered in later
pub fn estimate_nature_cost(tokens: u64) -> f64 {
    // Placeholder algorithm - will be user-configurable
    const BASE_NATURE_COST_PER_1K_TOKENS: f64 = 1.0;

    // TODO: In future versions, this will call into a configurable
    // algorithm system where users can define their own Nature Cost
    // calculation methods based on:
    // - Model type (local vs cloud, size, architecture)
    // - Energy source (renewable percentage, grid carbon intensity)
    // - Hardware efficiency (GPU type, utilization)
    // - Data center location (cooling requirements, PUE)
    // - Time of day (grid carbon intensity varies)

    (tokens as f64 / 1000.0) * BASE_NATURE_COST_PER_1K_TOKENS
}
