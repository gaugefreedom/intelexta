// src-tauri/src/orchestrator.rs
use crate::{governance, provenance, store, DbPool};
use anyhow::{anyhow, Context};
use chrono::Utc;
use ed25519_dalek::SigningKey;
use keyring::Error as KeyringError;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::TryFrom;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::ops::Deref;
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
    checkpoint_config_id: Option<&'a str>,
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
    prompt_payload: Option<&'a str>,
    output_payload: Option<&'a str>,
    message: Option<CheckpointMessageInput<'a>>,
}

struct PersistedCheckpoint {
    id: String,
    curr_chain: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

impl RunProofMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunProofMode::Exact => "exact",
            RunProofMode::Concordant => "concordant",
        }
    }

    pub fn is_concordant(&self) -> bool {
        matches!(self, RunProofMode::Concordant)
    }
}

impl TryFrom<&str> for RunProofMode {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "exact" => Ok(RunProofMode::Exact),
            "concordant" => Ok(RunProofMode::Concordant),
            "interactive" => Ok(RunProofMode::Exact),
            other => Err(anyhow!(format!("unsupported run proof mode: {other}"))),
        }
    }
}

fn default_checkpoint_type() -> String {
    "Step".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCheckpointTemplate {
    pub model: String,
    pub prompt: String,
    pub token_budget: u64,
    #[serde(default)]
    pub order_index: Option<i64>,
    #[serde(default = "default_checkpoint_type")]
    pub checkpoint_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSpec {
    pub project_id: String,
    pub name: String,
    pub seed: u64,
    pub token_budget: u64,
    pub model: String,
    pub checkpoints: Vec<RunCheckpointTemplate>,
    #[serde(default)]
    pub proof_mode: RunProofMode,
    #[serde(default)]
    pub epsilon: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCheckpointConfig {
    pub id: String,
    pub run_id: String,
    pub order_index: i64,
    pub checkpoint_type: String,
    pub model: String,
    pub prompt: String,
    pub token_budget: u64,
}

impl RunCheckpointConfig {
    pub fn is_interactive_chat(&self) -> bool {
        self.checkpoint_type.eq_ignore_ascii_case("InteractiveChat")
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredRun {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub proof_mode: RunProofMode,
    pub seed: u64,
    pub token_budget: u64,
    pub default_model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
    pub checkpoints: Vec<RunCheckpointConfig>,
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
    prompt_payload: Option<String>,
    output_payload: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmGeneration {
    pub response: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunCostEstimates {
    pub estimated_tokens: u64,
    pub estimated_usd: f64,
    pub estimated_g_co2e: f64,
    pub budget_tokens: u64,
    pub budget_usd: f64,
    pub budget_g_co2e: f64,
    pub exceeds_tokens: bool,
    pub exceeds_usd: bool,
    pub exceeds_g_co2e: bool,
}

impl RunCostEstimates {
    fn exceeds_any(&self) -> bool {
        self.exceeds_tokens || self.exceeds_usd || self.exceeds_g_co2e
    }
}

fn sum_token_budgets(configs: &[RunCheckpointConfig]) -> u64 {
    configs
        .iter()
        .filter(|cfg| !cfg.is_interactive_chat())
        .fold(0u64, |acc, cfg| acc.saturating_add(cfg.token_budget))
}

fn estimate_costs_with_policy(
    policy: &store::policies::Policy,
    projected_tokens: u64,
) -> RunCostEstimates {
    let token_budget = policy.budget_tokens;
    let estimated_tokens = projected_tokens;
    let tokens_f64 = estimated_tokens as f64;

    let usd_per_token = if token_budget > 0 {
        policy.budget_usd / token_budget as f64
    } else {
        0.0
    };
    let co2_per_token = if token_budget > 0 {
        policy.budget_g_co2e / token_budget as f64
    } else {
        0.0
    };

    let estimated_usd = usd_per_token * tokens_f64;
    let estimated_g_co2e = co2_per_token * tokens_f64;

    RunCostEstimates {
        estimated_tokens,
        estimated_usd,
        estimated_g_co2e,
        budget_tokens: token_budget,
        budget_usd: policy.budget_usd,
        budget_g_co2e: policy.budget_g_co2e,
        exceeds_tokens: estimated_tokens > token_budget,
        exceeds_usd: estimated_usd > policy.budget_usd,
        exceeds_g_co2e: estimated_g_co2e > policy.budget_g_co2e,
    }
}

#[cfg(feature = "interactive")]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitTurnOutcome {
    pub human_checkpoint_id: String,
    pub ai_checkpoint_id: String,
    pub ai_response: String,
    pub usage: TokenUsage,
}

pub trait LlmClient {
    fn stream_generate(&self, model: &str, prompt: &str) -> anyhow::Result<LlmGeneration>;
}

fn sanitize_payload(payload: &str) -> String {
    const MAX_CHARS: usize = 65_536;
    let mut result = String::new();
    let mut count = 0usize;
    let mut truncated = false;

    for ch in payload.chars() {
        if ch.is_control() && !matches!(ch, '\n' | '\r' | '\t') {
            continue;
        }
        if count >= MAX_CHARS {
            truncated = true;
            break;
        }
        result.push(ch);
        count += 1;
    }

    if truncated {
        if !result.ends_with('\n') {
            result.push('\n');
        }
        result.push_str("…[truncated]");
    }

    result
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

pub fn replay_llm_generation(model: &str, prompt: &str) -> anyhow::Result<LlmGeneration> {
    let client = DefaultOllamaClient::new();
    client.stream_generate(model, prompt)
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

pub fn create_run(pool: &DbPool, spec: RunSpec) -> anyhow::Result<String> {
    if spec.proof_mode.is_concordant() {
        let epsilon = spec
            .epsilon
            .ok_or_else(|| anyhow!("concordant runs require an epsilon"))?;
        if !epsilon.is_finite() || epsilon < 0.0 {
            return Err(anyhow!("epsilon must be a finite, non-negative value"));
        }
    }

    let mut conn = pool.get()?;
    ensure_project_signing_key(&conn, &spec.project_id)?;

    let run_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let run_kind = spec.proof_mode.as_str();
    let spec_json = serde_json::to_string(&spec)?;

    {
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO runs (id, project_id, name, created_at, kind, spec_json, sampler_json, seed, epsilon, token_budget, default_model) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                &run_id,
                &spec.project_id,
                &spec.name,
                &now,
                run_kind,
                &spec_json,
                Option::<String>::None,
                (spec.seed as i64),
                spec.epsilon,
                (spec.token_budget as i64),
                &spec.model,
            ],
        )?;

        for (index, template) in spec.checkpoints.iter().enumerate() {
            let checkpoint_id = Uuid::new_v4().to_string();
            let order_index = template.order_index.unwrap_or(index as i64);
            tx.execute(
                "INSERT INTO run_checkpoints (id, run_id, order_index, checkpoint_type, model, prompt, token_budget) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    &checkpoint_id,
                    &run_id,
                    order_index,
                    &template.checkpoint_type,
                    &template.model,
                    &template.prompt,
                    (template.token_budget as i64),
                ],
            )?;
        }

        tx.commit()?;
    }

    Ok(run_id)
}

pub fn start_hello_run(pool: &DbPool, spec: RunSpec) -> anyhow::Result<String> {
    let client = DefaultOllamaClient::new();
    start_hello_run_with_client(pool, spec, &client)
}

fn persist_checkpoint(
    conn: &Connection,
    signing_key: &SigningKey,
    params: &CheckpointInsert<'_>,
) -> anyhow::Result<PersistedCheckpoint> {
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
        "INSERT INTO checkpoints (id, run_id, checkpoint_config_id, parent_checkpoint_id, turn_index, kind, incident_json, timestamp, inputs_sha256, outputs_sha256, prev_chain, curr_chain, signature, usage_tokens, semantic_digest, prompt_tokens, completion_tokens) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
        params![
            &checkpoint_id,
            params.run_id,
            params.checkpoint_config_id,
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

    if params.prompt_payload.is_some() || params.output_payload.is_some() {
        conn.execute(
            "INSERT INTO checkpoint_payloads (checkpoint_id, prompt_payload, output_payload) VALUES (?1, ?2, ?3) ON CONFLICT(checkpoint_id) DO UPDATE SET prompt_payload = excluded.prompt_payload, output_payload = excluded.output_payload, updated_at = CURRENT_TIMESTAMP",
            params![
                &checkpoint_id,
                params.prompt_payload,
                params.output_payload,
            ],
        )?;
    }

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

    Ok(PersistedCheckpoint {
        id: checkpoint_id,
        curr_chain,
    })
}

#[cfg(feature = "interactive")]
fn sum_checkpoint_token_usage(
    conn: &Connection,
    run_id: &str,
    checkpoint_config_id: Option<&str>,
) -> anyhow::Result<(u64, u64)> {
    let (prompt_total, completion_total): (i64, i64) = match checkpoint_config_id {
        Some(config_id) => conn.query_row(
            "SELECT COALESCE(SUM(prompt_tokens), 0), COALESCE(SUM(completion_tokens), 0) FROM checkpoints WHERE run_id = ?1 AND checkpoint_config_id = ?2",
            params![run_id, config_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?,
        None => conn.query_row(
            "SELECT COALESCE(SUM(prompt_tokens), 0), COALESCE(SUM(completion_tokens), 0) FROM checkpoints WHERE run_id = ?1",
            params![run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?,
    };

    let prompt = prompt_total.max(0) as u64;
    let completion = completion_total.max(0) as u64;
    Ok((prompt, completion))
}

fn load_run_checkpoint_configs(
    conn: &Connection,
    run_id: &str,
) -> anyhow::Result<Vec<RunCheckpointConfig>> {
    let mut stmt = conn.prepare(
        "SELECT id, order_index, checkpoint_type, model, prompt, token_budget FROM run_checkpoints WHERE run_id = ?1 ORDER BY order_index ASC",
    )?;
    let rows = stmt.query_map(params![run_id], |row| {
        let token_budget: i64 = row.get(5)?;
        Ok(RunCheckpointConfig {
            id: row.get(0)?,
            run_id: run_id.to_string(),
            order_index: row.get(1)?,
            checkpoint_type: row.get(2)?,
            model: row.get(3)?,
            prompt: row.get(4)?,
            token_budget: token_budget.max(0) as u64,
        })
    })?;

    let mut configs = Vec::new();
    for row in rows {
        configs.push(row?);
    }

    Ok(configs)
}

pub fn estimate_run_cost(conn: &Connection, run_id: &str) -> anyhow::Result<RunCostEstimates> {
    let stored_run = load_stored_run(conn, run_id)?;
    let policy = store::policies::get(conn, &stored_run.project_id)?;
    let projected_tokens = sum_token_budgets(&stored_run.checkpoints);
    Ok(estimate_costs_with_policy(&policy, projected_tokens))
}

fn load_checkpoint_config_by_id(
    conn: &Connection,
    checkpoint_id: &str,
) -> anyhow::Result<Option<RunCheckpointConfig>> {
    let row: Option<(String, i64, String, String, String, i64)> = conn
        .query_row(
            "SELECT run_id, order_index, checkpoint_type, model, prompt, token_budget FROM run_checkpoints WHERE id = ?1",
            params![checkpoint_id],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            )),
        )
        .optional()?;

    let Some((run_id, order_index, checkpoint_type, model, prompt, token_budget_raw)) = row else {
        return Ok(None);
    };

    Ok(Some(RunCheckpointConfig {
        id: checkpoint_id.to_string(),
        run_id,
        order_index,
        checkpoint_type,
        model,
        prompt,
        token_budget: token_budget_raw.max(0) as u64,
    }))
}

pub fn load_stored_run(conn: &Connection, run_id: &str) -> anyhow::Result<StoredRun> {
    let row: Option<(String, String, String, i64, Option<f64>, i64, String)> = conn
        .query_row(
            "SELECT project_id, name, kind, seed, epsilon, token_budget, default_model FROM runs WHERE id = ?1",
            params![run_id],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            )),
        )
        .optional()?;

    let (project_id, name, kind, seed_raw, epsilon, token_budget_raw, default_model) =
        row.ok_or_else(|| anyhow!(format!("run {run_id} not found")))?;
    let proof_mode = RunProofMode::try_from(kind.as_str())?;
    let seed = seed_raw.max(0) as u64;
    let token_budget = token_budget_raw.max(0) as u64;
    let checkpoints = load_run_checkpoint_configs(conn, run_id)?;

    Ok(StoredRun {
        id: run_id.to_string(),
        project_id,
        name,
        proof_mode,
        seed,
        token_budget,
        default_model,
        epsilon,
        checkpoints,
    })
}

struct LastCheckpointInfo {
    id: String,
    curr_chain: String,
    turn_index: Option<u32>,
}

fn load_last_checkpoint(
    conn: &Connection,
    run_id: &str,
) -> anyhow::Result<Option<LastCheckpointInfo>> {
    let row = conn
        .query_row(
            "SELECT id, curr_chain, turn_index FROM checkpoints WHERE run_id = ?1 ORDER BY COALESCE(turn_index, -1) DESC, timestamp DESC LIMIT 1",
            params![run_id],
            |row| {
                let turn_index = row
                    .get::<_, Option<i64>>(2)?
                    .map(|value| value.max(0) as u32);
                Ok(LastCheckpointInfo {
                    id: row.get(0)?,
                    curr_chain: row.get(1)?,
                    turn_index,
                })
            },
        )
        .optional()?;

    Ok(row)
}

#[cfg(feature = "interactive")]
fn load_last_checkpoint_for_config(
    conn: &Connection,
    run_id: &str,
    checkpoint_config_id: &str,
) -> anyhow::Result<Option<LastCheckpointInfo>> {
    let row = conn
        .query_row(
            "SELECT id, curr_chain, turn_index FROM checkpoints WHERE run_id = ?1 AND checkpoint_config_id = ?2 ORDER BY COALESCE(turn_index, -1) DESC, timestamp DESC LIMIT 1",
            params![run_id, checkpoint_config_id],
            |row| {
                let turn_index = row
                    .get::<_, Option<i64>>(2)?
                    .map(|value| value.max(0) as u32);
                Ok(LastCheckpointInfo {
                    id: row.get(0)?,
                    curr_chain: row.get(1)?,
                    turn_index,
                })
            },
        )
        .optional()?;

    Ok(row)
}

#[cfg(feature = "interactive")]
fn load_interactive_messages(
    conn: &Connection,
    run_id: &str,
    checkpoint_config_id: &str,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT m.role, m.body FROM checkpoints c JOIN checkpoint_messages m ON m.checkpoint_id = c.id WHERE c.run_id = ?1 AND c.checkpoint_config_id = ?2 ORDER BY COALESCE(c.turn_index, -1) ASC, c.timestamp ASC",
    )?;

    let rows = stmt.query_map(params![run_id, checkpoint_config_id], |row| {
        let role: String = row.get(0)?;
        let body: String = row.get(1)?;
        Ok((role, body))
    })?;

    let mut messages = Vec::new();
    for row in rows {
        messages.push(row?);
    }

    Ok(messages)
}

#[cfg(feature = "interactive")]
fn build_interactive_prompt(
    template_prompt: &str,
    transcript: &[(String, String)],
    user_input: &str,
) -> String {
    let mut prompt = String::new();
    let trimmed_template = template_prompt.trim();
    if !trimmed_template.is_empty() {
        prompt.push_str(trimmed_template);
        prompt.push_str("\n\n");
    }

    for (role, body) in transcript {
        let normalized_role = role.trim();
        let normalized_body = body.trim();
        prompt.push_str(normalized_role);
        prompt.push_str(": ");
        prompt.push_str(normalized_body);
        prompt.push('\n');
    }

    prompt.push_str("Human: ");
    prompt.push_str(user_input.trim());
    prompt.push_str("\nAI:");

    prompt
}

#[cfg(feature = "interactive")]
pub fn submit_interactive_checkpoint_turn(
    pool: &DbPool,
    run_id: &str,
    checkpoint_config_id: &str,
    prompt_text: &str,
) -> anyhow::Result<SubmitTurnOutcome> {
    let client = DefaultOllamaClient::new();
    submit_interactive_checkpoint_turn_with_client(
        pool,
        run_id,
        checkpoint_config_id,
        prompt_text,
        &client,
    )
}

#[cfg(feature = "interactive")]
pub(crate) fn submit_interactive_checkpoint_turn_with_client(
    pool: &DbPool,
    run_id: &str,
    checkpoint_config_id: &str,
    prompt_text: &str,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<SubmitTurnOutcome> {
    let trimmed_prompt = prompt_text.trim();
    if trimmed_prompt.is_empty() {
        return Err(anyhow!("prompt text is required"));
    }

    let mut conn = pool.get()?;

    let stored_run = load_stored_run(&conn, run_id)?;
    let config = match load_checkpoint_config_by_id(&conn, checkpoint_config_id)? {
        Some(cfg) => {
            if cfg.run_id != run_id {
                return Err(anyhow!(
                    "checkpoint configuration does not belong to the specified run"
                ));
            }
            cfg
        }
        None => {
            return Err(anyhow!(format!(
                "checkpoint configuration {checkpoint_config_id} not found"
            )))
        }
    };

    if !config.is_interactive_chat() {
        return Err(anyhow!(
            "interactive turns are only supported for InteractiveChat checkpoints"
        ));
    }

    let transcript = load_interactive_messages(&conn, run_id, checkpoint_config_id)?;
    let llm_prompt = build_interactive_prompt(&config.prompt, &transcript, trimmed_prompt);

    let signing_key = ensure_project_signing_key(&conn, &stored_run.project_id)?;

    let LlmGeneration { response, usage } =
        llm_client.stream_generate(&config.model, &llm_prompt)?;
    let sanitized_llm_prompt = sanitize_payload(&llm_prompt);
    let sanitized_response = sanitize_payload(&response);

    let tx = conn.transaction()?;

    let (prior_prompt, prior_completion) =
        sum_checkpoint_token_usage(&tx, run_id, Some(checkpoint_config_id))?;
    let projected_prompt_total = prior_prompt
        .checked_add(usage.prompt_tokens)
        .ok_or_else(|| anyhow!("prompt token total overflow"))?;
    let projected_completion_total = prior_completion
        .checked_add(usage.completion_tokens)
        .ok_or_else(|| anyhow!("completion token total overflow"))?;
    let projected_usage_total = projected_prompt_total
        .checked_add(projected_completion_total)
        .ok_or_else(|| anyhow!("usage token total overflow"))?;

    if let Err(incident) = governance::enforce_budget(config.token_budget, projected_usage_total) {
        let incident_json = serde_json::to_string(&incident)?;
        return Err(anyhow!(format!(
            "turn would exceed checkpoint token budget: {incident_json}"
        )));
    }

    let last_checkpoint = load_last_checkpoint(&tx, run_id)?;
    let parent_checkpoint_id_owned = last_checkpoint.as_ref().map(|info| info.id.clone());
    let prev_chain_owned = last_checkpoint.as_ref().map(|info| info.curr_chain.clone());
    let parent_checkpoint_ref = parent_checkpoint_id_owned
        .as_ref()
        .map(|value| value.as_str());
    let prev_chain_ref = prev_chain_owned.as_deref().unwrap_or("");

    let config_last_checkpoint =
        load_last_checkpoint_for_config(&tx, run_id, checkpoint_config_id)?;
    let last_turn_index = config_last_checkpoint
        .as_ref()
        .and_then(|info| info.turn_index);
    let human_turn_index = match last_turn_index {
        Some(value) => value
            .checked_add(1)
            .ok_or_else(|| anyhow!("turn index overflow"))?,
        None => 0,
    };

    let human_timestamp = Utc::now().to_rfc3339();
    let human_insert = CheckpointInsert {
        run_id,
        checkpoint_config_id: Some(checkpoint_config_id),
        parent_checkpoint_id: parent_checkpoint_ref,
        turn_index: Some(human_turn_index),
        kind: "Step",
        timestamp: &human_timestamp,
        incident: None,
        inputs_sha256: None,
        outputs_sha256: None,
        prev_chain: prev_chain_ref,
        usage_tokens: 0,
        prompt_tokens: 0,
        completion_tokens: 0,
        semantic_digest: None,
        prompt_payload: None,
        output_payload: None,
        message: Some(CheckpointMessageInput {
            role: "human",
            body: trimmed_prompt,
        }),
    };
    let human_persisted = persist_checkpoint(&tx, &signing_key, &human_insert)?;

    let human_checkpoint_id = human_persisted.id.clone();
    let human_curr_chain = human_persisted.curr_chain.clone();

    let ai_turn_index = human_turn_index
        .checked_add(1)
        .ok_or_else(|| anyhow!("turn index overflow"))?;
    let ai_timestamp = Utc::now().to_rfc3339();
    let prompt_sha = provenance::sha256_hex(llm_prompt.as_bytes());
    let response_sha = provenance::sha256_hex(response.as_bytes());
    let usage_tokens = usage
        .prompt_tokens
        .checked_add(usage.completion_tokens)
        .ok_or_else(|| anyhow!("usage token overflow"))?;
    let ai_insert = CheckpointInsert {
        run_id,
        checkpoint_config_id: Some(checkpoint_config_id),
        parent_checkpoint_id: Some(human_checkpoint_id.as_str()),
        turn_index: Some(ai_turn_index),
        kind: "Step",
        timestamp: &ai_timestamp,
        incident: None,
        inputs_sha256: Some(prompt_sha.as_str()),
        outputs_sha256: Some(response_sha.as_str()),
        prev_chain: human_curr_chain.as_str(),
        usage_tokens,
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        semantic_digest: None,
        prompt_payload: Some(sanitized_llm_prompt.as_str()),
        output_payload: Some(sanitized_response.as_str()),
        message: Some(CheckpointMessageInput {
            role: "ai",
            body: &response,
        }),
    };
    let ai_persisted = persist_checkpoint(&tx, &signing_key, &ai_insert)?;

    tx.commit()?;

    Ok(SubmitTurnOutcome {
        human_checkpoint_id,
        ai_checkpoint_id: ai_persisted.id,
        ai_response: response,
        usage,
    })
}

#[cfg(feature = "interactive")]
pub fn finalize_interactive_checkpoint(
    pool: &DbPool,
    run_id: &str,
    checkpoint_config_id: &str,
) -> anyhow::Result<()> {
    let mut conn = pool.get()?;

    let config = match load_checkpoint_config_by_id(&conn, checkpoint_config_id)? {
        Some(cfg) => {
            if cfg.run_id != run_id {
                return Err(anyhow!(
                    "checkpoint configuration does not belong to the specified run"
                ));
            }
            cfg
        }
        None => {
            return Err(anyhow!(format!(
                "checkpoint configuration {checkpoint_config_id} not found"
            )))
        }
    };

    if !config.is_interactive_chat() {
        return Err(anyhow!(
            "finalization is only supported for InteractiveChat checkpoints"
        ));
    }

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM checkpoints WHERE run_id = ?1 AND checkpoint_config_id = ?2",
        params![run_id, checkpoint_config_id],
        |row| row.get(0),
    )?;

    if count == 0 {
        return Err(anyhow!(
            "interactive checkpoint cannot be finalized without any recorded turns"
        ));
    }

    Ok(())
}

pub(crate) fn start_hello_run_with_client(
    pool: &DbPool,
    spec: RunSpec,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<String> {
    if spec.checkpoints.is_empty() {
        return Err(anyhow!(
            "run requires at least one checkpoint configuration"
        ));
    }

    let run_id = create_run(pool, spec)?;
    start_run_with_client(pool, &run_id, llm_client)?;

    Ok(run_id)
}

pub fn start_run(pool: &DbPool, run_id: &str) -> anyhow::Result<()> {
    let client = DefaultOllamaClient::new();
    start_run_with_client(pool, run_id, &client)
}

pub(crate) fn start_run_with_client(
    pool: &DbPool,
    run_id: &str,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    let stored_run = load_stored_run(&conn, run_id)?;

    if stored_run.checkpoints.is_empty() {
        return Err(anyhow!(format!(
            "run {run_id} has no configured checkpoints"
        )));
    }

    let existing_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM checkpoints WHERE run_id = ?1",
        params![run_id],
        |row| row.get(0),
    )?;
    if existing_count > 0 {
        return Err(anyhow!(format!(
            "run {run_id} already has persisted checkpoints; reopen or clone before re-running"
        )));
    }

    let tx = conn.transaction()?;
    let signing_key = ensure_project_signing_key(&tx, &stored_run.project_id)?;
    let policy = store::policies::get(tx.deref(), &stored_run.project_id)?;
    let mut prev_chain = String::new();
    let mut cumulative_usage_tokens: u64 = 0;

    for (index, config) in stored_run.checkpoints.iter().enumerate() {
        if config.is_interactive_chat() {
            continue;
        }

        let timestamp = Utc::now().to_rfc3339();

        let projected_remaining_tokens = sum_token_budgets(&stored_run.checkpoints[index..]);
        let projected_total_tokens =
            cumulative_usage_tokens.saturating_add(projected_remaining_tokens);
        let projected_costs = estimate_costs_with_policy(&policy, projected_total_tokens);

        if projected_costs.exceeds_any() {
            let mut issues = Vec::new();
            if projected_costs.exceeds_tokens {
                issues.push(format!(
                    "tokens {} > {}",
                    projected_costs.estimated_tokens, projected_costs.budget_tokens
                ));
            }
            if projected_costs.exceeds_usd {
                issues.push(format!(
                    "USD {:.2} > {:.2}",
                    projected_costs.estimated_usd, projected_costs.budget_usd
                ));
            }
            if projected_costs.exceeds_g_co2e {
                issues.push(format!(
                    "CO₂ {:.2} g > {:.2} g",
                    projected_costs.estimated_g_co2e, projected_costs.budget_g_co2e
                ));
            }

            let summary = issues.join(", ");
            let incident = governance::Incident {
                kind: "budget_projection_exceeded".into(),
                severity: "error".into(),
                details: format!(
                    "Projected costs exceed policy budgets before executing checkpoint {} ({}): {}.",
                    config.id, config.checkpoint_type, summary
                ),
            };
            let incident_value = serde_json::to_value(&incident)?;

            let checkpoint_insert = CheckpointInsert {
                run_id,
                checkpoint_config_id: Some(config.id.as_str()),
                parent_checkpoint_id: None,
                turn_index: None,
                kind: "Incident",
                timestamp: &timestamp,
                incident: Some(&incident_value),
                inputs_sha256: None,
                outputs_sha256: None,
                prev_chain: prev_chain.as_str(),
                usage_tokens: 0,
                prompt_tokens: 0,
                completion_tokens: 0,
                semantic_digest: None,
                prompt_payload: None,
                output_payload: None,
                message: None,
            };

            let persisted = persist_checkpoint(&tx, &signing_key, &checkpoint_insert)?;
            prev_chain = persisted.curr_chain;
            break;
        }

        let execution = execute_checkpoint(config, stored_run.seed, llm_client)?;
        let total_usage = execution.usage.total();
        cumulative_usage_tokens = cumulative_usage_tokens.saturating_add(total_usage);
        let prompt_tokens = execution.usage.prompt_tokens;
        let completion_tokens = execution.usage.completion_tokens;
        let mut incident_value: Option<serde_json::Value> = None;

        let budget_outcome = governance::enforce_budget(config.token_budget, total_usage);

        let (kind, inputs_sha, outputs_sha, semantic_digest) =
            match budget_outcome {
                Ok(_) => {
                    let semantic =
                        if stored_run.proof_mode.is_concordant() {
                            Some(execution.semantic_digest.clone().ok_or_else(|| {
                                anyhow!("semantic digest missing for concordant run")
                            })?)
                        } else {
                            None
                        };
                    (
                        "Step",
                        execution.inputs_sha256.as_deref(),
                        execution.outputs_sha256.as_deref(),
                        semantic,
                    )
                }
                Err(incident) => {
                    incident_value = Some(serde_json::to_value(&incident)?);
                    ("Incident", None, None, None)
                }
            };

        let checkpoint_insert = CheckpointInsert {
            run_id,
            checkpoint_config_id: Some(config.id.as_str()),
            parent_checkpoint_id: None,
            turn_index: None,
            kind,
            timestamp: &timestamp,
            incident: incident_value.as_ref(),
            inputs_sha256: inputs_sha,
            outputs_sha256: outputs_sha,
            prev_chain: prev_chain.as_str(),
            usage_tokens: total_usage,
            prompt_tokens,
            completion_tokens,
            semantic_digest: semantic_digest.as_deref(),
            prompt_payload: execution.prompt_payload.as_deref(),
            output_payload: execution.output_payload.as_deref(),
            message: None,
        };

        let persisted = persist_checkpoint(&tx, &signing_key, &checkpoint_insert)?;
        prev_chain = persisted.curr_chain;

        if kind == "Incident" {
            break;
        }
    }

    tx.commit()?;
    Ok(())
}

pub fn reopen_run(pool: &DbPool, run_id: &str) -> anyhow::Result<()> {
    {
        let mut conn = pool.get()?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM checkpoints WHERE run_id = ?1", params![run_id])?;
        tx.commit()?;
    }

    start_run(pool, run_id)
}

pub fn clone_run(pool: &DbPool, source_run_id: &str) -> anyhow::Result<String> {
    let source_run = {
        let mut conn = pool.get()?;
        load_stored_run(&conn, source_run_id)?
    };

    let spec_templates: Vec<RunCheckpointTemplate> = source_run
        .checkpoints
        .iter()
        .map(|cfg| RunCheckpointTemplate {
            model: cfg.model.clone(),
            prompt: cfg.prompt.clone(),
            token_budget: cfg.token_budget,
            order_index: Some(cfg.order_index),
            checkpoint_type: cfg.checkpoint_type.clone(),
        })
        .collect();

    let spec_snapshot = RunSpec {
        project_id: source_run.project_id.clone(),
        name: format!("{} (clone)", source_run.name),
        seed: source_run.seed,
        token_budget: source_run.token_budget,
        model: source_run.default_model.clone(),
        checkpoints: spec_templates,
        proof_mode: source_run.proof_mode,
        epsilon: source_run.epsilon,
    };

    let new_run_id = create_run(pool, spec_snapshot)?;
    start_run(pool, &new_run_id)?;
    Ok(new_run_id)
}

fn execute_checkpoint(
    config: &RunCheckpointConfig,
    run_seed: u64,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<NodeExecution> {
    if config.model == STUB_MODEL_ID {
        Ok(execute_stub_checkpoint(
            run_seed,
            config.order_index,
            &config.prompt,
        ))
    } else {
        execute_llm_checkpoint(&config.model, &config.prompt, llm_client)
    }
}

fn stub_output_bytes(seed: u64, order_index: i64, prompt: &str) -> Vec<u8> {
    let mut output = b"hello".to_vec();
    output.extend_from_slice(&seed.to_le_bytes());
    output.extend_from_slice(&order_index.to_le_bytes());
    let prompt_hash = provenance::sha256_hex(prompt.as_bytes());
    output.extend_from_slice(prompt_hash.as_bytes());
    output
}

fn execute_stub_checkpoint(run_seed: u64, order_index: i64, prompt: &str) -> NodeExecution {
    let output_bytes = stub_output_bytes(run_seed, order_index, prompt);
    let outputs_hex = provenance::sha256_hex(&output_bytes);
    let inputs_hex = provenance::sha256_hex(prompt.as_bytes());
    let semantic_source = hex::encode(&output_bytes);
    let semantic_digest = provenance::semantic_digest(&semantic_source);
    let prompt_payload = sanitize_payload(prompt);
    let output_payload = sanitize_payload(&semantic_source);

    NodeExecution {
        inputs_sha256: Some(inputs_hex),
        outputs_sha256: Some(outputs_hex),
        semantic_digest: Some(semantic_digest),
        usage: TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 10,
        },
        prompt_payload: Some(prompt_payload),
        output_payload: Some(output_payload),
    }
}

fn execute_llm_checkpoint(
    model: &str,
    prompt: &str,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<NodeExecution> {
    let generation = llm_client.stream_generate(model, prompt)?;
    let inputs_hex = provenance::sha256_hex(prompt.as_bytes());
    let outputs_hex = provenance::sha256_hex(generation.response.as_bytes());
    let semantic_digest = provenance::semantic_digest(&generation.response);
    let prompt_payload = sanitize_payload(prompt);
    let output_payload = sanitize_payload(&generation.response);

    Ok(NodeExecution {
        inputs_sha256: Some(inputs_hex),
        outputs_sha256: Some(outputs_hex),
        semantic_digest: Some(semantic_digest),
        usage: generation.usage,
        prompt_payload: Some(prompt_payload),
        output_payload: Some(output_payload),
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
    use crate::{api, keychain, provenance, store};
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
            token_budget: 1_000,
            model: STUB_MODEL_ID.to_string(),
            checkpoints: vec![RunCheckpointTemplate {
                model: STUB_MODEL_ID.to_string(),
                prompt: "{\"nodes\":[]}".to_string(),
                token_budget: 1_000,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
            }],
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
            token_budget: 25,
            model: STUB_MODEL_ID.to_string(),
            checkpoints: vec![RunCheckpointTemplate {
                model: STUB_MODEL_ID.to_string(),
                prompt: "{}".to_string(),
                token_budget: 25,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
            }],
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
            token_budget: 10_000,
            model: "llama3".to_string(),
            checkpoints: vec![RunCheckpointTemplate {
                model: "llama3".to_string(),
                prompt: prompt_json.clone(),
                token_budget: 10_000,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
            }],
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

    #[cfg(feature = "interactive")]
    #[test]
    fn start_hello_run_interactive_skips_initial_checkpoint() -> Result<()> {
        init_keychain_backend();

        let manager = SqliteConnectionManager::memory();
        let pool: Pool<SqliteConnectionManager> = Pool::builder().max_size(1).build(manager)?;
        {
            let mut conn = pool.get()?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            store::migrate_db(&mut conn)?;
        }

        let project = api::create_project_with_pool("Interactive Start".into(), &pool)?;
        let spec = RunSpec {
            project_id: project.id.clone(),
            name: "interactive-start".to_string(),
            seed: 99,
            token_budget: 5_000,
            model: STUB_MODEL_ID.to_string(),
            checkpoints: vec![RunCheckpointTemplate {
                model: STUB_MODEL_ID.to_string(),
                prompt: "Interact with the operator".to_string(),
                token_budget: 1_000,
                order_index: Some(0),
                checkpoint_type: "InteractiveChat".to_string(),
            }],
            proof_mode: RunProofMode::Exact,
            epsilon: None,
        };
        let spec_clone = spec.clone();

        let start_client = RecordingLlmClient::new(
            spec.model.clone(),
            String::new(),
            "unused".to_string(),
            TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
            },
        );

        let run_id = start_hello_run_with_client(&pool, spec, &start_client)?;
        assert_eq!(*start_client.calls.lock().unwrap(), 0);

        let conn = pool.get()?;
        let (kind, stored_spec_json, checkpoint_count): (String, String, i64) = conn.query_row(
            "SELECT kind, spec_json, (SELECT COUNT(*) FROM checkpoints WHERE run_id = runs.id) FROM runs WHERE id = ?1",
            params![&run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(kind, "exact");
        let stored_spec: RunSpec = serde_json::from_str(&stored_spec_json)?;
        assert_eq!(stored_spec, spec_clone);
        assert_eq!(checkpoint_count, 0);

        let config_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM run_checkpoints WHERE run_id = ?1 AND LOWER(checkpoint_type) = 'interactivechat'",
            params![&run_id],
            |row| row.get(0),
        )?;
        assert_eq!(config_count, 1);

        Ok(())
    }

    #[cfg(feature = "interactive")]
    #[test]
    fn submit_turn_records_usage_and_messages() -> Result<()> {
        init_keychain_backend();

        let manager = SqliteConnectionManager::memory();
        let pool: Pool<SqliteConnectionManager> = Pool::builder().max_size(1).build(manager)?;
        {
            let mut conn = pool.get()?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            store::migrate_db(&mut conn)?;
        }

        let project = api::create_project_with_pool("Interactive Submit".into(), &pool)?;
        let chat_prompt = "You are a meticulous co-pilot.".to_string();
        let run_model = "stub-model".to_string();
        let run_spec = RunSpec {
            project_id: project.id.clone(),
            name: "interactive-run".into(),
            seed: 0,
            token_budget: 10_000,
            model: run_model.clone(),
            checkpoints: vec![RunCheckpointTemplate {
                model: run_model.clone(),
                prompt: chat_prompt.clone(),
                token_budget: 10_000,
                order_index: Some(0),
                checkpoint_type: "InteractiveChat".to_string(),
            }],
            proof_mode: RunProofMode::Exact,
            epsilon: None,
        };

        let start_client = RecordingLlmClient::new(
            run_model.clone(),
            String::new(),
            "unused".to_string(),
            TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
            },
        );

        let run_id = start_hello_run_with_client(&pool, run_spec.clone(), &start_client)?;
        assert_eq!(*start_client.calls.lock().unwrap(), 0);

        let config_id: String = {
            let conn = pool.get()?;
            conn.query_row(
                "SELECT id FROM run_checkpoints WHERE run_id = ?1",
                params![&run_id],
                |row| row.get(0),
            )?
        };

        let prompt_text = "Hello partner".to_string();
        let expected_prompt = super::build_interactive_prompt(&chat_prompt, &[], &prompt_text);
        let client = RecordingLlmClient::new(
            run_model.clone(),
            expected_prompt,
            "Greetings from AI".to_string(),
            TokenUsage {
                prompt_tokens: 7,
                completion_tokens: 11,
            },
        );

        let outcome = submit_interactive_checkpoint_turn_with_client(
            &pool,
            &run_id,
            &config_id,
            &prompt_text,
            &client,
        )?;
        assert_eq!(*client.calls.lock().unwrap(), 1);
        assert_eq!(outcome.ai_response, "Greetings from AI");
        assert_eq!(outcome.usage.prompt_tokens, 7);
        assert_eq!(outcome.usage.completion_tokens, 11);

        let conn = pool.get()?;
        struct SimpleCheckpoint {
            id: String,
            parent: Option<String>,
            turn_index: Option<u32>,
            usage_tokens: u64,
            prompt_tokens: u64,
            completion_tokens: u64,
            kind: String,
            config_id: Option<String>,
        }
        let mut stmt = conn.prepare(
            "SELECT id, parent_checkpoint_id, turn_index, usage_tokens, prompt_tokens, completion_tokens, kind, checkpoint_config_id \
             FROM checkpoints WHERE run_id = ?1 ORDER BY turn_index ASC",
        )?;
        let rows = stmt.query_map(params![&run_id], |row| {
            let turn_index = row
                .get::<_, Option<i64>>(2)?
                .map(|value| value.max(0) as u32);
            let usage_tokens: i64 = row.get(3)?;
            let prompt_tokens: i64 = row.get(4)?;
            let completion_tokens: i64 = row.get(5)?;
            Ok(SimpleCheckpoint {
                id: row.get(0)?,
                parent: row.get(1)?,
                turn_index,
                usage_tokens: usage_tokens.max(0) as u64,
                prompt_tokens: prompt_tokens.max(0) as u64,
                completion_tokens: completion_tokens.max(0) as u64,
                kind: row.get(6)?,
                config_id: row.get(7)?,
            })
        })?;
        let mut checkpoints = Vec::new();
        for row in rows {
            checkpoints.push(row?);
        }
        assert_eq!(checkpoints.len(), 2);
        let human = &checkpoints[0];
        let ai = &checkpoints[1];

        assert_eq!(human.kind, "Step");
        assert_eq!(human.parent, None);
        assert_eq!(human.turn_index, Some(0));
        assert_eq!(human.usage_tokens, 0);
        assert_eq!(human.prompt_tokens, 0);
        assert_eq!(human.completion_tokens, 0);
        assert_eq!(human.config_id.as_deref(), Some(config_id.as_str()));

        assert_eq!(ai.kind, "Step");
        assert_eq!(ai.parent.as_deref(), Some(human.id.as_str()));
        assert_eq!(ai.turn_index, Some(1));
        assert_eq!(ai.usage_tokens, 18);
        assert_eq!(ai.prompt_tokens, 7);
        assert_eq!(ai.completion_tokens, 11);
        assert_eq!(ai.config_id.as_deref(), Some(config_id.as_str()));

        let (human_role, human_body): (String, String) = conn.query_row(
            "SELECT role, body FROM checkpoint_messages WHERE checkpoint_id = ?1",
            params![&human.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(human_role, "human");
        assert_eq!(human_body, prompt_text);

        let (ai_role, ai_body): (String, String) = conn.query_row(
            "SELECT role, body FROM checkpoint_messages WHERE checkpoint_id = ?1",
            params![&ai.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(ai_role, "ai");
        assert_eq!(ai_body, "Greetings from AI");

        let totals: (i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(prompt_tokens), 0), COALESCE(SUM(completion_tokens), 0) FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(totals.0, 7);
        assert_eq!(totals.1, 11);

        Ok(())
    }

    #[cfg(feature = "interactive")]
    #[test]
    fn submit_turn_rejects_when_budget_exceeded() -> Result<()> {
        init_keychain_backend();

        let manager = SqliteConnectionManager::memory();
        let pool: Pool<SqliteConnectionManager> = Pool::builder().max_size(1).build(manager)?;
        {
            let mut conn = pool.get()?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            store::migrate_db(&mut conn)?;
        }

        let project = api::create_project_with_pool("Interactive Budget Gate".into(), &pool)?;
        let chat_prompt = "Keep responses concise.".to_string();
        let run_model = STUB_MODEL_ID.to_string();
        let token_budget = 10;
        let run_spec = RunSpec {
            project_id: project.id.clone(),
            name: "interactive-budget".into(),
            seed: 0,
            token_budget,
            model: run_model.clone(),
            checkpoints: vec![RunCheckpointTemplate {
                model: run_model.clone(),
                prompt: chat_prompt.clone(),
                token_budget,
                order_index: Some(0),
                checkpoint_type: "InteractiveChat".to_string(),
            }],
            proof_mode: RunProofMode::Exact,
            epsilon: None,
        };

        let start_client = RecordingLlmClient::new(
            run_model.clone(),
            String::new(),
            "unused".to_string(),
            TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
            },
        );
        let run_id = start_hello_run_with_client(&pool, run_spec, &start_client)?;
        assert_eq!(*start_client.calls.lock().unwrap(), 0);

        let config_id: String = {
            let conn = pool.get()?;
            conn.query_row(
                "SELECT id FROM run_checkpoints WHERE run_id = ?1",
                params![&run_id],
                |row| row.get(0),
            )?
        };

        let first_prompt = "Initial exchange".to_string();
        let first_expected = super::build_interactive_prompt(&chat_prompt, &[], &first_prompt);
        let first_client = RecordingLlmClient::new(
            run_model.clone(),
            first_expected,
            "First reply".to_string(),
            TokenUsage {
                prompt_tokens: 6,
                completion_tokens: 2,
            },
        );
        submit_interactive_checkpoint_turn_with_client(
            &pool,
            &run_id,
            &config_id,
            &first_prompt,
            &first_client,
        )?;
        assert_eq!(*first_client.calls.lock().unwrap(), 1);

        let transcript = {
            let conn = pool.get()?;
            super::load_interactive_messages(&conn, &run_id, &config_id)?
        };

        let second_prompt = "Need more budget".to_string();
        let second_expected =
            super::build_interactive_prompt(&chat_prompt, &transcript, &second_prompt);
        let second_client = RecordingLlmClient::new(
            run_model.clone(),
            second_expected,
            "Denied".to_string(),
            TokenUsage {
                prompt_tokens: 5,
                completion_tokens: 0,
            },
        );

        let result = submit_interactive_checkpoint_turn_with_client(
            &pool,
            &run_id,
            &config_id,
            &second_prompt,
            &second_client,
        );
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("token budget"));
        assert_eq!(*second_client.calls.lock().unwrap(), 1);

        let conn = pool.get()?;
        let checkpoint_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?;
        assert_eq!(checkpoint_count, 2);

        Ok(())
    }

    #[cfg(feature = "interactive")]
    #[test]
    fn finalize_interactive_checkpoint_requires_transcript() -> Result<()> {
        init_keychain_backend();

        let manager = SqliteConnectionManager::memory();
        let pool: Pool<SqliteConnectionManager> = Pool::builder().max_size(1).build(manager)?;
        {
            let mut conn = pool.get()?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            store::migrate_db(&mut conn)?;
        }

        let project = api::create_project_with_pool("Interactive Finalize".into(), &pool)?;
        let chat_prompt = "Act as a careful reviewer.".to_string();
        let run_model = STUB_MODEL_ID.to_string();
        let run_spec = RunSpec {
            project_id: project.id.clone(),
            name: "interactive-finalize".into(),
            seed: 1,
            token_budget: 5_000,
            model: run_model.clone(),
            checkpoints: vec![RunCheckpointTemplate {
                model: run_model.clone(),
                prompt: chat_prompt.clone(),
                token_budget: 5_000,
                order_index: Some(0),
                checkpoint_type: "InteractiveChat".to_string(),
            }],
            proof_mode: RunProofMode::Exact,
            epsilon: None,
        };

        let start_client = RecordingLlmClient::new(
            run_model.clone(),
            String::new(),
            "unused".to_string(),
            TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
            },
        );
        let run_id = start_hello_run_with_client(&pool, run_spec, &start_client)?;
        assert_eq!(*start_client.calls.lock().unwrap(), 0);

        let config_id: String = {
            let conn = pool.get()?;
            conn.query_row(
                "SELECT id FROM run_checkpoints WHERE run_id = ?1",
                params![&run_id],
                |row| row.get(0),
            )?
        };

        let empty_finalize = finalize_interactive_checkpoint(&pool, &run_id, &config_id);
        assert!(empty_finalize.is_err());

        let prompt_text = "First turn".to_string();
        let expected_prompt = super::build_interactive_prompt(&chat_prompt, &[], &prompt_text);
        let turn_client = RecordingLlmClient::new(
            run_model.clone(),
            expected_prompt,
            "Response".to_string(),
            TokenUsage {
                prompt_tokens: 2,
                completion_tokens: 3,
            },
        );
        submit_interactive_checkpoint_turn_with_client(
            &pool,
            &run_id,
            &config_id,
            &prompt_text,
            &turn_client,
        )?;
        assert_eq!(*turn_client.calls.lock().unwrap(), 1);

        finalize_interactive_checkpoint(&pool, &run_id, &config_id)?;

        Ok(())
    }
}
