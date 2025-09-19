// src-tauri/src/orchestrator.rs
use crate::{governance, provenance, DbPool};
use anyhow::{anyhow, Context};
use chrono::Utc;
use ed25519_dalek::SigningKey;
use keyring::Error as KeyringError;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use uuid::Uuid;

const STUB_MODEL_ID: &str = "stub-model";
const OLLAMA_HOST: &str = "127.0.0.1:11434";

#[derive(Serialize)]
struct CheckpointBody<'a> {
    run_id: &'a str,
    kind: &'a str, // "Step" or "Incident"
    timestamp: String,
    inputs_sha256: Option<&'a str>,
    outputs_sha256: Option<&'a str>,
    incident: Option<&'a serde_json::Value>,
    usage_tokens: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[derive(Clone, Copy)]
struct CheckpointMessageInput<'a> {
    role: &'a str,
    body: &'a str,
}

struct CheckpointInsert<'a> {
    run_id: &'a str,
    parent_checkpoint_id: Option<&'a str>,
    turn_index: Option<u32>,
    kind: &'a str,
    timestamp: &'a str,
    incident: Option<&'a serde_json::Value>,
    inputs_sha256: Option<&'a str>,
    outputs_sha256: Option<&'a str>,
    prev_chain: &'a str,
    usage_tokens: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    semantic_digest: Option<&'a str>,
    message: Option<CheckpointMessageInput<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunProofMode {
    Exact,
    Concordant,
    Interactive,
}

impl Default for RunProofMode {
    fn default() -> Self {
        RunProofMode::Exact
    }
}

impl RunProofMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunProofMode::Exact => "exact",
            RunProofMode::Concordant => "concordant",
            RunProofMode::Interactive => "interactive",
        }
    }

    pub fn is_concordant(&self) -> bool {
        matches!(self, RunProofMode::Concordant)
    }

    pub fn is_interactive(&self) -> bool {
        matches!(self, RunProofMode::Interactive)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunSpec {
    pub project_id: String,
    pub name: String,
    pub seed: u64,
    pub dag_json: String,
    pub token_budget: u64,
    pub model: String,
    #[serde(default)]
    pub proof_mode: RunProofMode,
    #[serde(default)]
    pub epsilon: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }
}

struct NodeExecution {
    inputs_sha256: Option<String>,
    outputs_sha256: Option<String>,
    semantic_digest: Option<String>,
    usage: TokenUsage,
}

pub struct LlmGeneration {
    pub response: String,
    pub usage: TokenUsage,
}

pub trait LlmClient {
    fn stream_generate(&self, model: &str, prompt: &str) -> anyhow::Result<LlmGeneration>;
}

struct DefaultOllamaClient;

impl DefaultOllamaClient {
    fn new() -> Self {
        Self
    }
}

impl LlmClient for DefaultOllamaClient {
    fn stream_generate(&self, model: &str, prompt: &str) -> anyhow::Result<LlmGeneration> {
        perform_ollama_stream(model, prompt)
    }
}

