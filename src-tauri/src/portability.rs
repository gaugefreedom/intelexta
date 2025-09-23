use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use zip::write::FileOptions;

use crate::{
    car, governance, provenance, replay,
    store::{self, policies::Policy},
    DbPool, Error, Project,
};

#[derive(Debug, Serialize, Deserialize)]
struct ManifestEntry {
    path: String,
    kind: String,
    sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExportManifest {
    version: u32,
    project_id: String,
    exported_at: String,
    entries: Vec<ManifestEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RunRecord {
    id: String,
    project_id: String,
    name: String,
    created_at: String,
    kind: String,
    spec_json: String,
    sampler_json: Option<String>,
    seed: i64,
    epsilon: Option<f64>,
    token_budget: i64,
    default_model: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckpointMessageExport {
    role: String,
    body: String,
    created_at: String,
    updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckpointPayloadExport {
    prompt_payload: Option<String>,
    output_payload: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckpointExport {
    id: String,
    run_id: String,
    checkpoint_config_id: Option<String>,
    parent_checkpoint_id: Option<String>,
    turn_index: Option<u32>,
    kind: String,
    incident_json: Option<serde_json::Value>,
    timestamp: String,
    inputs_sha256: Option<String>,
    outputs_sha256: Option<String>,
    prev_chain: Option<String>,
    curr_chain: String,
    signature: String,
    usage_tokens: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    semantic_digest: Option<String>,
    message: Option<CheckpointMessageExport>,
    payload: Option<CheckpointPayloadExport>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReceiptExport {
    id: String,
    run_id: String,
    created_at: String,
    match_kind: Option<String>,
    epsilon: Option<f64>,
    s_grade: Option<i64>,
    car_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RunExport {
    run: RunRecord,
    checkpoint_configs: Vec<crate::orchestrator::RunCheckpointConfig>,
    checkpoints: Vec<CheckpointExport>,
    receipts: Vec<ReceiptExport>,
}

#[derive(Debug)]
struct CarAttachment {
    zip_path: String,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct PendingEntry {
    path: String,
    kind: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct ProjectImportSummary {
    pub project: Project,
    pub runs_imported: usize,
    pub checkpoints_imported: usize,
    pub receipts_imported: usize,
    pub incidents_generated: usize,
}

fn sanitize_for_file(input: &str) -> String {
    let mut sanitized = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    sanitized.trim_matches('_').to_string()
}

fn append_entry(
    entries: &mut Vec<PendingEntry>,
    manifest: &mut Vec<ManifestEntry>,
    path: String,
    kind: &str,
    bytes: Vec<u8>,
) {
    let sha = provenance::sha256_hex(&bytes);
    manifest.push(ManifestEntry {
        path: path.clone(),
        kind: kind.to_string(),
        sha256: sha,
    });
    entries.push(PendingEntry {
        path,
        kind: kind.to_string(),
        bytes,
    });
}

fn load_project(conn: &Connection, project_id: &str) -> Result<Project, Error> {
    conn.query_row(
        "SELECT id, name, created_at, pubkey FROM projects WHERE id = ?1",
        params![project_id],
        |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                pubkey: row.get(3)?,
            })
        },
    )
    .map_err(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => {
            Error::Api(format!("project {project_id} not found"))
        }
        other => other.into(),
    })
}

fn load_runs_for_export(
    conn: &Connection,
    project_id: &str,
) -> Result<(Vec<RunExport>, Vec<CarAttachment>), Error> {
    let mut runs_stmt = conn.prepare(
        "SELECT id, project_id, name, created_at, kind, spec_json, sampler_json, seed, epsilon, token_budget, default_model
         FROM runs WHERE project_id = ?1 ORDER BY created_at ASC",
    )?;

    let mut runs_iter = runs_stmt.query_map(params![project_id], |row| {
        Ok(RunRecord {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            created_at: row.get(3)?,
            kind: row.get(4)?,
            spec_json: row.get(5)?,
            sampler_json: row.get(6)?,
            seed: row.get(7)?,
            epsilon: row.get(8)?,
            token_budget: row.get(9)?,
            default_model: row.get(10)?,
        })
    })?;

    let mut exports = Vec::new();
    let mut attachments = Vec::new();

    while let Some(run) = runs_iter.next() {
        let run = run?;

        let checkpoint_configs = {
            let mut stmt = conn.prepare(
                "SELECT id, run_id, order_index, checkpoint_type, model, prompt, token_budget, proof_mode
                 FROM run_checkpoints WHERE run_id = ?1 ORDER BY order_index ASC",
            )?;
            let rows = stmt.query_map(params![&run.id], |row| {
                let token_budget: i64 = row.get(6)?;
                let proof_mode_raw: String = row.get(7)?;
                let proof_mode = crate::orchestrator::RunProofMode::try_from(
                    proof_mode_raw.as_str(),
                )
                .map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        7,
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?;
                Ok(crate::orchestrator::RunCheckpointConfig {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    order_index: row.get(2)?,
                    checkpoint_type: row.get(3)?,
                    model: row.get(4)?,
                    prompt: row.get(5)?,
                    token_budget: token_budget.max(0) as u64,
                    proof_mode,
                })
            })?;
            let mut configs = Vec::new();
            for entry in rows {
                configs.push(entry?);
            }
            configs
        };

        let checkpoints = {
            let mut stmt = conn.prepare(
                "SELECT c.id, c.run_id, c.checkpoint_config_id, c.parent_checkpoint_id, c.turn_index, c.kind,
                        c.incident_json, c.timestamp, c.inputs_sha256, c.outputs_sha256, c.prev_chain, c.curr_chain,
                        c.signature, c.usage_tokens, c.prompt_tokens, c.completion_tokens, c.semantic_digest,
                        m.role, m.body, m.created_at, m.updated_at,
                        p.prompt_payload, p.output_payload, p.created_at, p.updated_at
                 FROM checkpoints c
                 LEFT JOIN checkpoint_messages m ON m.checkpoint_id = c.id
                 LEFT JOIN checkpoint_payloads p ON p.checkpoint_id = c.id
                 WHERE c.run_id = ?1
                 ORDER BY c.timestamp ASC",
            )?;

            let rows = stmt.query_map(params![&run.id], |row| {
                let incident_json: Option<String> = row.get(6)?;
                let incident = incident_json
                    .map(|payload| serde_json::from_str(&payload))
                    .transpose()
                    .map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(6, Type::Text, Box::new(err))
                    })?;
                let turn_index = row
                    .get::<_, Option<i64>>(4)?
                    .map(|value| value.max(0) as u32);
                let usage_tokens: i64 = row.get(13)?;
                let prompt_tokens: i64 = row.get(14)?;
                let completion_tokens: i64 = row.get(15)?;
                let message_role: Option<String> = row.get(17)?;
                let message_body: Option<String> = row.get(18)?;
                let message_created_at: Option<String> = row.get(19)?;
                let message_updated_at: Option<String> = row.get(20)?;
                let payload_prompt: Option<String> = row.get(21)?;
                let payload_output: Option<String> = row.get(22)?;
                let payload_created: Option<String> = row.get(23)?;
                let payload_updated: Option<String> = row.get(24)?;

                Ok(CheckpointExport {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    checkpoint_config_id: row.get(2)?,
                    parent_checkpoint_id: row.get(3)?,
                    turn_index,
                    kind: row.get(5)?,
                    incident_json: incident,
                    timestamp: row.get(7)?,
                    inputs_sha256: row.get(8)?,
                    outputs_sha256: row.get(9)?,
                    prev_chain: row.get(10)?,
                    curr_chain: row.get(11)?,
                    signature: row.get(12)?,
                    usage_tokens: usage_tokens.max(0) as u64,
                    prompt_tokens: prompt_tokens.max(0) as u64,
                    completion_tokens: completion_tokens.max(0) as u64,
                    semantic_digest: row.get(16)?,
                    message: match (message_role, message_body, message_created_at) {
                        (Some(role), Some(body), Some(created_at)) => {
                            Some(CheckpointMessageExport {
                                role,
                                body,
                                created_at,
                                updated_at: message_updated_at,
                            })
                        }
                        _ => None,
                    },
                    payload: match (payload_created, payload_updated) {
                        (Some(created_at), Some(updated_at)) => Some(CheckpointPayloadExport {
                            prompt_payload: payload_prompt,
                            output_payload: payload_output,
                            created_at,
                            updated_at,
                        }),
                        _ => None,
                    },
                })
            })?;

            let mut checkpoints = Vec::new();
            for entry in rows {
                checkpoints.push(entry?);
            }
            checkpoints
        };

        let (receipts, car_files) = {
            let mut stmt = conn.prepare(
                "SELECT id, run_id, created_at, file_path, match_kind, epsilon, s_grade
                 FROM receipts WHERE run_id = ?1",
            )?;
            let rows = stmt.query_map(params![&run.id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<f64>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                ))
            })?;

            let mut receipts = Vec::new();
            let mut cars = Vec::new();
            for row in rows {
                let (id, run_id, created_at, file_path, match_kind, epsilon, s_grade) = row?;
                let path = PathBuf::from(&file_path);
                let file_name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| {
                        Error::Api(format!("invalid receipt file path for {id}: {file_path}"))
                    })?;
                let zip_path = format!("cars/{file_name}");
                let bytes = fs::read(&path)
                    .map_err(|err| Error::Api(format!("failed to read CAR {file_path}: {err}")))?;
                receipts.push(ReceiptExport {
                    id: id.clone(),
                    run_id,
                    created_at,
                    match_kind,
                    epsilon,
                    s_grade,
                    car_path: Some(zip_path.clone()),
                });
                cars.push(CarAttachment { zip_path, bytes });
            }
            (receipts, cars)
        };

        attachments.extend(car_files.into_iter());

        exports.push(RunExport {
            run,
            checkpoint_configs,
            checkpoints,
            receipts,
        });
    }

