// src-tauri/src/orchestrator.rs
use crate::{governance, provenance, DbPool};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
struct CheckpointBody<'a> {
    run_id: &'a str,
    kind: &'a str, // "Step" or "Incident"
    timestamp: String,
    inputs_sha256: Option<&'a str>,
    outputs_sha256: Option<&'a str>,
    incident: Option<&'a serde_json::Value>,
    usage_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    let spec_json = serde_json::to_string(&spec)?;

    // Create run row
    conn.execute(
        "INSERT INTO runs (id, project_id, name, created_at, kind, spec_json) VALUES (?1,?2,?3,?4,?5,?6)",
        params![&run_id, &spec.project_id, &spec.name, &now, "exact", &spec_json],
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
            run_id: &run_id,
            kind: "Step",
            timestamp: now.clone(),
            inputs_sha256: Some(&inputs_hex),
            outputs_sha256: Some(&outputs_hex),
            incident: None,
            usage_tokens
        }),
        Err(inc) => {
            let inc_json = serde_json::to_value(inc)?;
            serde_json::json!(CheckpointBody {
                run_id: &run_id,
                kind: "Incident",
                timestamp: now.clone(),
                inputs_sha256: None,
                outputs_sha256: None,
                incident: Some(&inc_json),
                usage_tokens
            })
        }
    };

    let canon = provenance::canonical_json(&body_json);
    let curr_chain = provenance::sha256_hex(&[prev_chain.as_bytes(), &canon].concat());
    let signature = provenance::sign_bytes(&sk, curr_chain.as_bytes());

    let ckpt_id = Uuid::new_v4().to_string();
    let semantic_digest: Option<String> = None;

    conn.execute(
        "INSERT INTO checkpoints (id, run_id, kind, incident_json, timestamp, inputs_sha256, outputs_sha256, prev_chain, curr_chain, signature, usage_tokens, semantic_digest)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            &ckpt_id,
            &run_id,
            body_json.get("kind").and_then(|v| v.as_str()).unwrap_or("Step"),
            body_json.get("incident").and_then(|v| if v.is_null(){None} else {Some(v)}).map(|v| v.to_string()),
            now,
            body_json.get("inputs_sha256").and_then(|v| v.as_str()),
            body_json.get("outputs_sha256").and_then(|v| v.as_str()),
            prev_chain, curr_chain, signature, (usage_tokens as i64), semantic_digest
        ]
    )?;

    Ok(run_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{provenance, store};
    use anyhow::{anyhow, Result};
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use ed25519_dalek::SigningKey;
    use keyring::{mock, set_default_credential_builder};
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    use rusqlite::params;
    use std::convert::{TryFrom, TryInto};

    #[test]
    fn start_hello_run_persists_run_and_checkpoint() -> Result<()> {
        set_default_credential_builder(mock::default_credential_builder());

        let manager = SqliteConnectionManager::memory();
        let pool: Pool<SqliteConnectionManager> = Pool::builder().max_size(1).build(manager)?;
        {
            let conn = pool.get()?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            store::migrate_db(&conn)?;
        }

        let project_id = "proj-test";
        let keypair = provenance::generate_keypair();
        let secret_bytes = STANDARD.decode(&keypair.secret_key_b64)?;
        let secret_array: [u8; 32] = secret_bytes
            .try_into()
            .map_err(|_| anyhow!("unexpected secret length"))?;
        let signing_key = SigningKey::from_bytes(&secret_array);
        let pubkey = provenance::public_key_from_secret(&signing_key);

        {
            let conn = pool.get()?;
            let created_at = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO projects (id, name, created_at, pubkey) VALUES (?1, ?2, ?3, ?4)",
                params![project_id, "Test Project", created_at, pubkey],
            )?;
        }

        provenance::store_secret_key(project_id, &keypair.secret_key_b64)?;

        let spec = RunSpec {
            project_id: project_id.to_string(),
            name: "hello-run".to_string(),
            seed: 42,
            dag_json: "{\"nodes\":[]}".to_string(),
            token_budget: 1_000,
        };
        let spec_clone = spec.clone();
        let run_id = start_hello_run(&pool, spec)?;

        let conn = pool.get()?;
        let (project_id_db, name_db, kind_db, spec_json_db, created_at_db): (
            String,
            String,
            String,
            String,
            String,
        ) = conn.query_row(
            "SELECT project_id, name, kind, spec_json, created_at FROM runs WHERE id = ?1",
            params![&run_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;

        assert_eq!(project_id_db, spec_clone.project_id);
        assert_eq!(name_db, spec_clone.name);
        assert_eq!(kind_db, "exact");
        assert!(!created_at_db.is_empty());

        let stored_spec: RunSpec = serde_json::from_str(&spec_json_db)?;
        assert_eq!(stored_spec, spec_clone);

        let (
            kind,
            timestamp,
            inputs_sha,
            outputs_sha,
            prev_chain,
            curr_chain,
            signature,
            usage_tokens_db,
            incident_json,
            semantic_digest,
        ): (
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
            String,
            i64,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT kind, timestamp, inputs_sha256, outputs_sha256, prev_chain, curr_chain, signature, usage_tokens, incident_json, semantic_digest FROM checkpoints WHERE run_id = ?1",
                params![&run_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                    ))
                },
            )?;

        assert_eq!(kind, "Step");
        assert!(incident_json.is_none());
        assert!(semantic_digest.is_none());
        assert_eq!(prev_chain, "");

        let expected_inputs = provenance::sha256_hex(b"hello");
        assert_eq!(inputs_sha.as_deref(), Some(expected_inputs.as_str()));

        let mut input_bytes = b"hello".to_vec();
        input_bytes.extend_from_slice(&spec_clone.seed.to_le_bytes());
        let expected_outputs = provenance::sha256_hex(&input_bytes);
        assert_eq!(outputs_sha.as_deref(), Some(expected_outputs.as_str()));

        let usage_tokens = u64::try_from(usage_tokens_db)?;
        assert_eq!(usage_tokens, 10);

        let checkpoint_body = CheckpointBody {
            run_id: &run_id,
            kind: &kind,
            timestamp: timestamp.clone(),
            inputs_sha256: inputs_sha.as_deref(),
            outputs_sha256: outputs_sha.as_deref(),
            incident: None,
            usage_tokens,
        };
        let body_value = serde_json::to_value(&checkpoint_body)?;
        let canonical = provenance::canonical_json(&body_value);
        let expected_curr_chain = provenance::sha256_hex(&canonical);
        assert_eq!(curr_chain, expected_curr_chain);

        let expected_signature =
            provenance::sign_bytes(&signing_key, expected_curr_chain.as_bytes());
        assert_eq!(signature, expected_signature);

        Ok(())
    }
}
