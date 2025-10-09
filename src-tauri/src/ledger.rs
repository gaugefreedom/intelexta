use crate::{
    store::{
        self,
        policies::Policy,
        project_usage_ledgers::{self, ProjectUsageLedger},
    },
    Error,
};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerTotals {
    pub tokens: u64,
    pub usd: f64,
    pub nature_cost: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerBudgets {
    pub tokens: u64,
    pub usd: f64,
    pub nature_cost: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerRemaining {
    pub tokens: i64,
    pub usd: f64,
    pub nature_cost: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLedgerSnapshot {
    pub project_id: String,
    pub policy_version: i64,
    pub totals: LedgerTotals,
    pub budgets: LedgerBudgets,
    pub remaining: LedgerRemaining,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

fn compute_remaining_tokens(policy: &Policy, ledger: &ProjectUsageLedger) -> i64 {
    let budget = policy.budget_tokens as i128;
    let used = ledger.total_tokens as i128;
    (budget - used).clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

fn compute_remaining_usd(policy: &Policy, ledger: &ProjectUsageLedger) -> f64 {
    policy.budget_usd - ledger.total_usd
}

fn compute_remaining_nature_cost(policy: &Policy, ledger: &ProjectUsageLedger) -> f64 {
    policy.budget_nature_cost - ledger.total_nature_cost
}

pub fn get_project_ledger_snapshot(
    conn: &Connection,
    project_id: &str,
) -> Result<ProjectLedgerSnapshot, Error> {
    let policy_version = store::policies::get_current_version(conn, project_id).unwrap_or(0);
    let policy = store::policies::get_for_policy_version(conn, project_id, Some(policy_version))?;
    let ledger = project_usage_ledgers::get(conn, project_id, Some(policy_version))?;

    let totals = LedgerTotals {
        tokens: ledger.total_tokens,
        usd: ledger.total_usd,
        nature_cost: ledger.total_nature_cost,
    };

    let budgets = LedgerBudgets {
        tokens: policy.budget_tokens,
        usd: policy.budget_usd,
        nature_cost: policy.budget_nature_cost,
    };

    let remaining = LedgerRemaining {
        tokens: compute_remaining_tokens(&policy, &ledger),
        usd: compute_remaining_usd(&policy, &ledger),
        nature_cost: compute_remaining_nature_cost(&policy, &ledger),
    };

    Ok(ProjectLedgerSnapshot {
        project_id: project_id.to_string(),
        policy_version,
        totals,
        budgets,
        remaining,
        last_updated: ledger.updated_at,
    })
}