    Ok((exports, attachments))
}

pub fn export_project_archive(
    pool: &DbPool,
    project_id: &str,
    base_dir: &Path,
) -> Result<PathBuf, Error> {
    let conn = pool.get()?;
    let project = load_project(&conn, project_id)?;
    let policy = store::policies::get(&conn, project_id)?;
    let (runs, attachments) = load_runs_for_export(&conn, project_id)?;

    let exports_dir = base_dir.join(project_id).join("exports");
    fs::create_dir_all(&exports_dir).map_err(|err| {
        Error::Api(format!(
            "failed to create export dir {}: {err}",
            exports_dir.display()
        ))
    })?;

    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let sanitized_name = sanitize_for_file(&project.name);
    let file_name = if sanitized_name.is_empty() {
        format!("{project_id}-{timestamp}.ixp")
    } else {
        format!("{sanitized_name}-{timestamp}.ixp")
    };
    let export_path = exports_dir.join(file_name);

    let mut manifest_entries = Vec::new();
    let mut pending_entries = Vec::new();

    let project_json = serde_json::to_vec_pretty(&project)
        .map_err(|err| Error::Api(format!("failed to serialize project: {err}")))?;
    append_entry(
        &mut pending_entries,
        &mut manifest_entries,
        "project.json".to_string(),
        "project",
        project_json,
    );

    let policy_json = serde_json::to_vec_pretty(&policy)
        .map_err(|err| Error::Api(format!("failed to serialize policy: {err}")))?;
    append_entry(
        &mut pending_entries,
        &mut manifest_entries,
        "policy.json".to_string(),
        "policy",
        policy_json,
    );

    for run in &runs {
        let run_path = format!("runs/{}.json", run.run.id);
        let run_json = serde_json::to_vec_pretty(run)
            .map_err(|err| Error::Api(format!("failed to serialize run {}: {err}", run.run.id)))?;
        append_entry(
            &mut pending_entries,
            &mut manifest_entries,
            run_path,
            "run",
            run_json,
        );
    }

    for attachment in &attachments {
        append_entry(
            &mut pending_entries,
            &mut manifest_entries,
            attachment.zip_path.clone(),
            "car",
            attachment.bytes.clone(),
        );
    }

    let manifest = ExportManifest {
        version: 1,
        project_id: project.id.clone(),
        exported_at: Utc::now().to_rfc3339(),
        entries: manifest_entries,
    };
    let manifest_json = serde_json::to_vec_pretty(&manifest)
        .map_err(|err| Error::Api(format!("failed to serialize manifest: {err}")))?;

    let file = fs::File::create(&export_path)
        .map_err(|err| Error::Api(format!("failed to create export file: {err}")))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in pending_entries {
        zip.start_file(entry.path, options)
            .map_err(|err| Error::Api(format!("failed to add zip entry: {err}")))?;
        zip.write_all(&entry.bytes)
            .map_err(|err| Error::Api(format!("failed to write zip entry: {err}")))?;
    }

    zip.start_file("manifest.json", options)
        .map_err(|err| Error::Api(format!("failed to add manifest: {err}")))?;
    zip.write_all(&manifest_json)
        .map_err(|err| Error::Api(format!("failed to write manifest: {err}")))?;
    zip.finish()
        .map_err(|err| Error::Api(format!("failed to finalize export archive: {err}")))?;

    Ok(export_path)
}