pub fn replay_llm_generation(spec: &RunSpec) -> anyhow::Result<LlmGeneration> {
    let client = DefaultOllamaClient::new();
    client.stream_generate(&spec.model, &spec.dag_json)
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelEntry {
    name: String,
}

pub fn list_local_models() -> anyhow::Result<Vec<String>> {
    let mut models = fetch_ollama_models().unwrap_or_default();
    if !models.iter().any(|m| m == STUB_MODEL_ID) {
        models.insert(0, STUB_MODEL_ID.to_string());
    }
    if models.is_empty() {
        models.push(STUB_MODEL_ID.to_string());
    }
    models.sort();
    models.dedup();
    Ok(models)
}

fn fetch_ollama_models() -> anyhow::Result<Vec<String>> {
    let request = format!(
        "GET /api/tags HTTP/1.1\r\nHost: {OLLAMA_HOST}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    );

    let mut stream = TcpStream::connect(OLLAMA_HOST)?;
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;
    if !status_line.starts_with("HTTP/1.1 200") {
        return Err(anyhow!(format!(
            "unexpected Ollama tags response: {}",
            status_line.trim()
        )));
    }

    let mut transfer_chunked = false;
    let mut content_length: Option<usize> = None;
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line)?;
        if header_line == "\r\n" || header_line.is_empty() {
            break;
        }
        let lower = header_line.to_ascii_lowercase();
        if lower.contains("transfer-encoding") && lower.contains("chunked") {
            transfer_chunked = true;
        } else if lower.starts_with("content-length") {
            if let Some((_, value)) = header_line.split_once(':') {
                content_length = value.trim().parse::<usize>().ok();
            }
        }
    }

    let mut body = Vec::new();
    if transfer_chunked {
        loop {
            let mut size_line = String::new();
            reader.read_line(&mut size_line)?;
            if size_line.trim().is_empty() {
                continue;
            }
            let size = usize::from_str_radix(size_line.trim(), 16)?;
            if size == 0 {
                // Consume trailing CRLF after terminating chunk
                let mut crlf = [0u8; 2];
                reader.read_exact(&mut crlf)?;
                break;
            }

            let mut chunk = vec![0u8; size];
            reader.read_exact(&mut chunk)?;
            body.extend_from_slice(&chunk);

            let mut crlf = [0u8; 2];
            reader.read_exact(&mut crlf)?;
        }
    } else if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;
        body = buf;
    } else {
        reader.read_to_end(&mut body)?;
    }

    let tags: OllamaTagsResponse = serde_json::from_slice(&body)?;
    let models = tags.models.into_iter().map(|entry| entry.name).collect();
    Ok(models)
}

fn perform_ollama_stream(model: &str, prompt: &str) -> anyhow::Result<LlmGeneration> {
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": true,
    })
    .to_string();

    let request = format!(
        "POST /api/generate HTTP/1.1\r\nHost: {OLLAMA_HOST}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.as_bytes().len(),
        body
    );

    let mut stream = TcpStream::connect(OLLAMA_HOST)?;
    stream.set_read_timeout(Some(Duration::from_secs(120)))?;
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;
    if !status_line.starts_with("HTTP/1.1 200") {
        return Err(anyhow!(format!(
            "unexpected Ollama response: {status_line}"
        )));
    }

    let mut transfer_chunked = false;
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line)?;
        if header_line == "\r\n" || header_line.is_empty() {
            break;
        }
        if header_line
            .to_ascii_lowercase()
            .contains("transfer-encoding")
            && header_line.to_ascii_lowercase().contains("chunked")
        {
            transfer_chunked = true;
        }
    }

    if !transfer_chunked {
        return Err(anyhow!("ollama response was not chunked"));
    }

    let mut response_text = String::new();
    let mut prompt_tokens = 0_u64;
    let mut completion_tokens = 0_u64;

    loop {
        let mut size_line = String::new();
        reader.read_line(&mut size_line)?;
        if size_line.trim().is_empty() {
            continue;
        }
        let size = usize::from_str_radix(size_line.trim(), 16)?;
        if size == 0 {
            break;
        }

        let mut chunk_data = vec![0u8; size];
        reader.read_exact(&mut chunk_data)?;

        // Consume trailing CRLF after chunk
        let mut crlf = [0u8; 2];
        reader.read_exact(&mut crlf)?;

        process_stream_chunk(
            &chunk_data,
            &mut response_text,
            &mut prompt_tokens,
            &mut completion_tokens,
        )?;
    }

    Ok(LlmGeneration {
        response: response_text,
        usage: TokenUsage {
            prompt_tokens,
            completion_tokens,
        },
    })
}

fn process_stream_chunk(
    bytes: &[u8],
    response_text: &mut String,
    prompt_tokens: &mut u64,
    completion_tokens: &mut u64,
) -> anyhow::Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }

    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == b'\n' || bytes[end - 1] == b'\r') {
        end -= 1;
    }
    if end == 0 {
        return Ok(());
    }

    let value: Value = serde_json::from_slice(&bytes[..end])?;
    if let Some(error) = value.get("error").and_then(|v| v.as_str()) {
        return Err(anyhow!(error.to_string()));
    }

    if let Some(text) = value.get("response").and_then(|v| v.as_str()) {
        response_text.push_str(text);
    }

    if value.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
        if let Some(count) = value.get("prompt_eval_count").and_then(|v| v.as_u64()) {
            *prompt_tokens = count;
        }
        if let Some(count) = value.get("eval_count").and_then(|v| v.as_u64()) {
            *completion_tokens = count;
        }
    }

    Ok(())
}

