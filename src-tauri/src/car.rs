//! car.rs: Content-Addressable Receipt (CAR) Generation
//!
//! This module is responsible for building the verifiable, portable JSON receipts
//! that serve as the ultimate proof of a run's integrity. It also calculates
//! the S-Grade, a score reflecting the run's adherence to best practices.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PolicyRef {
    pub hash: String, // A hash of the policy state at the time of the run
    pub egress: bool, // Was network access allowed?
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
pub fn calculate_s_grade(replay_successful: bool, had_incidents: bool, energy_estimated: bool) -> SGrade {
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

/// Placeholder function to build a CAR.
/// The real implementation will fetch all data from the database based on the run_id.
pub fn build_car(run_id: &str) -> Result<Car, &'static str> {
    // TODO:
    // 1. Fetch the run (including its `kind`), checkpoints, policy, etc. from the DB.
    // 2. Populate all fields of the Car struct based on the run kind.
    // 3. Canonicalize the body, hash for the `id`, sign, and return.

    println!("Building CAR v0.2 for run_id: {}", run_id);
    
    let sgrade = calculate_s_grade(true, false, true);

    // This is a dummy object for a "Concordant" run.
    Ok(Car {
        id: "car:placeholder".to_string(),
        run_id: run_id.to_string(),
        created_at: Utc::now(),
        run: RunInfo {
            kind: "concordant".to_string(),
            model: "llama3-8b-local".to_string(),
            version: "1.0".to_string(),
            seed: 12345,
            sampler: Some(Sampler { temp: 0.7, top_p: 0.95, rng: "pcg64".to_string() }),
        },
        proof: Proof {
            match_kind: "semantic".to_string(),
            epsilon: Some(0.12),
            distance_metric: Some("simhash_hamming_256".to_string()),
            original_semantic_digest: Some("0f1e2d3c...".to_string()),
            replay_semantic_digest: Some("0f1e2d3b...".to_string()),
        },
        policy_ref: PolicyRef {
            hash: "sha256:abc...".to_string(),
            egress: false,
            estimator: "tokens * grid(model,region)".to_string(),
        },
        budgets: Budgets { usd: 0.02, tokens: 1200, g_co2e: 0.8 },
        provenance: vec![
            ProvenanceClaim { claim_type: "input".to_string(), sha256: "sha256:def...".to_string() },
            ProvenanceClaim { claim_type: "output".to_string(), sha256: "sha256:ghi...".to_string() },
        ],
        checkpoints: vec!["ckpt:1".to_string(), "ckpt:2".to_string()],
        sgrade,
        signatures: vec!["ed25519:placeholder_sig".to_string()],
    })
}