fn decode_verifying_key(pubkey_b64: &str) -> Result<VerifyingKey, Error> {
    let bytes = STANDARD
        .decode(pubkey_b64)
        .map_err(|err| Error::Api(format!("invalid project pubkey: {err}")))?;
    let array: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] = bytes
        .try_into()
        .map_err(|_| Error::Api("project pubkey has invalid length".to_string()))?;
    VerifyingKey::from_bytes(&array)
        .map_err(|err| Error::Api(format!("invalid verifying key: {err}")))
}

fn signature_valid(
    verifying_key: &VerifyingKey,
    curr_chain: &str,
    signature_b64: &str,
) -> Result<bool, Error> {
    let bytes = match STANDARD.decode(signature_b64) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err(Error::Api(format!(
                "failed to decode checkpoint signature: {err}"
            )))
        }
    };
    let array: [u8; ed25519_dalek::SIGNATURE_LENGTH] = bytes
        .try_into()
        .map_err(|_| Error::Api("checkpoint signature has invalid length".to_string()))?;
    let signature = Signature::from_bytes(&array);
    Ok(verifying_key
        .verify(curr_chain.as_bytes(), &signature)
        .is_ok())
}

fn ensure_incident(checkpoint: &mut CheckpointExport, incident: serde_json::Value) -> bool {
    if checkpoint.incident_json.is_none() {
        checkpoint.kind = "Incident".to_string();
        checkpoint.incident_json = Some(incident);
        true
    } else {
        false
    }
}