pub fn start_hello_run(pool: &DbPool, spec: RunSpec) -> anyhow::Result<String> {
    let client = DefaultOllamaClient::new();
    start_hello_run_with_client(pool, spec, &client)
}

fn persist_checkpoint(
    conn: &Connection,
    signing_key: &SigningKey,
    params: &CheckpointInsert<'_>,
) -> anyhow::Result<String> {
    let checkpoint_body = CheckpointBody {
        run_id: params.run_id,
        kind: params.kind,
        timestamp: params.timestamp.to_string(),
        inputs_sha256: params.inputs_sha256,
        outputs_sha256: params.outputs_sha256,
        incident: params.incident,
        usage_tokens: params.usage_tokens,
        prompt_tokens: params.prompt_tokens,
        completion_tokens: params.completion_tokens,
    };

    let body_json = serde_json::to_value(&checkpoint_body)?;
    let canonical = provenance::canonical_json(&body_json);
    let curr_chain = provenance::sha256_hex(&[params.prev_chain.as_bytes(), &canonical].concat());
    let signature = provenance::sign_bytes(signing_key, curr_chain.as_bytes());
    let checkpoint_id = Uuid::new_v4().to_string();
    let incident_json = params.incident.map(|value| value.to_string());

    conn.execute(
        "INSERT INTO checkpoints (id, run_id, parent_checkpoint_id, turn_index, kind, incident_json, timestamp, inputs_sha256, outputs_sha256, prev_chain, curr_chain, signature, usage_tokens, semantic_digest, prompt_tokens, completion_tokens) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![
            &checkpoint_id,
            params.run_id,
            params.parent_checkpoint_id,
            params.turn_index.map(|value| value as i64),
            params.kind,
            incident_json.as_deref(),
            params.timestamp,
            params.inputs_sha256,
            params.outputs_sha256,
            params.prev_chain,
            curr_chain,
            signature,
            (params.usage_tokens as i64),
            params.semantic_digest,
            (params.prompt_tokens as i64),
            (params.completion_tokens as i64),
        ],
    )?;

    if let Some(message) = params.message {
        conn.execute(
            "INSERT INTO checkpoint_messages (checkpoint_id, role, body, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![
                &checkpoint_id,
                message.role,
                message.body,
                params.timestamp,
            ],
        )?;
    }

    Ok(checkpoint_id)
}

