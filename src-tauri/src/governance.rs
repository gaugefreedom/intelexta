// src-tauri/src/governance.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Budgets { pub usd: f64, pub tokens: u64, pub g_co2e: f64 }

#[derive(Serialize, Deserialize, Clone)]
pub struct Incident {
    pub kind: String,         // "budget_exceeded", ...
    pub severity: String,     // "error" | "warn" | "info"
    pub details: String,
}

pub fn enforce_budget(budget_tokens: u64, usage_tokens: u64) -> Result<(), Incident> {
    if usage_tokens > budget_tokens {
        Err(Incident{
            kind: "budget_exceeded".into(),
            severity: "error".into(),
            details: format!("usage={} > budget={}", usage_tokens, budget_tokens),
        })
    } else {
        Ok(())
    }
}