pub fn import_project_archive(
    pool: &DbPool,
    archive_path: &Path,
    base_dir: &Path,
) -> Result<ProjectImportSummary, Error> {
    let file = fs::File::open(archive_path).map_err(|err| {
        Error::Api(format!(
            "failed to open archive {}: {err}",
            archive_path.display()
        ))
    })?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|err| Error::Api(format!("failed to read archive: {err}")))?;

    let mut manifest_bytes = Vec::new();
    archive
        .by_name("manifest.json")
        .map_err(|err| Error::Api(format!("manifest not found in archive: {err}")))?
        .read_to_end(&mut manifest_bytes)
        .map_err(|err| Error::Api(format!("failed to read manifest: {err}")))?;
    let manifest: ExportManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| Error::Api(format!("failed to parse manifest: {err}")))?;

    let mut contents: HashMap<String, Vec<u8>> = HashMap::new();
    for entry in &manifest.entries {
        let mut data = Vec::new();
        archive
            .by_name(&entry.path)
            .map_err(|err| Error::Api(format!("missing archive entry {}: {err}", entry.path)))?
            .read_to_end(&mut data)
            .map_err(|err| Error::Api(format!("failed to read entry {}: {err}", entry.path)))?;
        let actual = provenance::sha256_hex(&data);
        if actual != entry.sha256 {
            return Err(Error::Api(format!(
                "checksum mismatch for {} (expected {}, got {})",
                entry.path, entry.sha256, actual
            )));
        }
        contents.insert(entry.path.clone(), data);
    }

    let project_bytes = contents
        .remove("project.json")
        .ok_or_else(|| Error::Api("project.json missing from archive".to_string()))?;
    let project: Project = serde_json::from_slice(&project_bytes)
        .map_err(|err| Error::Api(format!("failed to parse project: {err}")))?;

    let policy_bytes = contents
        .remove("policy.json")
        .ok_or_else(|| Error::Api("policy.json missing from archive".to_string()))?;
    let policy: Policy = serde_json::from_slice(&policy_bytes)
        .map_err(|err| Error::Api(format!("failed to parse policy: {err}")))?;

    let verifying_key = decode_verifying_key(&project.pubkey)?;

    let mut run_exports = Vec::new();
    for entry in &manifest.entries {
        if entry.kind == "run" {
            let data = contents
                .remove(&entry.path)
                .ok_or_else(|| Error::Api(format!("missing run payload {}", entry.path)))?;
            let run: RunExport = serde_json::from_slice(&data)
                .map_err(|err| Error::Api(format!("failed to parse {}: {err}", entry.path)))?;
            run_exports.push(run);
        }
    }

    let mut conn = pool.get()?;

    let project_exists: Option<()> = conn
        .query_row(
            "SELECT 1 FROM projects WHERE id = ?1",
            params![&project.id],
            |_| Ok(()),
        )
        .optional()?;
    if project_exists.is_some() {
        return Err(Error::Api(format!(
            "project {} already exists in this workspace",
            project.id
        )));
    }

    let mut tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO projects (id, name, created_at, pubkey) VALUES (?1, ?2, ?3, ?4)",
        params![
            &project.id,
            &project.name,
            &project.created_at.to_rfc3339(),
            &project.pubkey,
        ],
    )?;

    store::policies::upsert(tx.deref(), &project.id, &policy)?;

    let mut checkpoints_imported = 0usize;
    let mut receipts_imported = 0usize;
    let mut incidents_generated = 0usize;
    let mut file_writes: Vec<(PathBuf, Vec<u8>)> = Vec::new();

    let runs_imported_count = run_exports.len();

    for run in run_exports {
        if run.run.project_id != project.id {
            return Err(Error::Api(format!(
                "run {} references different project id {}",
                run.run.id, run.run.project_id
            )));
        }

        tx.execute(
            "INSERT INTO runs (id, project_id, name, created_at, kind, spec_json, sampler_json, seed, epsilon, token_budget, default_model)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &run.run.id,
                &run.run.project_id,
                &run.run.name,
                &run.run.created_at,
                &run.run.kind,
                &run.run.spec_json,
                &run.run.sampler_json,
                &run.run.seed,
                &run.run.epsilon,
                &run.run.token_budget,
                &run.run.default_model,
            ],
        )?;

        let config_budgets: HashMap<_, _> = run
            .checkpoint_configs
            .iter()
            .map(|cfg| (cfg.id.clone(), cfg.token_budget))
            .collect();

        for config in &run.checkpoint_configs {
            tx.execute(
                "INSERT INTO run_checkpoints (id, run_id, order_index, checkpoint_type, model, prompt, token_budget, proof_mode)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    &config.id,
                    &config.run_id,
                    config.order_index,
                    &config.checkpoint_type,
                    &config.model,
                    &config.prompt,
                    config.token_budget as i64,
                    config.proof_mode.as_str(),
                ],
            )?;
        }

        let mut checkpoints = run.checkpoints;
        let mut total_usage = 0u64;

        for checkpoint in &mut checkpoints {
            total_usage = total_usage.saturating_add(checkpoint.usage_tokens);

            if !signature_valid(
                &verifying_key,
                &checkpoint.curr_chain,
                &checkpoint.signature,
            )? {
                let generated = ensure_incident(
                    checkpoint,
                    serde_json::json!({
                        "kind": "signature_verification_failed",
                        "severity": "error",
                        "details": "checkpoint signature did not verify",
                        "relatedCheckpointId": checkpoint.id,
                    }),
                );
                if generated {
                    incidents_generated += 1;
                }
            }

            if let Some(cfg_id) = checkpoint.checkpoint_config_id.as_ref() {
                if let Some(budget) = config_budgets.get(cfg_id) {
                    if checkpoint.usage_tokens > *budget {
                        let generated = ensure_incident(
                            checkpoint,
                            serde_json::json!({
                                "kind": "checkpoint_budget_exceeded",
                                "severity": "error",
                                "details": format!(
                                    "usage={} > checkpoint_budget={}",
                                    checkpoint.usage_tokens, budget
                                ),
                                "relatedCheckpointId": checkpoint.id,
                            }),
                        );
                        if generated {
                            incidents_generated += 1;
                        }
                    }
                }
            }
        }

        if let Err(incident) = governance::enforce_budget(policy.budget_tokens, total_usage) {
            if let Some(last) = checkpoints.last_mut() {
                let generated = ensure_incident(
                    last,
                    serde_json::json!({
                        "kind": incident.kind,
                        "severity": incident.severity,
                        "details": incident.details,
                        "relatedCheckpointId": last.id,
                    }),
                );
                if generated {
                    incidents_generated += 1;
                }
            }
        }

        if total_usage > run.run.token_budget.max(0) as u64 {
            if let Some(last) = checkpoints.last_mut() {
                let generated = ensure_incident(
                    last,
                    serde_json::json!({
                        "kind": "run_budget_exceeded",
                        "severity": "error",
                        "details": format!(
                            "usage={} > run_budget={}",
                            total_usage,
                            run.run.token_budget.max(0)
                        ),
                        "relatedCheckpointId": last.id,
                    }),
                );
                if generated {
                    incidents_generated += 1;
                }
            }
        }

        for checkpoint in checkpoints {
            tx.execute(
                "INSERT INTO checkpoints (id, run_id, checkpoint_config_id, parent_checkpoint_id, turn_index, kind, incident_json, timestamp,
                                          inputs_sha256, outputs_sha256, prev_chain, curr_chain, signature, usage_tokens, prompt_tokens,
                                          completion_tokens, semantic_digest)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    &checkpoint.id,
                    &checkpoint.run_id,
                    &checkpoint.checkpoint_config_id,
                    &checkpoint.parent_checkpoint_id,
                    checkpoint.turn_index.map(|value| value as i64),
                    &checkpoint.kind,
                    checkpoint
                        .incident_json
                        .as_ref()
                        .map(|value| serde_json::to_string(value).unwrap()),
                    &checkpoint.timestamp,
                    &checkpoint.inputs_sha256,
                    &checkpoint.outputs_sha256,
                    &checkpoint.prev_chain,
                    &checkpoint.curr_chain,
                    &checkpoint.signature,
                    checkpoint.usage_tokens as i64,
                    checkpoint.prompt_tokens as i64,
                    checkpoint.completion_tokens as i64,
                    &checkpoint.semantic_digest,
                ],
            )?;

            if let Some(message) = checkpoint.message {
                tx.execute(
                    "INSERT INTO checkpoint_messages (checkpoint_id, role, body, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, COALESCE(?5, ?4))",
                    params![
                        &checkpoint.id,
                        &message.role,
                        &message.body,
                        &message.created_at,
                        &message.updated_at,
                    ],
                )?;
            }

            if let Some(payload) = checkpoint.payload {
                tx.execute(
                    "INSERT INTO checkpoint_payloads (checkpoint_id, prompt_payload, output_payload, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        &checkpoint.id,
                        &payload.prompt_payload,
                        &payload.output_payload,
                        &payload.created_at,
                        &payload.updated_at,
                    ],
                )?;
            }

            checkpoints_imported += 1;
        }

        for receipt in run.receipts {
            let dest_dir = base_dir.join(&project.id).join("receipts");
            let car_bytes = match receipt
                .car_path
                .as_ref()
                .and_then(|path| contents.get(path))
            {
                Some(bytes) => bytes.clone(),
                None => {
                    return Err(Error::Api(format!(
                        "missing CAR payload for receipt {}",
                        receipt.id
                    )))
                }
            };

            let car: car::Car = serde_json::from_slice(&car_bytes)
                .map_err(|err| Error::Api(format!("failed to parse CAR {}: {err}", receipt.id)))?;
            if car.id != receipt.id {
                return Err(Error::Api(format!(
                    "CAR {} has mismatched id {}",
                    receipt.id, car.id
                )));
            }
            if car.run_id != run.run.id {
                return Err(Error::Api(format!(
                    "CAR {} references run {} but archive contains run {}",
                    receipt.id, car.run_id, run.run.id
                )));
            }

            for signature in &car.signatures {
                let Some(encoded) = signature.strip_prefix("ed25519:") else {
                    continue;
                };
                if !signature_valid(&verifying_key, &car.id, encoded)? {
                    return Err(Error::Api(format!(
                        "CAR {} failed signature verification",
                        receipt.id
                    )));
                }
            }

            let dest_path = dest_dir.join(format!("{}.car.json", receipt.id));
            file_writes.push((dest_path.clone(), car_bytes));

            tx.execute(
                "INSERT INTO receipts (id, run_id, created_at, file_path, match_kind, epsilon, s_grade)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &receipt.id,
                    &receipt.run_id,
                    &receipt.created_at,
                    dest_path.to_string_lossy(),
                    &receipt.match_kind,
                    &receipt.epsilon,
                    receipt.s_grade,
                ],
            )?;
            receipts_imported += 1;
        }
    }

    let mut written_paths: Vec<PathBuf> = Vec::new();
    for (path, bytes) in &file_writes {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|err| {
                Error::Api(format!(
                    "failed to create directory {}: {err}",
                    dir.display()
                ))
            })?;
        }

        if let Err(err) = fs::write(path, bytes) {
            for written in written_paths {
                let _ = fs::remove_file(&written);
            }
            return Err(Error::Api(format!(
                "failed to write CAR {}: {err}",
                path.display()
            )));
        }

        written_paths.push(path.clone());
    }

    tx.commit()?;

    Ok(ProjectImportSummary {
        project,
        runs_imported: runs_imported_count,
        checkpoints_imported,
        receipts_imported,
        incidents_generated,
    })
}

