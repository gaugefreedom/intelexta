//! car.rs: Content-Addressable Receipt (CAR) Generation
//!
//! This module is responsible for building the verifiable, portable JSON receipts
//! that serve as the ultimate proof of a run's integrity. It also calculates
//! the S-Grade, a score reflecting the run's adherence to best practices.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{orchestrator::RunSpec, provenance, store};
// TODO: You will need a robust canonical JSON crate. `serde_json_canon` is a good choice.
// use serde_json_canon;

// --- CAR v0.2 Schema Definition ---
// These structs define the precise layout of the .car.json file, updated to support
// multiple replay modes (Exact, Concordant, Interactive).

#[derive(Serialize, Deserialize, Debug)]
pub struct Car {
    pub id: String, // "car:..." - sha256 of the canonical body
    pub run_id: String,
    pub created_at: DateTime<Utc>,
    pub run: RunInfo, // Formerly 'runtime'
    pub proof: Proof,
    pub policy_ref: PolicyRef,
    pub budgets: Budgets,
    pub provenance: Vec<ProvenanceClaim>,
    pub checkpoints: Vec<String>, // List of checkpoint IDs
    pub sgrade: SGrade,
    pub signatures: Vec<String>, // e.g., ["ed25519:..."]
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RunInfo {
    pub kind: String, // 'exact' | 'concordant' | 'interactive'
    pub model: String,
    pub version: String,
    pub seed: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampler: Option<Sampler>, // Details for stochastic runs
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sampler {
    pub temp: f32,
    pub top_p: f32,
    pub rng: String, // e.g., "pcg64"
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Proof {
    pub match_kind: String, // 'exact' | 'semantic' | 'process'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>, // Allowed semantic distance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_metric: Option<String>, // e.g., "simhash_hamming_256"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_semantic_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_semantic_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<ProcessProof>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessProof {
    pub sequential_checkpoints: Vec<ProcessCheckpointProof>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessCheckpointProof {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_checkpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_index: Option<u32>,
    pub prev_chain: String,
    pub curr_chain: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PolicyRef {
    pub hash: String,      // A hash of the policy state at the time of the run
    pub egress: bool,      // Was network access allowed?
    pub estimator: String, // e.g., "gCO₂e = tokens * grid_intensity(model, region)"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Budgets {
    pub usd: f64,
    pub tokens: u64,
    pub g_co2e: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProvenanceClaim {
    pub claim_type: String, // "input", "output", "config"
    pub sha256: String,
}

// NOTE: The Replay struct is now replaced by the more detailed `Proof` struct.

#[derive(Serialize, Deserialize, Debug)]
pub struct SGrade {
    pub score: u8, // 0-100
    pub components: SGradeComponents,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SGradeComponents {
    pub provenance: f32, // 0.0 - 1.0
    pub energy: f32,     // 0.0 - 1.0
    pub replay: f32,     // 0.0 - 1.0
    pub consent: f32,    // 0.0 - 1.0
    pub incidents: f32,  // 0.0 - 1.0
}

// --- S-Grade Calculation ---

/// Calculates the S-Grade based on the results of a run.
/// This is a simple weighted average for now, but can evolve.
pub fn calculate_s_grade(
    replay_successful: bool,
    had_incidents: bool,
    energy_estimated: bool,
) -> SGrade {
    // Define the weights for each component. They should sum to 1.0.
    const WEIGHT_PROVENANCE: f32 = 0.30;
    const WEIGHT_REPLAY: f32 = 0.30;
    const WEIGHT_ENERGY: f32 = 0.15;
    const WEIGHT_CONSENT: f32 = 0.15;
    const WEIGHT_INCIDENTS: f32 = 0.10;

    // For S1, we make some assumptions.
    let provenance_score = 1.0; // If a CAR is being made, provenance is assumed to be 100% intact.
    let replay_score = if replay_successful { 1.0 } else { 0.0 };
    let energy_score = if energy_estimated { 1.0 } else { 0.2 }; // Penalize heavily if not estimated
    let consent_score = 0.8; // Placeholder: In the future, this would be read from the project's policy.
    let incidents_score = if had_incidents { 0.0 } else { 1.0 };

    let components = SGradeComponents {
        provenance: provenance_score,
        energy: energy_score,
        replay: replay_score,
        consent: consent_score,
        incidents: incidents_score,
    };

    let final_score = (components.provenance * WEIGHT_PROVENANCE
        + components.replay * WEIGHT_REPLAY
        + components.energy * WEIGHT_ENERGY
        + components.consent * WEIGHT_CONSENT
        + components.incidents * WEIGHT_INCIDENTS)
        * 100.0;

    SGrade {
        score: final_score.round() as u8,
        components,
    }
}

// --- CAR Building Logic ---

struct CheckpointRow {
    id: String,
    kind: String,
    timestamp: DateTime<Utc>,
    inputs_sha256: Option<String>,
    outputs_sha256: Option<String>,
    usage_tokens: u64,
    parent_checkpoint_id: Option<String>,
    turn_index: Option<u32>,
    prev_chain: String,
    curr_chain: String,
    signature: String,
}

pub fn build_car(conn: &Connection, run_id: &str) -> Result<Car> {
    let (project_id, run_created_at, run_kind, spec_json): (String, String, String, String) = conn
        .query_row(
            "SELECT project_id, created_at, kind, spec_json FROM runs WHERE id = ?1",
            params![run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|err| anyhow!("failed to load run {run_id}: {err}"))?;

    let created_at = DateTime::parse_from_rfc3339(&run_created_at)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| anyhow!("invalid run created_at timestamp: {err}"))?;
    let run_spec: RunSpec = serde_json::from_str(&spec_json)
        .map_err(|err| anyhow!("failed to parse stored RunSpec: {err}"))?;

    let mut stmt = conn.prepare(
        "SELECT id, kind, timestamp, inputs_sha256, outputs_sha256, usage_tokens, parent_checkpoint_id, turn_index, prev_chain, curr_chain, signature
         FROM checkpoints WHERE run_id = ?1 ORDER BY timestamp ASC",
    )?;
    let rows = stmt.query_map(params![run_id], |row| {
        let ts: String = row.get(2)?;
        let parsed_ts = DateTime::parse_from_rfc3339(&ts)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
        let usage: i64 = row.get(5)?;
        Ok(CheckpointRow {
            id: row.get(0)?,
            kind: row.get(1)?,
            timestamp: parsed_ts,
            inputs_sha256: row.get(3)?,
            outputs_sha256: row.get(4)?,
            usage_tokens: usage.max(0) as u64,
            parent_checkpoint_id: row.get(6)?,
            turn_index: row
                .get::<_, Option<i64>>(7)?
                .map(|value| value.max(0) as u32),
            prev_chain: row.get(8)?,
            curr_chain: row.get(9)?,
            signature: row.get(10)?,
        })
    })?;

    let mut checkpoints: Vec<CheckpointRow> = Vec::new();
    for row in rows {
        checkpoints.push(row?);
    }

    let policy = store::policies::get(conn, &project_id)?;
    let policy_canon = provenance::canonical_json(&policy);
    let policy_hash = provenance::sha256_hex(&policy_canon);

    let total_usage_tokens: u64 = checkpoints.iter().map(|ck| ck.usage_tokens).sum();
    let usd_per_token = if policy.budget_tokens > 0 {
        policy.budget_usd / policy.budget_tokens as f64
    } else {
        0.0
    };
    let co2_per_token = if policy.budget_tokens > 0 {
        policy.budget_g_co2e / policy.budget_tokens as f64
    } else {
        0.0
    };
    let estimated_usd = usd_per_token * total_usage_tokens as f64;
    let estimated_g_co2e = co2_per_token * total_usage_tokens as f64;

    let mut provenance_claims = Vec::new();
    let spec_canon = provenance::canonical_json(&run_spec);
    let spec_hash = provenance::sha256_hex(&spec_canon);
    provenance_claims.push(ProvenanceClaim {
        claim_type: "config".to_string(),
        sha256: format!("sha256:{spec_hash}"),
    });

    for ck in &checkpoints {
        if let Some(ref input_sha) = ck.inputs_sha256 {
            provenance_claims.push(ProvenanceClaim {
                claim_type: "input".to_string(),
                sha256: format!("sha256:{input_sha}"),
            });
        }
        if let Some(ref output_sha) = ck.outputs_sha256 {
            provenance_claims.push(ProvenanceClaim {
                claim_type: "output".to_string(),
                sha256: format!("sha256:{output_sha}"),
            });
        }
    }

    let model_identifier = format!("workflow:{}", run_spec.name);
    let version_digest = provenance::sha256_hex(run_spec.dag_json.as_bytes());
    let had_incident = checkpoints
        .iter()
        .any(|ck| ck.kind.eq_ignore_ascii_case("Incident"));

    let car_created_at = checkpoints
        .iter()
        .map(|ck| ck.timestamp)
        .max()
        .unwrap_or(created_at);

    let process_proof = if run_kind.eq_ignore_ascii_case("interactive") {
        let sequential = checkpoints
            .iter()
            .map(|ck| ProcessCheckpointProof {
                id: ck.id.clone(),
                parent_checkpoint_id: ck.parent_checkpoint_id.clone(),
                turn_index: ck.turn_index,
                prev_chain: ck.prev_chain.clone(),
                curr_chain: ck.curr_chain.clone(),
                signature: ck.signature.clone(),
            })
            .collect();
        Some(ProcessProof {
            sequential_checkpoints: sequential,
        })
    } else {
        None
    };

    let proof_match_kind = match run_kind.as_str() {
        kind if kind.eq_ignore_ascii_case("interactive") => "process".to_string(),
        kind if kind.eq_ignore_ascii_case("concordant") => "semantic".to_string(),
        _ => "exact".to_string(),
    };

    let checkpoint_ids: Vec<String> = checkpoints.iter().map(|ck| ck.id.clone()).collect();

    let mut car = Car {
        id: String::new(),
        run_id: run_id.to_string(),
        created_at: car_created_at,
        run: RunInfo {
            kind: run_kind,
            model: model_identifier,
            version: version_digest,
            seed: run_spec.seed,
            sampler: None,
        },
        proof: Proof {
            match_kind: proof_match_kind,
            epsilon: None,
            distance_metric: None,
            original_semantic_digest: None,
            replay_semantic_digest: None,
            process: process_proof,
        },
        policy_ref: PolicyRef {
            hash: format!("sha256:{policy_hash}"),
            egress: policy.allow_network,
            estimator: format!("usage_tokens * {:.6} g/token", co2_per_token),
        },
        budgets: Budgets {
            usd: estimated_usd,
            tokens: total_usage_tokens,
            g_co2e: estimated_g_co2e,
        },
        provenance: provenance_claims,
        checkpoints: checkpoint_ids,
        sgrade: calculate_s_grade(true, had_incident, true),
        signatures: Vec::new(),
    };

    let mut body_value = serde_json::to_value(&car)?;
    if let Value::Object(ref mut obj) = body_value {
        obj.remove("id");
        obj.remove("signatures");
    }
    let canonical = provenance::canonical_json(&body_value);
    let car_id = provenance::sha256_hex(&canonical);
    car.id = format!("car:{car_id}");

    let signing_key = provenance::load_secret_key(&project_id)
        .with_context(|| format!("failed to load signing key for project {project_id}"))?;
    let signature = provenance::sign_bytes(&signing_key, car.id.as_bytes());
    car.signatures.push(format!("ed25519:{signature}"));

    Ok(car)
}
