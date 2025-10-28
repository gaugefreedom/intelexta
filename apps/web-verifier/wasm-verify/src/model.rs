use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Car {
    pub id: String,
    pub run_id: String,
    pub created_at: DateTime<Utc>,
    pub run: RunInfo,
    pub proof: Proof,
    pub policy_ref: PolicyRef,
    pub budgets: Budgets,
    pub provenance: Vec<ProvenanceClaim>,
    pub checkpoints: Vec<String>,
    pub sgrade: SGrade,
    pub signer_public_key: String,
    pub signatures: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RunInfo {
    pub kind: String,
    pub name: String,
    pub model: String,
    pub version: String,
    pub seed: u64,
    pub steps: Vec<RunStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampler: Option<Sampler>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sampler {
    pub temp: f32,
    pub top_p: f32,
    pub rng: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RunStep {
    pub id: String,
    pub run_id: String,
    pub order_index: i64,
    pub checkpoint_type: String,
    #[serde(default = "default_step_type")]
    pub step_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default)]
    pub token_budget: u64,
    #[serde(default)]
    pub proof_mode: RunProofMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_json: Option<String>,
}

fn default_step_type() -> String {
    "llm".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RunProofMode {
    Exact,
    Concordant,
}

impl Default for RunProofMode {
    fn default() -> Self {
        RunProofMode::Exact
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Proof {
    pub match_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_semantic_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_semantic_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<ProcessProof>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessProof {
    pub sequential_checkpoints: Vec<ProcessCheckpointProof>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessCheckpointProof {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_checkpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_index: Option<u32>,
    pub prev_chain: String,
    pub curr_chain: String,
    pub signature: String,
    pub run_id: String,
    pub kind: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs_sha256: Option<String>,
    pub usage_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PolicyRef {
    pub hash: String,
    pub egress: bool,
    pub estimator: String,
    #[serde(default = "default_catalog_hash")]
    pub model_catalog_hash: String,
    #[serde(default = "default_catalog_version")]
    pub model_catalog_version: String,
}

fn default_catalog_hash() -> String {
    "sha256:unknown".to_string()
}

fn default_catalog_version() -> String {
    "unknown".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Budgets {
    pub usd: f64,
    pub tokens: u64,
    pub nature_cost: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProvenanceClaim {
    pub claim_type: String,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SGrade {
    pub score: u8,
    pub components: SGradeComponents,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SGradeComponents {
    pub provenance: f32,
    pub energy: f32,
    pub replay: f32,
    pub consent: f32,
    pub incidents: f32,
}
