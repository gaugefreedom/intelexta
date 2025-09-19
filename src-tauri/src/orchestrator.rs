// src-tauri/src/orchestrator.rs
use crate::{governance, provenance, DbPool};
use anyhow::{anyhow, Context};
use chrono::Utc;
use ed25519_dalek::SigningKey;
use keyring::Error as KeyringError;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
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

    let signing_key = ensure_project_signing_key(&conn, &spec.project_id)?;
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
    let signature = provenance::sign_bytes(&signing_key, curr_chain.as_bytes());

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

fn ensure_project_signing_key(conn: &Connection, project_id: &str) -> anyhow::Result<SigningKey> {
    match provenance::load_secret_key(project_id) {
        Ok(signing_key) => Ok(signing_key),
        Err(err) => {
            let missing_in_keyring = err
                .downcast_ref::<KeyringError>()
                .map(|inner| matches!(inner, KeyringError::NoEntry))
                .unwrap_or(false);

            let missing_on_disk = err
                .downcast_ref::<std::io::Error>()
                .map(|io_err| io_err.kind() == ErrorKind::NotFound)
                .unwrap_or(false);

            if missing_in_keyring || missing_on_disk {
                println!(
                    "[intelexta] WARNING: Secret for project {} missing; generating a new key pair.",
                    project_id
                );
                regenerate_project_signing_key(conn, project_id)
                    .context("failed to regenerate missing project secret")
            } else {
                Err(err)
            }
        }
    }
}

fn regenerate_project_signing_key(
    conn: &Connection,
    project_id: &str,
) -> anyhow::Result<SigningKey> {
    let keypair = provenance::generate_keypair();

    provenance::store_secret_key(project_id, &keypair.secret_key_b64)
        .context("failed to persist regenerated project secret")?;

    let updated = conn.execute(
        "UPDATE projects SET pubkey = ?1 WHERE id = ?2",
        params![keypair.public_key_b64, project_id],
    )?;

    if updated == 0 {
        return Err(anyhow!(
            "project {project_id} not found while regenerating secret"
        ));
    }

    provenance::load_secret_key(project_id).context("failed to load regenerated project secret")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{keychain, provenance, store};
    use anyhow::{anyhow, Result};
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use ed25519_dalek::SigningKey;
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    use rusqlite::params;
    use std::convert::{TryFrom, TryInto};
    use std::path::PathBuf;
    use std::sync::Once;

    fn init_keychain_backend() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let base_dir = std::env::temp_dir().join(format!(
                "intelexta-orchestrator-tests-{}",
                std::process::id()
            ));
            std::fs::create_dir_all(&base_dir).expect("create orchestrator keychain dir");
            std::env::set_var("INTELEXTA_KEYCHAIN_DIR", &base_dir);
        });
        keychain::force_fallback_for_tests();
    }

    #[test]
    fn start_hello_run_persists_run_and_checkpoint() -> Result<()> {
        init_keychain_backend();

        let manager = SqliteConnectionManager::memory();
        let pool: Pool<SqliteConnectionManager> = Pool::builder().max_size(1).build(manager)?;
        {
            let mut conn = pool.get()?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            store::migrate_db(&mut conn)?;
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

    #[test]
    fn start_hello_run_regenerates_secret_when_missing() -> Result<()> {
        init_keychain_backend();

        let manager = SqliteConnectionManager::memory();
        let pool: Pool<SqliteConnectionManager> = Pool::builder().max_size(1).build(manager)?;
        {
            let mut conn = pool.get()?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            store::migrate_db(&mut conn)?;
        }

        let project_id = "proj-missing-secret";
        let placeholder_keys = provenance::generate_keypair();
        let original_pubkey = placeholder_keys.public_key_b64.clone();

        {
            let conn = pool.get()?;
            let created_at = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO projects (id, name, created_at, pubkey) VALUES (?1, ?2, ?3, ?4)",
                params![project_id, "Missing Secret", created_at, &original_pubkey],
            )?;
        }

        // Intentionally skip storing a secret for this project to simulate a missing key entry.

        let spec = RunSpec {
            project_id: project_id.to_string(),
            name: "recover-secret".to_string(),
            seed: 99,
            dag_json: "{}".to_string(),
            token_budget: 25,
        };

        let run_id = start_hello_run(&pool, spec)?;
        assert!(!run_id.is_empty());

        let conn = pool.get()?;
        let pubkey_after: String = conn.query_row(
            "SELECT pubkey FROM projects WHERE id = ?1",
            params![project_id],
            |row| row.get(0),
        )?;

        // The orchestrator should have rotated the key and stored a new secret.
        assert_ne!(pubkey_after, original_pubkey);

        let recovered_secret = provenance::load_secret_key(project_id)?;
        let derived_pubkey = provenance::public_key_from_secret(&recovered_secret);
        assert_eq!(pubkey_after, derived_pubkey);

        let fallback_dir = PathBuf::from(std::env::var("INTELEXTA_KEYCHAIN_DIR")?);
        let fallback_path = fallback_dir.join(format!("{}.key", project_id));
        assert!(
            fallback_path.exists(),
            "regenerated key should be persisted to fallback store"
        );

        Ok(())
    }
}