pub(crate) fn start_hello_run_with_client(
    pool: &DbPool,
    spec: RunSpec,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<String> {
    let conn = pool.get()?;

    let signing_key = ensure_project_signing_key(&conn, &spec.project_id)?;
    let run_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let spec_json = serde_json::to_string(&spec)?;
    let run_kind = spec.proof_mode.as_str();

    if spec.proof_mode.is_concordant() {
        let epsilon = spec
            .epsilon
            .ok_or_else(|| anyhow!("concordant runs require an epsilon"))?;
        if !epsilon.is_finite() || epsilon < 0.0 {
            return Err(anyhow!("epsilon must be a finite, non-negative value"));
        }
    }

    conn.execute(
        "INSERT INTO runs (id, project_id, name, created_at, kind, spec_json) VALUES (?1,?2,?3,?4,?5,?6)",
        params![&run_id, &spec.project_id, &spec.name, &now, run_kind, &spec_json],
    )?;

    let execution = execute_node(&spec, llm_client)?;
    let total_usage = execution.usage.total();
    let prompt_tokens = execution.usage.prompt_tokens;
    let completion_tokens = execution.usage.completion_tokens;
    let budget_check = governance::enforce_budget(spec.token_budget, total_usage);

    let prev_chain = "";
    let mut incident_value: Option<serde_json::Value> = None;
    let (kind, inputs_sha, outputs_sha) = match budget_check {
        Ok(_) => (
            "Step",
            execution.inputs_sha256.as_deref(),
            execution.outputs_sha256.as_deref(),
        ),
        Err(incident) => {
            incident_value = Some(serde_json::to_value(&incident)?);
            ("Incident", None, None)
        }
    };

    let semantic_digest = if spec.proof_mode.is_concordant() {
        Some(
            execution
                .semantic_digest
                .clone()
                .ok_or_else(|| anyhow!("semantic digest missing for concordant run"))?,
        )
    } else {
        None
    };
    let checkpoint_insert = CheckpointInsert {
        run_id: &run_id,
        parent_checkpoint_id: None,
        turn_index: None,
        kind,
        timestamp: &now,
        incident: incident_value.as_ref(),
        inputs_sha256: inputs_sha,
        outputs_sha256: outputs_sha,
        prev_chain,
        usage_tokens: total_usage,
        prompt_tokens,
        completion_tokens,
        semantic_digest: semantic_digest.as_deref(),
        message: None,
    };

    persist_checkpoint(&conn, &signing_key, &checkpoint_insert)?;

    Ok(run_id)
}

fn execute_node(spec: &RunSpec, llm_client: &dyn LlmClient) -> anyhow::Result<NodeExecution> {
    if spec.model == STUB_MODEL_ID {
        Ok(execute_stub_node(spec))
    } else {
        execute_llm_run(spec, llm_client)
    }
}

fn stub_output_bytes(seed: u64) -> Vec<u8> {
    let mut output = b"hello".to_vec();
    output.extend_from_slice(&seed.to_le_bytes());
    output
}

fn execute_stub_node(spec: &RunSpec) -> NodeExecution {
    let output_bytes = stub_output_bytes(spec.seed);
    let outputs_hex = provenance::sha256_hex(&output_bytes);
    let inputs_hex = provenance::sha256_hex(b"hello");
    let semantic_source = hex::encode(&output_bytes);
    let semantic_digest = provenance::semantic_digest(&semantic_source);

    NodeExecution {
        inputs_sha256: Some(inputs_hex),
        outputs_sha256: Some(outputs_hex),
        semantic_digest: Some(semantic_digest),
        usage: TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 10,
        },
    }
}

