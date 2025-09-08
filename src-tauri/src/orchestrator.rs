// src-tauri/src/orchestrator.rs
use crate::{DbPool, provenance, governance};
use chrono::Utc;
use rusqlite::params;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
struct CheckpointBody<'a> {
    run_id: &'a str,
    kind: &'a str,                // "Step" or "Incident"
    timestamp: String,
    inputs_sha256: Option<&'a str>,
    outputs_sha256: Option<&'a str>,
    incident: Option<&'a serde_json::Value>,
    usage_tokens: u64,
}

pub struct RunSpec {
    pub project_id: String,
    pub name: String,
    pub seed: u64,
    pub dag_json: String,
    pub token_budget: u64,
}

pub fn start_hello_run(pool: &DbPool, spec: RunSpec) -> anyhow::Result<String> {
    let conn = pool.get()?;
    let run_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Create run row
    conn.execute(
        "INSERT INTO runs (id, project_id, name, kind, seed, dag_json, created_at) VALUES (?1,?2,?3,'exact',?4,?5,?6)",
        params![&run_id, &spec.project_id, &spec.name, &(spec.seed as i64), &spec.dag_json, &now],
    )?;

    // Deterministic stub op: sha256("hello" || seed_le)
    let mut input = b"hello".to_vec();
    input.extend_from_slice(&spec.seed.to_le_bytes());
    let outputs_hex = provenance::sha256_hex(&input);
    let inputs_hex = provenance::sha256_hex(b"hello");

    // Budget check: pretend we used 10 tokens
    let usage_tokens = 10_u64;
    let budget_ok = governance::enforce_budget(spec.token_budget, usage_tokens);

    // Load secret & compute signed, hash-chained checkpoint
    let sk = provenance::load_secret_key(&spec.project_id)?;
    let prev_chain = ""; // first checkpoint in run
    let body_json = match &budget_ok {
        Ok(_) => serde_json::json!(CheckpointBody {
            run_id: &run_id, kind: "Step", timestamp: now.clone(),
            inputs_sha256: Some(&inputs_hex), outputs_sha256: Some(&outputs_hex),
            incident: None, usage_tokens
        }),
        Err(inc) => {
            let inc_json = serde_json::to_value(inc)?;
            serde_json::json!(CheckpointBody {
                run_id: &run_id, kind: "Incident", timestamp: now.clone(),
                inputs_sha256: None, outputs_sha256: None,
                incident: Some(&inc_json), usage_tokens
            })
        }
    };

    let canon = provenance::canonical_json(&body_json);
    let curr_chain = provenance::sha256_hex(&[prev_chain.as_bytes(), &canon].concat());
    let signature = provenance::sign_bytes(&sk, curr_chain.as_bytes());

    let ckpt_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO checkpoints (id, run_id, kind, incident_json, timestamp, inputs_sha256, outputs_sha256, prev_chain, curr_chain, signature, usage_tokens)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            &ckpt_id,
            &run_id,
            body_json.get("kind").and_then(|v| v.as_str()).unwrap_or("Step"),
            body_json.get("incident").and_then(|v| if v.is_null(){None} else {Some(v)}).map(|v| v.to_string()),
            now,
            body_json.get("inputs_sha256").and_then(|v| v.as_str()),
            body_json.get("outputs_sha256").and_then(|v| v.as_str()),
            prev_chain, curr_chain, signature, (usage_tokens as i64)
        ]
    )?;

    Ok(run_id)
}
