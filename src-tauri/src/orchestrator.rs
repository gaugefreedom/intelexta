// src-tauri/src/orchestrator.rs
use crate::api::RunStepRequest;
use crate::{governance, provenance, store, DbPool};
use anyhow::{anyhow, Context};
use chrono::Utc;
use ed25519_dalek::SigningKey;
use keyring::Error as KeyringError;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::TryFrom;
use std::fmt;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::ops::Deref;
use std::time::Duration;
use uuid::Uuid;

const STUB_MODEL_ID: &str = "stub-model";

// Debug logging flag - set to false for production
const DEBUG_STEP_EXECUTION: bool = true;
const OLLAMA_HOST: &str = "127.0.0.1:11434";
const MAX_RUN_NAME_LENGTH: usize = 120;
const MAX_PAYLOAD_PREVIEW_SIZE: usize = 65_536; // 64KB preview limit

// External API provider prefixes
const CLAUDE_MODEL_PREFIX: &str = "claude-";
const CLAUDE_API_PLACEHOLDER_KEY: &str = "sk-ant-placeholder-key-not-configured";

/// Configuration for document ingestion steps
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIngestionConfig {
    pub source_path: String,
    pub format: String, // "pdf", "latex", "docx", "txt"
    pub privacy_status: String, // "public", "consent_obtained_anonymized", etc.
    #[serde(default)]
    pub output_storage: String, // "database" or "file", defaults to "database"
}

/// Typed step configuration enum
/// Each step type has its own configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stepType")]
pub enum StepConfig {
    /// Ingest document from filesystem
    #[serde(rename = "ingest", rename_all = "camelCase")]
    Ingest {
        source_path: String,
        format: String,  // "pdf", "latex", "txt", "docx"
        privacy_status: String,
    },

    /// Summarize output from a previous step
    #[serde(rename = "summarize", rename_all = "camelCase")]
    Summarize {
        /// Optional: index of source step to summarize (None = error)
        source_step: Option<usize>,

        model: String,
        summary_type: String,  // "brief", "detailed", "academic", "custom"

        #[serde(skip_serializing_if = "Option::is_none")]
        custom_instructions: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        token_budget: Option<i32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        proof_mode: Option<String>,  // "exact" or "concordant"

        #[serde(skip_serializing_if = "Option::is_none")]
        epsilon: Option<f64>,
    },

    /// Custom LLM prompt (optionally using previous step output)
    #[serde(rename = "prompt", rename_all = "camelCase")]
    Prompt {
        model: String,
        prompt: String,

        /// Optional: index of step to use as context
        #[serde(skip_serializing_if = "Option::is_none")]
        use_output_from: Option<usize>,

        #[serde(skip_serializing_if = "Option::is_none")]
        token_budget: Option<i32>,

        #[serde(skip_serializing_if = "Option::is_none")]
        proof_mode: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        epsilon: Option<f64>,
    },
}

/// Output from a step execution (for chaining)
#[derive(Debug, Clone)]
pub struct StepOutput {
    pub order_index: usize,
    pub step_type: String,
    pub output_text: String,
    pub output_json: Option<serde_json::Value>,
    pub outputs_sha256: String,
}

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
    run_execution_id: &'a str,
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

#[derive(Debug, Clone)]
pub struct RunProofModeParseError {
    mode: String,
}

impl RunProofModeParseError {
    fn new(mode: &str) -> Self {
        Self {
            mode: mode.to_string(),
        }
    }
}

impl fmt::Display for RunProofModeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported run proof mode: {}", self.mode)
    }
}

impl std::error::Error for RunProofModeParseError {}

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
    type Error = RunProofModeParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "exact" => Ok(RunProofMode::Exact),
            "concordant" => Ok(RunProofMode::Concordant),
            "interactive" => Ok(RunProofMode::Exact),
            other => Err(RunProofModeParseError::new(other)),
        }
    }
}

fn default_checkpoint_type() -> String {
    "Step".to_string()
}