fn execute_llm_run(spec: &RunSpec, llm_client: &dyn LlmClient) -> anyhow::Result<NodeExecution> {
    let prompt = spec.dag_json.clone();
    let generation = llm_client.stream_generate(&spec.model, &prompt)?;
    let inputs_hex = provenance::sha256_hex(prompt.as_bytes());
    let outputs_hex = provenance::sha256_hex(generation.response.as_bytes());
    let semantic_digest = provenance::semantic_digest(&generation.response);

    Ok(NodeExecution {
        inputs_sha256: Some(inputs_hex),
        outputs_sha256: Some(outputs_hex),
        semantic_digest: Some(semantic_digest),
        usage: generation.usage,
    })
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
    use std::sync::{Mutex, Once};

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
            model: STUB_MODEL_ID.to_string(),
            proof_mode: RunProofMode::Exact,
            epsilon: None,
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
            prompt_tokens_db,
            completion_tokens_db,
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
            i64,
            i64,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT kind, timestamp, inputs_sha256, outputs_sha256, prev_chain, curr_chain, signature, usage_tokens, prompt_tokens, completion_tokens, incident_json, semantic_digest FROM checkpoints WHERE run_id = ?1",
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
                        row.get(10)?,
                        row.get(11)?,
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
        let prompt_tokens = u64::try_from(prompt_tokens_db)?;
        let completion_tokens = u64::try_from(completion_tokens_db)?;
        assert_eq!(usage_tokens, 10);
        assert_eq!(prompt_tokens, 0);
        assert_eq!(completion_tokens, 10);

        let checkpoint_body = CheckpointBody {
            run_id: &run_id,
            kind: &kind,
            timestamp: timestamp.clone(),
            inputs_sha256: inputs_sha.as_deref(),
            outputs_sha256: outputs_sha.as_deref(),
            incident: None,
            usage_tokens,
            prompt_tokens,
            completion_tokens,
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
            model: STUB_MODEL_ID.to_string(),
            proof_mode: RunProofMode::Exact,
            epsilon: None,
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

    struct RecordingLlmClient {
        expected_model: String,
        expected_prompt: String,
        response: String,
        usage: TokenUsage,
        calls: Mutex<usize>,
    }

    impl RecordingLlmClient {
        fn new(model: String, prompt: String, response: String, usage: TokenUsage) -> Self {
            Self {
                expected_model: model,
                expected_prompt: prompt,
                response,
                usage,
                calls: Mutex::new(0),
            }
        }
    }

    impl LlmClient for RecordingLlmClient {
        fn stream_generate(&self, model: &str, prompt: &str) -> anyhow::Result<LlmGeneration> {
            assert_eq!(model, self.expected_model);
            assert_eq!(prompt, self.expected_prompt);
            let mut calls = self.calls.lock().expect("lock call count");
            *calls += 1;
            Ok(LlmGeneration {
                response: self.response.clone(),
                usage: self.usage,
            })
        }
    }

    #[test]
    fn start_hello_run_records_llm_usage() -> Result<()> {
        init_keychain_backend();

        let manager = SqliteConnectionManager::memory();
        let pool: Pool<SqliteConnectionManager> = Pool::builder().max_size(1).build(manager)?;
        {
            let mut conn = pool.get()?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            store::migrate_db(&mut conn)?;
        }

        let project_id = "proj-llm";
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
                params![project_id, "LLM Project", created_at, pubkey],
            )?;
        }

        provenance::store_secret_key(project_id, &keypair.secret_key_b64)?;

        let prompt_json = "{\"prompt\":\"Say hello\"}".to_string();
        let spec = RunSpec {
            project_id: project_id.to_string(),
            name: "llm-run".to_string(),
            seed: 5,
            dag_json: prompt_json.clone(),
            token_budget: 10_000,
            model: "llama3".to_string(),
            proof_mode: RunProofMode::Exact,
            epsilon: None,
        };

        let mock_client = RecordingLlmClient::new(
            spec.model.clone(),
            prompt_json.clone(),
            "Hello from mock".to_string(),
            TokenUsage {
                prompt_tokens: 12,
                completion_tokens: 8,
            },
        );

        let run_id = start_hello_run_with_client(&pool, spec.clone(), &mock_client)?;

        assert_eq!(*mock_client.calls.lock().unwrap(), 1);

        let conn = pool.get()?;
        let stored_spec: RunSpec = conn.query_row(
            "SELECT spec_json FROM runs WHERE id = ?1",
            params![&run_id],
            |row| {
                let payload: String = row.get(0)?;
                Ok(serde_json::from_str(&payload)?)
            },
        )?;
        assert_eq!(stored_spec.model, "llama3");

        let (
            inputs_sha,
            outputs_sha,
            usage_tokens_db,
            prompt_tokens_db,
            completion_tokens_db,
        ): (Option<String>, Option<String>, i64, i64, i64) = conn.query_row(
            "SELECT inputs_sha256, outputs_sha256, usage_tokens, prompt_tokens, completion_tokens FROM checkpoints WHERE run_id = ?1",
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

        let expected_input_sha = provenance::sha256_hex(prompt_json.as_bytes());
        assert_eq!(inputs_sha.as_deref(), Some(expected_input_sha.as_str()));
        let expected_output_sha = provenance::sha256_hex(b"Hello from mock");
        assert_eq!(outputs_sha.as_deref(), Some(expected_output_sha.as_str()));

        assert_eq!(usage_tokens_db, 20);
        assert_eq!(prompt_tokens_db, 12);
        assert_eq!(completion_tokens_db, 8);

        let signature: String = conn.query_row(
            "SELECT signature FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?;
        let curr_chain: String = conn.query_row(
            "SELECT curr_chain FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?;
        let sig_bytes = STANDARD.decode(signature)?;
        let sig_array: [u8; ed25519_dalek::SIGNATURE_LENGTH] = sig_bytes
            .try_into()
            .map_err(|_| anyhow!("signature length mismatch"))?;
        let signature = ed25519_dalek::Signature::from_bytes(&sig_array);
        signing_key
            .verifying_key()
            .verify(curr_chain.as_bytes(), &signature)?;

        Ok(())
    }
}
