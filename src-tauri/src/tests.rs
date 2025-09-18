use std::any::Any;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::{Arc, Mutex, Once};

use crate::{
    api, orchestrator, provenance,
    store::{
        self,
        policies::{self, Policy},
    },
    DbPool,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signature, Verifier};
use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi, CredentialPersistence};
use keyring::Error as KeyringError;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

fn setup_pool() -> Result<DbPool> {
    let manager = SqliteConnectionManager::memory();
    let pool = r2d2::Pool::builder().max_size(1).build(manager)?;
    {
        let conn = pool.get()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        store::migrate_db(&conn)?;
        let latest_version = store::migrations::latest_version();
        let recorded: Option<i64> =
            conn.query_row("SELECT MAX(version) FROM migrations", [], |row| row.get(0))?;
        assert_eq!(recorded.unwrap_or_default(), latest_version);
    }
    Ok(pool)
}

fn init_keyring_mock() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        keyring::set_default_credential_builder(Box::new(InMemoryCredentialBuilder::default()));
    });
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct EntryKey {
    target: Option<String>,
    service: String,
    user: String,
}

#[derive(Clone, Debug, Default)]
struct InMemoryCredentialBuilder {
    store: Arc<Mutex<HashMap<EntryKey, Vec<u8>>>>,
}

#[derive(Clone, Debug)]
struct InMemoryCredential {
    key: EntryKey,
    store: Arc<Mutex<HashMap<EntryKey, Vec<u8>>>>,
}

impl CredentialBuilderApi for InMemoryCredentialBuilder {
    fn build(
        &self,
        target: Option<&str>,
        service: &str,
        user: &str,
    ) -> keyring::Result<Box<Credential>> {
        let key = EntryKey {
            target: target.map(|s| s.to_string()),
            service: service.to_string(),
            user: user.to_string(),
        };
        Ok(Box::new(InMemoryCredential {
            key,
            store: Arc::clone(&self.store),
        }))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn persistence(&self) -> CredentialPersistence {
        CredentialPersistence::ProcessOnly
    }
}

impl CredentialApi for InMemoryCredential {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        let mut store = self.store.lock().unwrap();
        store.insert(self.key.clone(), secret.to_vec());
        Ok(())
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        let store = self.store.lock().unwrap();
        store.get(&self.key).cloned().ok_or(KeyringError::NoEntry)
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        let mut store = self.store.lock().unwrap();
        if store.remove(&self.key).is_some() {
            Ok(())
        } else {
            Err(KeyringError::NoEntry)
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn create_project_stores_secret_for_later_use() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Test Project".into(), &pool)?;

    let sk = provenance::load_secret_key(&project.id)?;
    let derived_pub = provenance::public_key_from_secret(&sk);
    assert_eq!(derived_pub, project.pubkey);
    Ok(())
}

#[test]
fn orchestrator_writes_incident_checkpoint_when_budget_fails() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Budget".into(), &pool)?;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "low-budget".into(),
            seed: 1,
            dag_json: "{}".into(),
            token_budget: 5,
        },
    )?;

    let conn = pool.get()?;
    let (kind, incident_json): (String, Option<String>) = conn.query_row(
        "SELECT kind, incident_json FROM checkpoints WHERE run_id = ?1",
        params![run_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    assert_eq!(kind, "Incident");
    let incident_json = incident_json.expect("incident details");
    let incident: serde_json::Value = serde_json::from_str(&incident_json)?;
    assert_eq!(incident["kind"], "budget_exceeded");
    assert_eq!(incident["severity"], "error");
    Ok(())
}

#[test]
fn orchestrator_emits_signed_step_checkpoint_on_success() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Happy".into(), &pool)?;
    let seed = 42_u64;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "happy-path".into(),
            seed,
            dag_json: "{\"hello\":true}".into(),
            token_budget: 50,
        },
    )?;

    let conn = pool.get()?;
    let (
        kind,
        incident_json,
        signature_b64,
        curr_chain,
        prev_chain,
        timestamp,
        inputs_sha,
        outputs_sha,
        usage_tokens,
    ): (String, Option<String>, String, String, String, String, Option<String>, Option<String>, i64) =
        conn.query_row(
            "SELECT kind, incident_json, signature, curr_chain, prev_chain, timestamp, inputs_sha256, outputs_sha256, usage_tokens FROM checkpoints WHERE run_id = ?1",
            params![run_id.clone()],
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
                ))
            },
        )?;

    assert_eq!(kind, "Step");
    assert!(incident_json.is_none());
    assert_eq!(prev_chain, "");
    assert_eq!(usage_tokens, 10);

    let inputs_sha = inputs_sha.expect("inputs sha");
    let outputs_sha = outputs_sha.expect("outputs sha");
    assert_eq!(inputs_sha, provenance::sha256_hex(b"hello"));
    let mut expected_output_input = b"hello".to_vec();
    expected_output_input.extend_from_slice(&seed.to_le_bytes());
    assert_eq!(outputs_sha, provenance::sha256_hex(&expected_output_input));

    let checkpoint_json = serde_json::json!({
        "run_id": run_id,
        "kind": "Step",
        "timestamp": timestamp,
        "inputs_sha256": inputs_sha,
        "outputs_sha256": outputs_sha,
        "incident": serde_json::Value::Null,
        "usage_tokens": usage_tokens as u64,
    });
    let canon = provenance::canonical_json(&checkpoint_json);
    let expected_curr_chain = provenance::sha256_hex(&canon);
    assert_eq!(expected_curr_chain, curr_chain);

    let sig_bytes = STANDARD.decode(signature_b64)?;
    let sig_array: [u8; ed25519_dalek::SIGNATURE_LENGTH] = sig_bytes
        .try_into()
        .map_err(|_| anyhow!("signature has wrong length"))?;
    let signature = Signature::from_bytes(&sig_array);

    let signing_key = provenance::load_secret_key(&project.id)?;
    let verifying_key = signing_key.verifying_key();
    verifying_key.verify(curr_chain.as_bytes(), &signature)?;
    Ok(())
}

#[test]
fn get_policy_returns_default_for_new_project() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Policy Defaults".into(), &pool)?;

    let conn = pool.get()?;
    let policy = policies::get(&conn, &project.id)?;

    assert_eq!(policy, Policy::default());
    Ok(())
}

#[test]
fn update_policy_persists_values() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Policy Persist".into(), &pool)?;

    let desired = Policy {
        allow_network: true,
        budget_tokens: 512,
        budget_usd: 4.25,
        budget_g_co2e: 0.75,
    };

    {
        let conn = pool.get()?;
        policies::upsert(&conn, &project.id, &desired)?;
    }

    let conn = pool.get()?;
    let fetched = policies::get(&conn, &project.id)?;
    assert_eq!(fetched, desired);
    Ok(())
}