fn default_step_type() -> String {
    "llm".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStepTemplate {
    #[serde(default = "default_step_type")]
    pub step_type: String,
    // LLM step fields (optional for document ingestion steps)
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub token_budget: u64,
    #[serde(default)]
    pub proof_mode: RunProofMode,
    #[serde(default)]
    pub epsilon: Option<f64>,
    // Document ingestion config (as JSON string)
    #[serde(default)]
    pub config_json: Option<String>,
    // Common fields
    #[serde(default)]
    pub order_index: Option<i64>,
    #[serde(default = "default_checkpoint_type")]
    pub checkpoint_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStep {
    pub id: String,
    pub run_id: String,
    pub order_index: i64,
    pub checkpoint_type: String,
    #[serde(default = "default_step_type")]
    pub step_type: String,
    // LLM step fields (optional for document ingestion steps)
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
    // Document ingestion config (as JSON string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_json: Option<String>,
}

impl RunStep {
    pub fn is_interactive_chat(&self) -> bool {
        self.checkpoint_type.eq_ignore_ascii_case("InteractiveChat")
    }

    pub fn is_llm_step(&self) -> bool {
        self.step_type == "llm"
    }

    pub fn is_document_ingestion(&self) -> bool {
        self.step_type == "document_ingestion"
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredRun {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub seed: u64,
    pub token_budget: u64,
    pub default_model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_mode: Option<RunProofMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
    pub steps: Vec<RunStep>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunExecutionRecord {
    pub id: String,
    pub run_id: String,
    pub created_at: String,
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

pub(crate) struct NodeExecution {
    pub(crate) inputs_sha256: Option<String>,
    pub(crate) outputs_sha256: Option<String>,
    pub(crate) semantic_digest: Option<String>,
    pub(crate) usage: TokenUsage,
    pub(crate) prompt_payload: Option<String>,
    pub(crate) output_payload: Option<String>,
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
    pub estimated_nature_cost: f64,
    pub budget_tokens: u64,
    pub budget_usd: f64,
    pub budget_nature_cost: f64,
    pub exceeds_tokens: bool,
    pub exceeds_usd: bool,
    pub exceeds_nature_cost: bool,
}

impl RunCostEstimates {
    fn exceeds_any(&self) -> bool {
        self.exceeds_tokens || self.exceeds_usd || self.exceeds_nature_cost
    }
}

fn sum_token_budgets(configs: &[RunStep]) -> u64 {
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

    // Use governance module functions for cost estimation
    let estimated_usd = governance::estimate_usd_cost(estimated_tokens);
    let estimated_nature_cost = governance::estimate_nature_cost(estimated_tokens);

    RunCostEstimates {
        estimated_tokens,
        estimated_usd,
        estimated_nature_cost,
        budget_tokens: token_budget,
        budget_usd: policy.budget_usd,
        budget_nature_cost: policy.budget_nature_cost,
        exceeds_tokens: estimated_tokens > token_budget,
        exceeds_usd: estimated_usd > policy.budget_usd,
        exceeds_nature_cost: estimated_nature_cost > policy.budget_nature_cost,
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

    // Add stub model for testing
    if !models.iter().any(|m| m == STUB_MODEL_ID) {
        models.insert(0, STUB_MODEL_ID.to_string());
    }

    // Add Claude API models (mock implementations)
    let claude_models = vec![
        "claude-3-5-sonnet-20241022".to_string(),
        "claude-3-5-haiku-20241022".to_string(),
        "claude-3-opus-20240229".to_string(),
    ];
    models.extend(claude_models);

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

pub fn create_run(
    pool: &DbPool,
    project_id: &str,
    name: &str,
    proof_mode: RunProofMode,
    epsilon: Option<f64>,
    seed: u64,
    token_budget: u64,
    default_model: &str,
    mut steps: Vec<RunStepTemplate>,
) -> anyhow::Result<String> {
    let run_epsilon = match (proof_mode.is_concordant(), epsilon) {
        (true, Some(value)) => {
            if !value.is_finite() || value < 0.0 {
                return Err(anyhow!("epsilon must be a finite, non-negative value"));
            }
            Some(value)
        }
        (true, None) => {
            return Err(anyhow!("concordant runs require an epsilon"));
        }
        (false, Some(value)) => {
            if !value.is_finite() || value < 0.0 {
                return Err(anyhow!("epsilon must be a finite, non-negative value"));
            }
            Some(value)
        }
        (false, None) => None,
    };

    for template in &mut steps {
        if template.proof_mode.is_concordant() {
            let epsilon = template
                .epsilon
                .or(run_epsilon)
                .ok_or_else(|| anyhow!("concordant steps require an epsilon"))?;
            if !epsilon.is_finite() || epsilon < 0.0 {
                return Err(anyhow!("epsilon must be a finite, non-negative value"));
            }
            template.epsilon = Some(epsilon);
        } else if let Some(value) = template.epsilon {
            if !value.is_finite() || value < 0.0 {
                return Err(anyhow!("epsilon must be a finite, non-negative value"));
            }
        }
    }

    let mut conn = pool.get()?;
    ensure_project_signing_key(&conn, project_id)?;

    let run_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    // Check if the provided name is empty.
    let sanitized_name = sanitize_run_name_input(name);
    if !sanitized_name.is_empty() && sanitized_name.chars().count() > MAX_RUN_NAME_LENGTH {
        return Err(anyhow!(format!(
            "run name must be {} characters or fewer",
            MAX_RUN_NAME_LENGTH
        )));
    }
    let final_name = if sanitized_name.is_empty() {
        "New run".to_string()
    } else {
        sanitized_name
    };

    {
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO runs (id, project_id, name, created_at, sampler_json, seed, epsilon, token_budget, default_model, proof_mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                &run_id,
                project_id,
                &final_name,
                &now,
                Option::<String>::None,
                (seed as i64),
                run_epsilon,
                (token_budget as i64),
                default_model,
                proof_mode.as_str(),
            ],
        )?;

        for (index, template) in steps.iter().enumerate() {
            let checkpoint_id = Uuid::new_v4().to_string();
            let order_index = template.order_index.unwrap_or(index as i64);
            tx.execute(
                "INSERT INTO run_steps (id, run_id, order_index, checkpoint_type, step_type, model, prompt, token_budget, proof_mode, epsilon, config_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    &checkpoint_id,
                    &run_id,
                    order_index,
                    &template.checkpoint_type,
                    &template.step_type,
                    &template.model,
                    &template.prompt,
                    (template.token_budget as i64),
                    template.proof_mode.as_str(),
                    template.epsilon,
                    &template.config_json,
                ],
            )?;
        }

        tx.commit()?;
    }

    Ok(run_id)
}

fn sanitize_run_name_input(value: &str) -> String {
    let without_nulls: String = value.chars().filter(|&ch| ch != '\u{0}').collect();
    let collapsed = without_nulls
        .split_whitespace()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    collapsed.trim().to_string()
}

pub fn rename_run(pool: &DbPool, run_id: &str, name: &str) -> anyhow::Result<()> {
    let sanitized = sanitize_run_name_input(name);
    if sanitized.is_empty() {
        return Err(anyhow!("run name cannot be empty"));
    }
    if sanitized.chars().count() > MAX_RUN_NAME_LENGTH {
        return Err(anyhow!(format!(
            "run name must be {} characters or fewer",
            MAX_RUN_NAME_LENGTH
        )));
    }

    let conn = pool.get()?;
    let affected = conn.execute(
        "UPDATE runs SET name = ?1 WHERE id = ?2",
        params![sanitized, run_id],
    )?;
    if affected == 0 {
        return Err(anyhow!(format!("run {run_id} not found")));
    }
    Ok(())
}

pub fn delete_run(pool: &DbPool, run_id: &str) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    let tx = conn.transaction()?;

    tx.execute(
        "DELETE FROM checkpoint_payloads WHERE checkpoint_id IN (SELECT id FROM checkpoints WHERE run_id = ?1)",
        params![run_id],
    )?;

    tx.execute(
        "DELETE FROM checkpoint_messages WHERE checkpoint_id IN (SELECT id FROM checkpoints WHERE run_id = ?1)",
        params![run_id],
    )?;

    tx.execute("DELETE FROM receipts WHERE run_id = ?1", params![run_id])?;

    tx.execute("DELETE FROM checkpoints WHERE run_id = ?1", params![run_id])?;

    tx.execute(
        "DELETE FROM run_executions WHERE run_id = ?1",
        params![run_id],
    )?;

    tx.execute("DELETE FROM run_steps WHERE run_id = ?1", params![run_id])?;

    let affected = tx.execute("DELETE FROM runs WHERE id = ?1", params![run_id])?;
    if affected == 0 {
        return Err(anyhow!(format!("run {run_id} not found")));
    }

    tx.commit()?;
    Ok(())
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
        "INSERT INTO checkpoints (id, run_id, run_execution_id, checkpoint_config_id, parent_checkpoint_id, turn_index, kind, incident_json, timestamp, inputs_sha256, outputs_sha256, prev_chain, curr_chain, signature, usage_tokens, semantic_digest, prompt_tokens, completion_tokens) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
        params![
            &checkpoint_id,
            params.run_id,
            params.run_execution_id,
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
    run_execution_id: &str,
    checkpoint_config_id: Option<&str>,
) -> anyhow::Result<(u64, u64)> {
    let (prompt_total, completion_total): (i64, i64) = match checkpoint_config_id {
        Some(config_id) => conn.query_row(
            "SELECT COALESCE(SUM(prompt_tokens), 0), COALESCE(SUM(completion_tokens), 0) FROM checkpoints WHERE run_id = ?1 AND run_execution_id = ?2 AND checkpoint_config_id = ?3",
            params![run_id, run_execution_id, config_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?,
        None => conn.query_row(
            "SELECT COALESCE(SUM(prompt_tokens), 0), COALESCE(SUM(completion_tokens), 0) FROM checkpoints WHERE run_id = ?1 AND run_execution_id = ?2",
            params![run_id, run_execution_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?,
    };

    let prompt = prompt_total.max(0) as u64;
    let completion = completion_total.max(0) as u64;
    Ok((prompt, completion))
}

fn load_run_steps(conn: &Connection, run_id: &str) -> anyhow::Result<Vec<RunStep>> {
    let mut stmt = conn.prepare(
        "SELECT id, order_index, checkpoint_type, step_type, model, prompt, token_budget, proof_mode, epsilon, config_json FROM run_steps WHERE run_id = ?1 ORDER BY order_index ASC",
    )?;
    let rows = stmt.query_map(params![run_id], |row| {
        let token_budget: i64 = row.get(6)?;
        let proof_mode_str: String = row.get(7)?;
        let proof_mode = RunProofMode::try_from(proof_mode_str.as_str()).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(err))
        })?;
        Ok(RunStep {
            id: row.get(0)?,
            run_id: run_id.to_string(),
            order_index: row.get(1)?,
            checkpoint_type: row.get(2)?,
            step_type: row.get(3)?,
            model: row.get(4)?,
            prompt: row.get(5)?,
            token_budget: token_budget.max(0) as u64,
            proof_mode,
            epsilon: row.get(8)?,
            config_json: row.get(9)?,
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
    let projected_tokens = sum_token_budgets(&stored_run.steps);
    Ok(estimate_costs_with_policy(&policy, projected_tokens))
}

fn load_checkpoint_config_by_id(
    conn: &Connection,
    checkpoint_id: &str,
) -> anyhow::Result<Option<RunStep>> {
    let row: Option<(String, i64, String, String, Option<String>, Option<String>, i64, String, Option<f64>, Option<String>)> = conn
        .query_row(
            "SELECT run_id, order_index, checkpoint_type, step_type, model, prompt, token_budget, proof_mode, epsilon, config_json FROM run_steps WHERE id = ?1",
            params![checkpoint_id],
            |row| Ok((
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
            )),
        )
        .optional()?;

    let Some((
        run_id,
        order_index,
        checkpoint_type,
        step_type,
        model,
        prompt,
        token_budget_raw,
        proof_mode_raw,
        epsilon,
        config_json,
    )) = row
    else {
        return Ok(None);
    };

    let proof_mode = RunProofMode::try_from(proof_mode_raw.as_str()).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(err))
    })?;

    Ok(Some(RunStep {
        id: checkpoint_id.to_string(),
        run_id,
        order_index,
        checkpoint_type,
        step_type,
        model,
        prompt,
        token_budget: token_budget_raw.max(0) as u64,
        proof_mode,
        epsilon,
        config_json,
    }))
}

pub fn load_stored_run(conn: &Connection, run_id: &str) -> anyhow::Result<StoredRun> {
    let row: Option<(String, String, i64, Option<f64>, i64, String, String)> = conn
        .query_row(
            "SELECT project_id, name, seed, epsilon, token_budget, default_model, proof_mode FROM runs WHERE id = ?1",
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

    let (project_id, name, seed_raw, epsilon, token_budget_raw, default_model, proof_mode_raw) =
        row.ok_or_else(|| anyhow!(format!("run {run_id} not found")))?;
    let seed = seed_raw.max(0) as u64;
    let token_budget = token_budget_raw.max(0) as u64;
    let steps = load_run_steps(conn, run_id)?;
    let proof_mode = RunProofMode::try_from(proof_mode_raw.as_str()).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(err))
    })?;

    Ok(StoredRun {
        id: run_id.to_string(),
        project_id,
        name,
        seed,
        token_budget,
        default_model,
        proof_mode: Some(proof_mode),
        epsilon,
        steps,
    })
}

fn insert_run_execution(conn: &Connection, run_id: &str) -> anyhow::Result<RunExecutionRecord> {
    let execution_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO run_executions (id, run_id, created_at) VALUES (?1, ?2, ?3)",
        params![&execution_id, run_id, &created_at],
    )?;

    Ok(RunExecutionRecord {
        id: execution_id,
        run_id: run_id.to_string(),
        created_at,
    })
}

pub fn list_run_executions(
    conn: &Connection,
    run_id: &str,
) -> anyhow::Result<Vec<RunExecutionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, created_at FROM run_executions WHERE run_id = ?1 ORDER BY datetime(created_at) DESC, id DESC",
    )?;

    let rows = stmt.query_map(params![run_id], |row| {
        Ok(RunExecutionRecord {
            id: row.get(0)?,
            run_id: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;

    let mut executions = Vec::new();
    for entry in rows {
        executions.push(entry?);
    }

    Ok(executions)
}

pub fn load_latest_run_execution(
    conn: &Connection,
    run_id: &str,
) -> anyhow::Result<Option<RunExecutionRecord>> {
    conn.query_row(
        "SELECT id, run_id, created_at FROM run_executions WHERE run_id = ?1 ORDER BY datetime(created_at) DESC, id DESC LIMIT 1",
        params![run_id],
        |row| {
            Ok(RunExecutionRecord {
                id: row.get(0)?,
                run_id: row.get(1)?,
                created_at: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

struct LastCheckpointInfo {
    id: String,
    curr_chain: String,
    turn_index: Option<u32>,
}

fn load_last_checkpoint(
    conn: &Connection,
    run_id: &str,
    run_execution_id: &str,
) -> anyhow::Result<Option<LastCheckpointInfo>> {
    let row = conn
        .query_row(
            "SELECT id, curr_chain, turn_index FROM checkpoints WHERE run_id = ?1 AND run_execution_id = ?2 ORDER BY COALESCE(turn_index, -1) DESC, timestamp DESC LIMIT 1",
            params![run_id, run_execution_id],
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
    run_execution_id: &str,
    checkpoint_config_id: &str,
) -> anyhow::Result<Option<LastCheckpointInfo>> {
    let row = conn
        .query_row(
            "SELECT id, curr_chain, turn_index FROM checkpoints WHERE run_id = ?1 AND run_execution_id = ?2 AND checkpoint_config_id = ?3 ORDER BY COALESCE(turn_index, -1) DESC, timestamp DESC LIMIT 1",
            params![run_id, run_execution_id, checkpoint_config_id],
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
    run_execution_id: &str,
    checkpoint_config_id: &str,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT m.role, m.body FROM checkpoints c JOIN checkpoint_messages m ON m.checkpoint_id = c.id WHERE c.run_id = ?1 AND c.run_execution_id = ?2 AND c.checkpoint_config_id = ?3 ORDER BY COALESCE(c.turn_index, -1) ASC, c.timestamp ASC",
    )?;

    let rows = stmt.query_map(
        params![run_id, run_execution_id, checkpoint_config_id],
        |row| {
            let role: String = row.get(0)?;
            let body: String = row.get(1)?;
            Ok((role, body))
        },
    )?;

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

    let latest_execution = load_latest_run_execution(&conn, run_id)?
        .ok_or_else(|| anyhow!("run has not been executed yet"))?;
    let run_execution_id = latest_execution.id.clone();

    let transcript =
        load_interactive_messages(&conn, run_id, &run_execution_id, checkpoint_config_id)?;

    // Interactive checkpoints must have prompt and model
    let config_prompt = config.prompt.as_ref()
        .ok_or_else(|| anyhow!("interactive checkpoint missing prompt"))?;
    let config_model = config.model.as_ref()
        .ok_or_else(|| anyhow!("interactive checkpoint missing model"))?;

    let llm_prompt = build_interactive_prompt(config_prompt, &transcript, trimmed_prompt);

    let signing_key = ensure_project_signing_key(&conn, &stored_run.project_id)?;

    // Enforce network policy for interactive checkpoints
    let policy = store::policies::get(&conn, &stored_run.project_id)?;
    if let Err(network_incident) = governance::enforce_network_policy(&policy) {
        return Err(anyhow!(format!(
            "Network access denied by project policy: {}",
            network_incident.details
        )));
    }

    let LlmGeneration { response, usage } =
        llm_client.stream_generate(config_model, &llm_prompt)?;
    let sanitized_llm_prompt = sanitize_payload(&llm_prompt);
    let sanitized_response = sanitize_payload(&response);

    let tx = conn.transaction()?;

    let (prior_prompt, prior_completion) = sum_checkpoint_token_usage(
        &tx,
        run_id,
        run_execution_id.as_str(),
        Some(checkpoint_config_id),
    )?;
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

    let last_checkpoint = load_last_checkpoint(&tx, run_id, run_execution_id.as_str())?;
    let parent_checkpoint_id_owned = last_checkpoint.as_ref().map(|info| info.id.clone());
    let prev_chain_owned = last_checkpoint.as_ref().map(|info| info.curr_chain.clone());
    let parent_checkpoint_ref = parent_checkpoint_id_owned
        .as_ref()
        .map(|value| value.as_str());
    let prev_chain_ref = prev_chain_owned.as_deref().unwrap_or("");

    let config_last_checkpoint = load_last_checkpoint_for_config(
        &tx,
        run_id,
        run_execution_id.as_str(),
        checkpoint_config_id,
    )?;
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
        run_execution_id: run_execution_id.as_str(),
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
        run_execution_id: run_execution_id.as_str(),
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
    let conn = pool.get()?;

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

    let latest_execution = load_latest_run_execution(&conn, run_id)?
        .ok_or_else(|| anyhow!("run has not been executed yet"))?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM checkpoints WHERE run_id = ?1 AND run_execution_id = ?2 AND checkpoint_config_id = ?3",
        params![run_id, latest_execution.id, checkpoint_config_id],
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
    project_id: &str,
    name: &str,
    proof_mode: RunProofMode,
    epsilon: Option<f64>,
    seed: u64,
    token_budget: u64,
    default_model: &str,
    steps: Vec<RunStepTemplate>,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<String> {
    if steps.is_empty() {
        return Err(anyhow!(
            "run requires at least one checkpoint configuration"
        ));
    }

    let run_id = create_run(
        pool,
        project_id,
        name,
        proof_mode,
        epsilon,
        seed,
        token_budget,
        default_model,
        steps,
    )?;
    let _ = start_run_with_client(pool, &run_id, llm_client)?;

    Ok(run_id)
}

pub fn start_run(pool: &DbPool, run_id: &str) -> anyhow::Result<RunExecutionRecord> {
    let client = DefaultOllamaClient::new();
    start_run_with_client(pool, run_id, &client)
}

pub(crate) fn start_run_with_client(
    pool: &DbPool,
    run_id: &str,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<RunExecutionRecord> {
    let mut conn = pool.get()?;
    let stored_run = load_stored_run(&conn, run_id)?;

    if stored_run.steps.is_empty() {
        return Err(anyhow!(format!(
            "run {run_id} has no configured checkpoints"
        )));
    }

    for config in stored_run
        .steps
        .iter()
        .filter(|config| !config.is_interactive_chat())
        .filter(|config| config.proof_mode.is_concordant())
    {
        let epsilon = config
            .epsilon
            .or(stored_run.epsilon)
            .ok_or_else(|| anyhow!("concordant steps require an epsilon"))?;
        if !epsilon.is_finite() || epsilon < 0.0 {
            return Err(anyhow!(
                "step epsilon must be a finite, non-negative value for concordant checkpoints"
            ));
        }
    }

    let tx = conn.transaction()?;
    let execution_record = insert_run_execution(&tx, run_id)?;
    let signing_key = ensure_project_signing_key(&tx, &stored_run.project_id)?;
    let policy = store::policies::get(tx.deref(), &stored_run.project_id)?;
    let mut prev_chain = String::new();
    let mut cumulative_usage_tokens: u64 = 0;

    // Track step outputs for chaining
    let mut prior_outputs: std::collections::HashMap<usize, StepOutput> = std::collections::HashMap::new();

    for (index, config) in stored_run.steps.iter().enumerate() {
        if config.is_interactive_chat() {
            continue;
        }

        let timestamp = Utc::now().to_rfc3339();

        let projected_remaining_tokens = sum_token_budgets(&stored_run.steps[index..]);
        let projected_total_tokens =
            cumulative_usage_tokens.saturating_add(projected_remaining_tokens);
        let projected_costs = estimate_costs_with_policy(&policy, projected_total_tokens);

        // Check blocking budget violations (tokens and USD)
        let has_blocking_violation = projected_costs.exceeds_tokens || projected_costs.exceeds_usd;

        if has_blocking_violation {
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
                run_execution_id: execution_record.id.as_str(),
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

            persist_checkpoint(&tx, &signing_key, &checkpoint_insert)?;
            break;
        }

        // Handle Nature Cost warning (non-blocking)
        if projected_costs.exceeds_nature_cost {
            let warning = governance::Incident {
                kind: "nature_cost_warning".into(),
                severity: "warn".into(),
                details: format!(
                    "Nature Cost {:.2} exceeds budget {:.2} for checkpoint {} (execution continues)",
                    projected_costs.estimated_nature_cost, projected_costs.budget_nature_cost, config.id
                ),
            };
            let warning_value = serde_json::to_value(&warning)?;

            let warning_checkpoint = CheckpointInsert {
                run_id,
                run_execution_id: execution_record.id.as_str(),
                checkpoint_config_id: Some(config.id.as_str()),
                parent_checkpoint_id: None,
                turn_index: None,
                kind: "Incident",
                timestamp: &timestamp,
                incident: Some(&warning_value),
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

            let warning_persisted = persist_checkpoint(&tx, &signing_key, &warning_checkpoint)?;
            prev_chain = warning_persisted.curr_chain;
            // Continue execution despite warning
        }

        // Check network policy before executing non-stub checkpoints
        if config.model.as_deref() != Some(STUB_MODEL_ID) {
            if let Err(network_incident) = governance::enforce_network_policy(&policy) {
                let incident_value = serde_json::to_value(&network_incident)?;
                let checkpoint_insert = CheckpointInsert {
                    run_id,
                    run_execution_id: execution_record.id.as_str(),
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
                persist_checkpoint(&tx, &signing_key, &checkpoint_insert)?;
                break;
            }
        }

        // Execute the checkpoint - handle typed steps with chaining
        let execution = if let Some(ref config_json_str) = config.config_json {
            // Try to parse as typed StepConfig
            if DEBUG_STEP_EXECUTION {
                eprintln!("🔍 Attempting to parse config_json: {}", config_json_str);
            }
            match serde_json::from_str::<StepConfig>(config_json_str) {
                Ok(step_config) => {
                    if DEBUG_STEP_EXECUTION {
                        eprintln!("✅ Successfully parsed typed step: {:?}", step_config);
                    }
                    // Execute based on step type
                    match step_config {
                    StepConfig::Ingest { source_path, format, privacy_status } => {
                        // Build DocumentIngestionConfig JSON for the ingestion function
                        let ingestion_config = DocumentIngestionConfig {
                            source_path,
                            format,
                            privacy_status,
                            output_storage: "database".to_string(),
                        };
                        let ingestion_json = serde_json::to_string(&ingestion_config)?;
                        execute_document_ingestion_checkpoint(&ingestion_json)?
                    }
                    StepConfig::Summarize {
                        source_step,
                        model,
                        summary_type,
                        custom_instructions,
                        token_budget: _,
                        proof_mode: _,
                        epsilon: _,
                    } => {
                        // Resolve source step if specified
                        if let Some(source_idx) = source_step {
                            let source = prior_outputs.get(&source_idx).ok_or_else(|| {
                                anyhow!(
                                    "Step {} references non-existent source step {}",
                                    config.order_index,
                                    source_idx
                                )
                            })?;

                            // Build summary prompt
                            let prompt = build_summary_prompt(
                                source,
                                &summary_type,
                                custom_instructions.as_deref(),
                            )?;

                            // Execute based on model type (stub, mock, or real LLM)
                            if model == STUB_MODEL_ID {
                                execute_stub_checkpoint(stored_run.seed, config.order_index, &prompt)
                            } else if model.starts_with(CLAUDE_MODEL_PREFIX) {
                                execute_claude_mock_checkpoint(&model, &prompt)?
                            } else {
                                execute_llm_checkpoint(&model, &prompt, llm_client)?
                            }
                        } else {
                            return Err(anyhow!(
                                "Summarize step {} requires a source_step",
                                config.order_index
                            ));
                        }
                    }
                    StepConfig::Prompt {
                        model,
                        prompt,
                        use_output_from,
                        token_budget: _,
                        proof_mode: _,
                        epsilon: _,
                    } => {
                        // Optionally use output from previous step
                        let final_prompt = if let Some(source_idx) = use_output_from {
                            let source = prior_outputs.get(&source_idx).ok_or_else(|| {
                                anyhow!(
                                    "Step {} references non-existent source step {}",
                                    config.order_index,
                                    source_idx
                                )
                            })?;
                            if DEBUG_STEP_EXECUTION {
                                eprintln!("🔗 Prompt step {} using output from step {}", config.order_index, source_idx);
                                eprintln!("   Source output length: {} chars", source.output_text.len());
                                eprintln!("   Source output preview: {}",
                                    if source.output_text.len() > 200 {
                                        format!("{}...", &source.output_text[..200])
                                    } else {
                                        source.output_text.clone()
                                    });
                            }
                            let context_prompt = build_prompt_with_context(&prompt, source);
                            if DEBUG_STEP_EXECUTION {
                                eprintln!("   Final prompt length: {} chars", context_prompt.len());
                            }
                            context_prompt
                        } else {
                            if DEBUG_STEP_EXECUTION {
                                eprintln!("🔗 Prompt step {} running standalone (no context)", config.order_index);
                            }
                            prompt.clone()
                        };

                        // Execute based on model type (stub, mock, or real LLM)
                        if model == STUB_MODEL_ID {
                            execute_stub_checkpoint(stored_run.seed, config.order_index, &final_prompt)
                        } else if model.starts_with(CLAUDE_MODEL_PREFIX) {
                            execute_claude_mock_checkpoint(&model, &final_prompt)?
                        } else {
                            execute_llm_checkpoint(&model, &final_prompt, llm_client)?
                        }
                    }
                    }
                }
                Err(parse_err) => {
                    if DEBUG_STEP_EXECUTION {
                        eprintln!("❌ Failed to parse as typed step: {}", parse_err);
                        eprintln!("   Falling back to legacy execution");
                    }
                    // Not a typed config, use legacy execution
                    execute_checkpoint(config, stored_run.seed, llm_client)?
                }
            }
        } else {
            // No config_json, use legacy execution
            execute_checkpoint(config, stored_run.seed, llm_client)?
        };

        let total_usage = execution.usage.total();
        cumulative_usage_tokens = cumulative_usage_tokens.saturating_add(total_usage);
        let prompt_tokens = execution.usage.prompt_tokens;
        let completion_tokens = execution.usage.completion_tokens;
        let mut incident_value: Option<serde_json::Value> = None;

        let budget_outcome = governance::enforce_budget(config.token_budget, total_usage);

        let (kind, inputs_sha, outputs_sha, semantic_digest) = match budget_outcome {
            Ok(_) => {
                let semantic = if config.proof_mode.is_concordant() {
                    Some(execution.semantic_digest.clone().ok_or_else(|| {
                        anyhow!("semantic digest missing for concordant checkpoint")
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
            run_execution_id: execution_record.id.as_str(),
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

        // Store step output for chaining (only if execution was successful)
        if kind == "Step" {
            let step_output = StepOutput {
                order_index: config.order_index as usize,
                step_type: config.step_type.clone(),
                output_text: execution.output_payload.clone().unwrap_or_default(),
                output_json: execution.output_payload.as_ref().and_then(|s| serde_json::from_str(s).ok()),
                outputs_sha256: execution.outputs_sha256.clone().unwrap_or_default(),
            };
            prior_outputs.insert(config.order_index as usize, step_output);
        }
    }

    tx.commit()?;
    Ok(execution_record)
}

pub fn clone_run(pool: &DbPool, source_run_id: &str) -> anyhow::Result<String> {
    let source_run = {
        let conn = pool.get()?;
        load_stored_run(&conn, source_run_id)?
    };

    if source_run.steps.is_empty() {
        return Err(anyhow!(
            "Cannot clone a run with no checkpoints. Add a checkpoint before cloning."
        ));
    }

    let spec_templates: Vec<RunStepTemplate> = source_run
        .steps
        .iter()
        .map(|cfg| RunStepTemplate {
            step_type: cfg.step_type.clone(),
            model: cfg.model.clone(),
            prompt: cfg.prompt.clone(),
            token_budget: cfg.token_budget,
            proof_mode: cfg.proof_mode,
            epsilon: cfg.epsilon,
            config_json: cfg.config_json.clone(),
            order_index: Some(cfg.order_index),
            checkpoint_type: cfg.checkpoint_type.clone(),
        })
        .collect();

    let clone_name = format!("{} (clone)", source_run.name);
    create_run(
        pool,
        &source_run.project_id,
        &clone_name,
        source_run.proof_mode.unwrap_or_default(),
        source_run.epsilon,
        source_run.seed,
        source_run.token_budget,
        &source_run.default_model,
        spec_templates,
    )
}

/// Truncate a string to a maximum size for database storage
fn truncate_payload(content: &str, max_size: usize) -> String {
    if content.len() <= max_size {
        return content.to_string();
    }

    let truncated = &content[..max_size];
    format!("{}... [TRUNCATED - {} total bytes]", truncated, content.len())
}

/// Execute a document ingestion checkpoint
pub(crate) fn execute_document_ingestion_checkpoint(
    config_json: &str,
) -> anyhow::Result<NodeExecution> {
    use crate::document_processing;

    // Parse the configuration
    let ingestion_config: DocumentIngestionConfig = serde_json::from_str(config_json)
        .context("Failed to parse document ingestion config")?;

    // Process the document based on format
    let canonical_doc = match ingestion_config.format.to_lowercase().as_str() {
        "pdf" => {
            document_processing::process_pdf_to_canonical(
                &ingestion_config.source_path,
                Some(ingestion_config.privacy_status.clone())
            )?
        }
        "tex" | "latex" => {
            document_processing::process_latex_to_canonical(
                &ingestion_config.source_path,
                Some(ingestion_config.privacy_status.clone())
            )?
        }
        "txt" => {
            document_processing::process_txt_to_canonical(
                &ingestion_config.source_path,
                Some(ingestion_config.privacy_status.clone())
            )?
        }
        "docx" | "doc" => {
            document_processing::process_docx_to_canonical(
                &ingestion_config.source_path,
                Some(ingestion_config.privacy_status.clone())
            )?
        }
        unsupported => {
            return Err(anyhow!(
                "Unsupported document format: {}. Supported formats: pdf, latex, txt, docx",
                unsupported
            ));
        }
    };

    // Serialize to JSON
    let canonical_json = serde_json::to_string_pretty(&canonical_doc)
        .context("Failed to serialize canonical document")?;

    // Create preview for database storage
    let preview = truncate_payload(&canonical_json, MAX_PAYLOAD_PREVIEW_SIZE);

    // Compute provenance hashes
    let inputs_sha256 = provenance::sha256_hex(ingestion_config.source_path.as_bytes());
    let outputs_sha256 = provenance::sha256_hex(canonical_json.as_bytes());

    // Use document_id as semantic digest
    let semantic_digest = canonical_doc.document_id.clone();

    // Create input description
    let prompt_payload = format!(
        "Document: {} (format: {}, privacy: {})",
        ingestion_config.source_path,
        ingestion_config.format,
        ingestion_config.privacy_status
    );

    Ok(NodeExecution {
        inputs_sha256: Some(inputs_sha256),
        outputs_sha256: Some(outputs_sha256),
        semantic_digest: Some(semantic_digest),
        usage: TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
        },
        prompt_payload: Some(prompt_payload),
        output_payload: Some(preview),
    })
}

/// Extract text content from a step output
/// For ingest steps: extracts cleaned_text from CanonicalDocument
/// For LLM steps: uses the output_text directly
fn extract_text_from_output(output: &StepOutput) -> anyhow::Result<String> {
    // If output is CanonicalDocument JSON, extract cleaned text
    if let Some(json) = &output.output_json {
        if let Some(cleaned_text) = json.get("cleaned_text_with_markdown_structure") {
            if let Some(text) = cleaned_text.as_str() {
                return Ok(text.to_string());
            }
        }
    }

    // Otherwise just use the text output
    Ok(output.output_text.clone())
}

/// Build prompt for summarization based on summary type
fn build_summary_prompt(
    source: &StepOutput,
    summary_type: &str,
    custom_instructions: Option<&str>,
) -> anyhow::Result<String> {
    let base_prompt = match summary_type {
        "brief" => "Provide a brief 2-3 sentence summary of the following:\n\n",
        "detailed" => "Provide a comprehensive summary covering all main points of:\n\n",
        "academic" => "Provide an academic summary including methodology, findings, and conclusions of:\n\n",
        "custom" => custom_instructions.unwrap_or("Summarize the following:\n\n"),
        _ => "Summarize the following:\n\n",
    };

    let source_text = extract_text_from_output(source)?;

    Ok(format!("{}{}", base_prompt, source_text))
}

/// Build prompt with context from previous step
fn build_prompt_with_context(prompt: &str, source: &StepOutput) -> String {
    format!(
        "{}\n\n--- Context from previous step ---\n{}",
        prompt,
        source.output_text
    )
}

fn execute_checkpoint(
    config: &RunStep,
    run_seed: u64,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<NodeExecution> {
    // Check if this is a document ingestion step
    if config.is_document_ingestion() {
        let config_json = config.config_json.as_ref()
            .ok_or_else(|| anyhow!("Document ingestion step missing config_json"))?;
        return execute_document_ingestion_checkpoint(config_json);
    }

    // For LLM steps, model and prompt must be present
    let model = config.model.as_ref()
        .ok_or_else(|| anyhow!("LLM step missing model"))?;
    let prompt = config.prompt.as_ref()
        .ok_or_else(|| anyhow!("LLM step missing prompt"))?;

    if model == STUB_MODEL_ID {
        Ok(execute_stub_checkpoint(
            run_seed,
            config.order_index,
            prompt,
        ))
    } else if model.starts_with(CLAUDE_MODEL_PREFIX) {
        execute_claude_mock_checkpoint(model, prompt)
    } else {
        execute_llm_checkpoint(model, prompt, llm_client)
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

fn execute_claude_mock_checkpoint(model: &str, prompt: &str) -> anyhow::Result<NodeExecution> {
    // Mock Claude API response - requires network access policy
    // In production, would use actual Claude API with user-configured key
    let mock_response = format!(
        "[MOCK CLAUDE RESPONSE - Model: {}]\n\nThis is a simulated response from Claude. \
        In production, this would make a real API call to Anthropic's servers using your configured API key.\n\n\
        Your prompt was: {}",
        model,
        if prompt.len() > 100 { &format!("{}...", &prompt[..100]) } else { prompt }
    );

    let inputs_hex = provenance::sha256_hex(prompt.as_bytes());
    let outputs_hex = provenance::sha256_hex(mock_response.as_bytes());
    let semantic_digest = provenance::semantic_digest(&mock_response);
    let prompt_payload = sanitize_payload(prompt);
    let output_payload = sanitize_payload(&mock_response);

    // Estimate token usage based on text length (rough approximation)
    let prompt_tokens = (prompt.len() / 4).max(1) as u64;
    let completion_tokens = (mock_response.len() / 4).max(1) as u64;

    Ok(NodeExecution {
        inputs_sha256: Some(inputs_hex),
        outputs_sha256: Some(outputs_hex),
        semantic_digest: Some(semantic_digest),
        usage: TokenUsage {
            prompt_tokens,
            completion_tokens,
        },
        prompt_payload: Some(prompt_payload),
        output_payload: Some(output_payload),
    })
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

pub fn create_run_step(
    pool: &DbPool,
    run_id: &str,
    config: RunStepRequest,
) -> anyhow::Result<RunStep> {
    let mut conn = pool.get()?;
    let tx = conn.transaction()?;

    // First, check if the parent run exists.
    let exists: Option<()> = tx
        .query_row("SELECT 1 FROM runs WHERE id = ?1", params![run_id], |_| {
            Ok(())
        })
        .optional()?;
    if exists.is_none() {
        return Err(anyhow!(format!("run {run_id} not found")));
    }

    // Determine the correct order_index for the new step.
    let checkpoint_type = config.checkpoint_type.unwrap_or_else(|| "Step".to_string());
    let order_index = if let Some(index) = config.order_index {
        // If an index is provided, shift subsequent steps.
        tx.execute(
            "UPDATE run_steps SET order_index = order_index + 1, updated_at = CURRENT_TIMESTAMP WHERE run_id = ?1 AND order_index >= ?2",
            params![run_id, index],
        )?;
        index
    } else {
        // Otherwise, append it to the end.
        tx.query_row(
            "SELECT COALESCE(MAX(order_index), -1) + 1 FROM run_steps WHERE run_id = ?1",
            params![run_id],
            |row| row.get::<_, i64>(0),
        )?
    };

    let step_id = Uuid::new_v4().to_string();
    let RunStepRequest {
        step_type,
        model,
        prompt,
        token_budget,
        proof_mode,
        epsilon,
        config_json,
        ..
    } = config;

    let step_type = step_type.unwrap_or_else(|| "llm".to_string());

    // Validate config_json if provided (for typed step system)
    if let Some(ref json_str) = config_json {
        // Try to parse as StepConfig to validate structure
        let parsed_config: Result<StepConfig, _> = serde_json::from_str(json_str);
        if let Ok(step_config) = parsed_config {
            // Verify that the step_type tag matches the parsed variant
            let expected_type = match step_config {
                StepConfig::Ingest { .. } => "ingest",
                StepConfig::Summarize { .. } => "summarize",
                StepConfig::Prompt { .. } => "prompt",
            };

            if step_type != expected_type {
                return Err(anyhow!(
                    "step_type '{}' doesn't match config variant '{}'",
                    step_type,
                    expected_type
                ));
            }
        }
        // If parsing fails, it's okay - might be legacy config or other format
    }

    // Validate epsilon for concordant mode (only for LLM steps).
    let validated_epsilon = if proof_mode.is_concordant() {
        let value = epsilon.ok_or_else(|| anyhow!("concordant steps require an epsilon"))?;
        if !value.is_finite() || value < 0.0 {
            return Err(anyhow!("epsilon must be a finite, non-negative value"));
        }
        Some(value)
    } else {
        None
    };

    // Insert the new step into the database.
    tx.execute(
        "INSERT INTO run_steps (id, run_id, order_index, checkpoint_type, step_type, model, prompt, token_budget, proof_mode, epsilon, config_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            &step_id,
            run_id,
            order_index,
            &checkpoint_type,
            &step_type,
            &model,
            &prompt,
            (token_budget as i64),
            proof_mode.as_str(),
            validated_epsilon,
            &config_json,
        ],
    )?;

    tx.commit()?;

    // Return the complete RunStep object.
    Ok(RunStep {
        id: step_id,
        run_id: run_id.to_string(),
        order_index,
        checkpoint_type,
        step_type,
        model,
        prompt,
        token_budget,
        proof_mode,
        epsilon: validated_epsilon,
        config_json,
    })
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

        let run_name = "hello-run";
        let seed = 42_u64;
        let token_budget = 1_000_u64;
        let step_template = RunStepTemplate {
            model: STUB_MODEL_ID.to_string(),
            prompt: "{\"nodes\":[]}".to_string(),
            token_budget,
            order_index: Some(0),
            checkpoint_type: "Step".to_string(),
            proof_mode: RunProofMode::Exact,
            epsilon: None,
        };
        let run_id = start_hello_run(
            &pool,
            project_id,
            run_name,
            RunProofMode::Exact,
            None,
            seed,
            token_budget,
            STUB_MODEL_ID,
            vec![step_template.clone()],
        )?;

        let conn = pool.get()?;
        let (
            project_id_db,
            name_db,
            proof_mode_db,
            epsilon_db,
            seed_db,
            token_budget_db,
            default_model_db,
            created_at_db,
        ): (String, String, String, Option<f64>, i64, i64, String, String) = conn.query_row(
            "SELECT project_id, name, proof_mode, epsilon, seed, token_budget, default_model, created_at FROM runs WHERE id = ?1",
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
                ))
            },
        )?;

        assert_eq!(project_id_db, project_id);
        assert_eq!(name_db, run_name);
        assert_eq!(proof_mode_db, RunProofMode::Exact.as_str());
        assert_eq!(epsilon_db, None);
        assert_eq!(seed_db as u64, seed);
        assert_eq!(token_budget_db as u64, token_budget);
        assert_eq!(default_model_db, STUB_MODEL_ID);
        assert!(!created_at_db.is_empty());

        let stored_step: (String, String, i64, i64, String, String, Option<f64>) = conn.query_row(
            "SELECT model, prompt, token_budget, order_index, checkpoint_type, proof_mode, epsilon FROM run_steps WHERE run_id = ?1",
            params![&run_id],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            )),
        )?;
        assert_eq!(stored_step.0, STUB_MODEL_ID);
        assert_eq!(stored_step.1, step_template.prompt);
        assert_eq!(stored_step.2.max(0) as u64, token_budget);
        assert_eq!(stored_step.3, 0);
        assert_eq!(stored_step.4, "Step");
        assert_eq!(stored_step.5, RunProofMode::Exact.as_str());
        assert!(stored_step.6.is_none());

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
        input_bytes.extend_from_slice(&seed.to_le_bytes());
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

        let run_name = "recover-secret";
        let proof_mode = RunProofMode::Exact;
        let seed = 99_u64;
        let token_budget = 25_u64;
        let step_template = RunStepTemplate {
            model: STUB_MODEL_ID.to_string(),
            prompt: "{}".to_string(),
            token_budget,
            order_index: Some(0),
            checkpoint_type: "Step".to_string(),
            proof_mode,
            epsilon: None,
        };

        let run_id = start_hello_run(
            &pool,
            project_id,
            run_name,
            proof_mode,
            None,
            seed,
            token_budget,
            STUB_MODEL_ID,
            vec![step_template],
        )?;
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
}