pub fn import_car_file(
    pool: &DbPool,
    car_path: &Path,
    base_dir: &Path,
) -> Result<replay::ReplayReport, Error> {
    let data = fs::read_to_string(car_path)
        .map_err(|err| Error::Api(format!("failed to read CAR {}: {err}", car_path.display())))?;
    let car: car::Car = serde_json::from_str(&data)
        .map_err(|err| Error::Api(format!("failed to parse CAR: {err}")))?;

    let conn = pool.get()?;
    let (project_id, pubkey): (String, String) = conn
        .query_row(
            "SELECT projects.id, projects.pubkey FROM runs JOIN projects ON projects.id = runs.project_id WHERE runs.id = ?1",
            params![&car.run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Error::Api(format!(
                "run {} referenced by CAR not found",
                car.run_id
            )),
            other => other.into(),
        })?;

    let verifying_key = decode_verifying_key(&pubkey)?;

    for signature in &car.signatures {
        let Some(encoded) = signature.strip_prefix("ed25519:") else {
            continue;
        };
        if !signature_valid(&verifying_key, &car.id, encoded)? {
            return Err(Error::Api(format!(
                "CAR {} failed signature verification",
                car.id
            )));
        }
    }

    let receipts_dir = base_dir.join(&project_id).join("receipts");
    fs::create_dir_all(&receipts_dir).map_err(|err| {
        Error::Api(format!(
            "failed to create receipts dir {}: {err}",
            receipts_dir.display()
        ))
    })?;
    let dest_path = receipts_dir.join(format!("{}.car.json", car.id));
    fs::write(&dest_path, data)
        .map_err(|err| Error::Api(format!("failed to copy CAR to workspace: {err}")))?;

    conn.execute(
        "INSERT INTO receipts (id, run_id, created_at, file_path, match_kind, epsilon, s_grade)
         VALUES (?1, ?2, CURRENT_TIMESTAMP, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET run_id = excluded.run_id, created_at = excluded.created_at, file_path = excluded.file_path,
             match_kind = excluded.match_kind, epsilon = excluded.epsilon, s_grade = excluded.s_grade",
        params![
            &car.id,
            &car.run_id,
            dest_path.to_string_lossy(),
            &car.proof.match_kind,
            &car.proof.epsilon,
            i64::from(car.sgrade.score),
        ],
    )?;

    crate::api::replay_run_with_pool(car.run_id.clone(), pool)
}
