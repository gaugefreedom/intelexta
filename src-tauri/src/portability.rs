use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
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
    sampler_json: Option<String>,
    seed: i64,
    epsilon: Option<f64>,
    token_budget: i64,
    default_model: String,
    proof_mode: crate::orchestrator::RunProofMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy_version: Option<i64>,
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
    #[serde(default)]
    run_execution_id: Option<String>,
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
pub(crate) struct PolicyVersionExport {
    id: i64,
    project_id: String,
    version: i64,
    policy_json: String,
    created_at: String,
    created_by: Option<String>,
    change_notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RunExecutionExport {
    id: String,
    run_id: String,
    created_at: String,
    checkpoints: Vec<CheckpointExport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RunExport {
    run: RunRecord,
    checkpoint_configs: Vec<crate::orchestrator::RunStep>,
    executions: Vec<RunExecutionExport>,
    receipts: Vec<ReceiptExport>,
}

#[derive(Debug)]
pub(crate) struct CarAttachment {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCarCheckpointSnapshot {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_checkpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_chain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curr_chain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCarBudgets {
    pub usd: f64,
    pub tokens: u64,
    pub nature_cost: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCarSnapshot {
    pub car_id: String,
    pub run_id: String,
    pub created_at: String,
    pub run: car::RunInfo,
    pub proof: car::Proof,
    pub policy_ref: car::PolicyRef,
    pub budgets: ImportedCarBudgets,
    pub provenance: Vec<car::ProvenanceClaim>,
    pub checkpoints: Vec<ImportedCarCheckpointSnapshot>,
    pub sgrade: car::SGrade,
    pub signer_public_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarImportResult {
    pub replay_report: replay::ReplayReport,
    pub snapshot: ImportedCarSnapshot,
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

pub(crate) fn load_project(conn: &Connection, project_id: &str) -> Result<Project, Error> {
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

pub(crate) fn load_policy_versions_for_export(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<PolicyVersionExport>, Error> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, version, policy_json, created_at, created_by, change_notes
         FROM policy_versions WHERE project_id = ?1 ORDER BY version ASC",
    )?;

    let rows = stmt.query_map(params![project_id], |row| {
        Ok(PolicyVersionExport {
            id: row.get(0)?,
            project_id: row.get(1)?,
            version: row.get(2)?,
            policy_json: row.get(3)?,
            created_at: row.get(4)?,
            created_by: row.get(5)?,
            change_notes: row.get(6)?,
        })
    })?;

    let mut versions = Vec::new();
    for row in rows {
        versions.push(row?);
    }

    Ok(versions)
}

pub(crate) fn load_runs_for_export(
    conn: &Connection,
    project_id: &str,
) -> Result<(Vec<RunExport>, Vec<CarAttachment>), Error> {
    let mut runs_stmt = conn.prepare(
        "SELECT id, project_id, name, created_at, sampler_json, seed, epsilon, token_budget, default_model, proof_mode, policy_version
         FROM runs WHERE project_id = ?1 ORDER BY created_at ASC",
    )?;

    let mut runs_iter = runs_stmt.query_map(params![project_id], |row| {
        let proof_mode_raw: String = row.get(9)?;
        let proof_mode = crate::orchestrator::RunProofMode::try_from(proof_mode_raw.as_str())
            .map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(9, Type::Text, Box::new(err))
            })?;
        let kind = if proof_mode.is_concordant() {
            "concordant".to_string()
        } else {
            "exact".to_string()
        };
        Ok(RunRecord {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            created_at: row.get(3)?,
            kind,
            sampler_json: row.get(4)?,
            seed: row.get(5)?,
            epsilon: row.get(6)?,
            token_budget: row.get(7)?,
            default_model: row.get(8)?,
            proof_mode,
            policy_version: row.get(10)?,
        })
    })?;

    let mut exports = Vec::new();
    let mut attachments = Vec::new();

    while let Some(run) = runs_iter.next() {
        let mut run = run?;

        // CHECKPOINT-FIRST APPROACH: First get all checkpoints to know which steps are needed
        let checkpoints_preview = {
            let mut stmt = conn.prepare(
                "SELECT checkpoint_config_id FROM checkpoints WHERE run_id = ?1 AND checkpoint_config_id IS NOT NULL",
            )?;
            let rows = stmt.query_map(params![&run.id], |row| {
                row.get::<_, String>(0)
            })?;
            let mut config_ids = std::collections::HashSet::new();
            for row in rows {
                config_ids.insert(row?);
            }
            config_ids
        };

        // Now fetch ONLY the run_steps that are actually referenced by checkpoints
        let checkpoint_configs = if checkpoints_preview.is_empty() {
            Vec::new()
        } else {
            let placeholders = checkpoints_preview.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "SELECT id, run_id, order_index, checkpoint_type, step_type, model, prompt, token_budget, proof_mode, epsilon, config_json
                 FROM run_steps WHERE run_id = ?1 AND id IN ({}) ORDER BY order_index ASC",
                placeholders
            );
            let mut stmt = conn.prepare(&query)?;
            let mut params: Vec<&dyn rusqlite::ToSql> = vec![&run.id];
            for config_id in &checkpoints_preview {
                params.push(config_id);
            }
            let rows = stmt.query_map(params.as_slice(), |row| {
                let token_budget: i64 = row.get(7)?;
                let proof_mode_raw: String = row.get(8)?;
                let proof_mode = crate::orchestrator::RunProofMode::try_from(
                    proof_mode_raw.as_str(),
                )
                .map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        8,
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?;
                Ok(crate::orchestrator::RunStep {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    order_index: row.get(2)?,
                    checkpoint_type: row.get(3)?,
                    step_type: row.get(4)?,
                    model: row.get(5)?,
                    prompt: row.get(6)?,
                    token_budget: token_budget.max(0) as u64,
                    proof_mode,
                    epsilon: row.get(9)?,
                    config_json: row.get(10)?,
                })
            })?;
            let mut configs = Vec::new();
            for entry in rows {
                configs.push(entry?);
            }
            configs
        };

        let has_concordant_step = checkpoint_configs
            .iter()
            .any(|cfg| cfg.proof_mode.is_concordant());
        run.kind = if run.proof_mode.is_concordant() || has_concordant_step {
            "concordant".to_string()
        } else {
            "exact".to_string()
        };

        // Get all run_executions for this run
        let executions = {
            let mut stmt = conn.prepare(
                "SELECT id, run_id, created_at FROM run_executions WHERE run_id = ?1 ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map(params![&run.id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            let mut execs = Vec::new();
            for row in rows {
                execs.push(row?);
            }
            execs
        };

        // For each execution, get its checkpoints
        let mut execution_exports = Vec::new();
        for (exec_id, exec_run_id, exec_created_at) in executions {
            let checkpoints = {
            let mut stmt = conn.prepare(
                "SELECT c.id, c.run_id, c.run_execution_id, c.checkpoint_config_id, c.parent_checkpoint_id, c.turn_index, c.kind,
                        c.incident_json, c.timestamp, c.inputs_sha256, c.outputs_sha256, c.prev_chain, c.curr_chain,
                        c.signature, c.usage_tokens, c.prompt_tokens, c.completion_tokens, c.semantic_digest,
                        m.role, m.body, m.created_at, m.updated_at,
                        p.prompt_payload, p.output_payload, p.created_at, p.updated_at
                 FROM checkpoints c
                 LEFT JOIN checkpoint_messages m ON m.checkpoint_id = c.id
                 LEFT JOIN checkpoint_payloads p ON p.checkpoint_id = c.id
                 WHERE c.run_execution_id = ?1
                 ORDER BY c.timestamp ASC",
            )?;

            let rows = stmt.query_map(params![&exec_id], |row| {
                let incident_json: Option<String> = row.get(7)?;
                let incident = incident_json
                    .map(|payload| serde_json::from_str(&payload))
                    .transpose()
                    .map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(7, Type::Text, Box::new(err))
                    })?;
                let turn_index = row
                    .get::<_, Option<i64>>(5)?
                    .map(|value| value.max(0) as u32);
                let usage_tokens: i64 = row.get(14)?;
                let prompt_tokens: i64 = row.get(15)?;
                let completion_tokens: i64 = row.get(16)?;
                let message_role: Option<String> = row.get(18)?;
                let message_body: Option<String> = row.get(19)?;
                let message_created_at: Option<String> = row.get(20)?;
                let message_updated_at: Option<String> = row.get(21)?;
                let payload_prompt: Option<String> = row.get(22)?;
                let payload_output: Option<String> = row.get(23)?;
                let payload_created: Option<String> = row.get(24)?;
                let payload_updated: Option<String> = row.get(25)?;

                Ok(CheckpointExport {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    run_execution_id: Some(row.get(2)?),
                    checkpoint_config_id: row.get(3)?,
                    parent_checkpoint_id: row.get(4)?,
                    turn_index,
                    kind: row.get(6)?,
                    incident_json: incident,
                    timestamp: row.get(8)?,
                    inputs_sha256: row.get(9)?,
                    outputs_sha256: row.get(10)?,
                    prev_chain: row.get(11)?,
                    curr_chain: row.get(12)?,
                    signature: row.get(13)?,
                    usage_tokens: usage_tokens.max(0) as u64,
                    prompt_tokens: prompt_tokens.max(0) as u64,
                    completion_tokens: completion_tokens.max(0) as u64,
                    semantic_digest: row.get(17)?,
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

            execution_exports.push(RunExecutionExport {
                id: exec_id,
                run_id: exec_run_id,
                created_at: exec_created_at,
                checkpoints,
            });
        }

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
            executions: execution_exports,
            receipts,
        });
    }

    Ok((exports, attachments))
}

/// Write project archive directly to the specified path
pub fn write_project_archive_to_path(
    export_path: &Path,
    project: &Project,
    policy: &Policy,
    policy_versions: &[PolicyVersionExport],
    runs: &[RunExport],
    attachments: &[CarAttachment],
) -> Result<(), Error> {
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

    // Export policy version history
    if !policy_versions.is_empty() {
        let policy_versions_json = serde_json::to_vec_pretty(&policy_versions)
            .map_err(|err| Error::Api(format!("failed to serialize policy versions: {err}")))?;
        append_entry(
            &mut pending_entries,
            &mut manifest_entries,
            "policy_versions.json".to_string(),
            "policy_versions",
            policy_versions_json,
        );
    }

    for run in runs {
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

    for attachment in attachments {
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

    let file = fs::File::create(export_path)
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

    Ok(())
}

pub fn export_project_archive(
    pool: &DbPool,
    project_id: &str,
    base_dir: &Path,
) -> Result<PathBuf, Error> {
    let conn = pool.get()?;
    let project = load_project(&conn, project_id)?;
    let policy = store::policies::get(&conn, project_id)?;
    let policy_versions = load_policy_versions_for_export(&conn, project_id)?;
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

    // Export policy version history
    if !policy_versions.is_empty() {
        let policy_versions_json = serde_json::to_vec_pretty(&policy_versions)
            .map_err(|err| Error::Api(format!("failed to serialize policy versions: {err}")))?;
        append_entry(
            &mut pending_entries,
            &mut manifest_entries,
            "policy_versions.json".to_string(),
            "policy_versions",
            policy_versions_json,
        );
    }

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
        .map_err(|err| Error::Api(format!("invalid verifying key: {err}")))?;
    let array: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] = bytes
        .try_into()
        .map_err(|_| Error::Api("verifying key has invalid length".to_string()))?;
    VerifyingKey::from_bytes(&array)
        .map_err(|err| Error::Api(format!("invalid verifying key material: {err}")))
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

/// Extract CAR JSON and attachments from either .car.json or .car.zip format
fn extract_car_data(
    car_bytes: &[u8],
    file_name: &str,
) -> Result<(car::Car, HashMap<String, Vec<u8>>), Error> {
    let mut attachments = HashMap::new();

    // Check if it's a zip file (starts with PK magic bytes)
    if car_bytes.len() >= 4 && &car_bytes[0..2] == b"PK" {
        // It's a zip file - extract car.json and attachments
        let cursor = std::io::Cursor::new(car_bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|err| Error::Api(format!("failed to read CAR zip {}: {err}", file_name)))?;

        // Read car.json
        let mut car_json_bytes = Vec::new();
        archive
            .by_name("car.json")
            .map_err(|err| Error::Api(format!("car.json not found in CAR zip {}: {err}", file_name)))?
            .read_to_end(&mut car_json_bytes)
            .map_err(|err| Error::Api(format!("failed to read car.json from {}: {err}", file_name)))?;

        let car: car::Car = serde_json::from_slice(&car_json_bytes)
            .map_err(|err| Error::Api(format!("failed to parse car.json from {}: {err}", file_name)))?;

        // Extract all attachments from attachments/ directory
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|err| Error::Api(format!("failed to read zip entry {}: {err}", i)))?;

            if file.name().starts_with("attachments/") && !file.is_dir() {
                let attachment_name = file.name().to_string();
                let mut attachment_bytes = Vec::new();
                file.read_to_end(&mut attachment_bytes)
                    .map_err(|err| Error::Api(format!("failed to read attachment {}: {err}", attachment_name)))?;

                // Extract hash from filename (attachments/{hash}.txt)
                if let Some(hash) = attachment_name
                    .strip_prefix("attachments/")
                    .and_then(|name| name.strip_suffix(".txt"))
                {
                    attachments.insert(hash.to_string(), attachment_bytes);
                }
            }
        }

        Ok((car, attachments))
    } else {
        // It's a plain JSON file
        let car: car::Car = serde_json::from_slice(car_bytes)
            .map_err(|err| Error::Api(format!("failed to parse CAR JSON {}: {err}", file_name)))?;
        Ok((car, attachments))
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

    // Load policy versions if available (optional for backwards compatibility)
    let policy_versions: Vec<PolicyVersionExport> = contents
        .remove("policy_versions.json")
        .map(|bytes| {
            serde_json::from_slice(&bytes)
                .map_err(|err| Error::Api(format!("failed to parse policy versions: {err}")))
        })
        .transpose()?
        .unwrap_or_default();

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

    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO projects (id, name, created_at, pubkey) VALUES (?1, ?2, ?3, ?4)",
        params![
            &project.id,
            &project.name,
            &project.created_at.to_rfc3339(),
            &project.pubkey,
        ],
    )?;

    // Insert policy directly into policies table without creating version history
    // (we'll restore the version history from the archive)
    let policy_json = serde_json::to_string(&policy)
        .map_err(|err| Error::Api(format!("failed to serialize policy: {err}")))?;

    let current_version = if !policy_versions.is_empty() {
        policy_versions.iter().map(|v| v.version).max().unwrap_or(1)
    } else {
        1
    };

    tx.execute(
        "INSERT INTO policies (project_id, policy_json, current_version) VALUES (?1, ?2, ?3)",
        params![&project.id, &policy_json, current_version],
    )?;

    // Import policy version history
    if !policy_versions.is_empty() {
        // We have version history - import it
        for policy_version in &policy_versions {
            tx.execute(
                "INSERT INTO policy_versions (id, project_id, version, policy_json, created_at, created_by, change_notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &policy_version.id,
                    &policy_version.project_id,
                    &policy_version.version,
                    &policy_version.policy_json,
                    &policy_version.created_at,
                    &policy_version.created_by,
                    &policy_version.change_notes,
                ],
            )?;
        }
    } else {
        // No version history in archive (old format) - create version 1 from current policy
        tx.execute(
            "INSERT INTO policy_versions (project_id, version, policy_json, created_by, change_notes)
             VALUES (?1, 1, ?2, 'import', 'Imported from IXP archive without version history')",
            params![&project.id, &policy_json],
        )?;
    }

    let mut checkpoints_imported = 0usize;
    let mut receipts_imported = 0usize;
    let mut incidents_generated = 0usize;
    let mut file_writes: Vec<(PathBuf, Vec<u8>)> = Vec::new();

    let runs_imported_count = run_exports.len();

    for mut run in run_exports {
        if run.run.project_id != project.id {
            return Err(Error::Api(format!(
                "run {} references different project id {}",
                run.run.id, run.run.project_id
            )));
        }

        tx.execute(
            "INSERT INTO runs (id, project_id, name, created_at, sampler_json, seed, epsilon, token_budget, default_model, proof_mode, policy_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &run.run.id,
                &run.run.project_id,
                &run.run.name,
                &run.run.created_at,
                &run.run.sampler_json,
                &run.run.seed,
                &run.run.epsilon,
                &run.run.token_budget,
                &run.run.default_model,
                run.run.proof_mode.as_str(),
                &run.run.policy_version,
            ],
        )?;

        // Import all run_executions for this run
        for execution in &run.executions {
            tx.execute(
                "INSERT INTO run_executions (id, run_id, created_at) VALUES (?1, ?2, ?3)",
                params![
                    &execution.id,
                    &execution.run_id,
                    &execution.created_at,
                ],
            )?;
        }

        let config_budgets: HashMap<_, _> = run
            .checkpoint_configs
            .iter()
            .map(|cfg| (cfg.id.clone(), cfg.token_budget))
            .collect();

        // Track which step IDs we're inserting for validation
        let inserted_step_ids: std::collections::HashSet<String> = run
            .checkpoint_configs
            .iter()
            .map(|cfg| cfg.id.clone())
            .collect();

        for config in &run.checkpoint_configs {
            tx.execute(
                "INSERT INTO run_steps (id, run_id, order_index, checkpoint_type, step_type, model, prompt, token_budget, proof_mode, epsilon, config_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    &config.id,
                    &config.run_id,
                    config.order_index,
                    &config.checkpoint_type,
                    &config.step_type,
                    &config.model,
                    &config.prompt,
                    config.token_budget as i64,
                    config.proof_mode.as_str(),
                    config.epsilon,
                    &config.config_json,
                ],
            ).map_err(|err| Error::Api(format!(
                "failed to insert run_step {}: {}", config.id, err
            )))?;
        }

        // Process checkpoints for each execution
        let mut total_usage = 0u64;
        for execution in &mut run.executions {
            for checkpoint in &mut execution.checkpoints {
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
        }

        // Budget enforcement on the last checkpoint of the last execution
        if let Err(incident) = governance::enforce_budget(policy.budget_tokens, total_usage) {
            if let Some(last_execution) = run.executions.last_mut() {
                if let Some(last) = last_execution.checkpoints.last_mut() {
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
        }

        if total_usage > run.run.token_budget.max(0) as u64 {
            if let Some(last_execution) = run.executions.last_mut() {
                if let Some(last) = last_execution.checkpoints.last_mut() {
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
        }

        // Debug: Count total checkpoints across all executions
        let total_checkpoints: usize = run.executions.iter().map(|e| e.checkpoints.len()).sum();
        eprintln!("DEBUG: Inserted {} steps for run {}: {:?}", inserted_step_ids.len(), run.run.id, inserted_step_ids);
        eprintln!("DEBUG: Processing {} executions with {} total checkpoints for run {}",
            run.executions.len(), total_checkpoints, run.run.id);

        // Fix orphaned checkpoint_config_id references BEFORE inserting
        let mut fixed_count = 0;
        for execution in &mut run.executions {
            for checkpoint in &mut execution.checkpoints {
            if let Some(ref config_id) = checkpoint.checkpoint_config_id {
                eprintln!("DEBUG: Checking checkpoint {} with config_id {}", checkpoint.id, config_id);
                if !inserted_step_ids.contains(config_id) {
                    eprintln!(
                        "WARNING: checkpoint {} references non-existent step {}, setting to NULL (available steps: {:?})",
                        checkpoint.id, config_id, inserted_step_ids
                    );
                    checkpoint.checkpoint_config_id = None;
                    fixed_count += 1;
                }
            }
            }
        }
        eprintln!("DEBUG: Fixed {} orphaned checkpoint references", fixed_count);

        // Now insert the checkpoints from all executions
        for execution in &run.executions {
            for checkpoint in &execution.checkpoints {

            tx.execute(
                "INSERT INTO checkpoints (id, run_id, run_execution_id, checkpoint_config_id, parent_checkpoint_id, turn_index, kind, incident_json, timestamp,
                                          inputs_sha256, outputs_sha256, prev_chain, curr_chain, signature, usage_tokens, prompt_tokens,
                                          completion_tokens, semantic_digest)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                params![
                    &checkpoint.id,
                    &checkpoint.run_id,
                    checkpoint.run_execution_id.as_ref().expect("run_execution_id should be set"),
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
            ).map_err(|err| Error::Api(format!(
                "failed to insert checkpoint {}: config_id={:?}, parent_id={:?}, error={}",
                checkpoint.id, checkpoint.checkpoint_config_id, checkpoint.parent_checkpoint_id, err
            )))?;

            if let Some(ref message) = checkpoint.message {
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

            if let Some(ref payload) = checkpoint.payload {
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

            // Extract CAR JSON and attachments (handles both .car.json and .car.zip)
            let car_filename = receipt.car_path.as_deref().unwrap_or("unknown");
            let (car, attachments) = extract_car_data(&car_bytes, car_filename)?;

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

            // Store attachments in the global attachment store
            let attachment_store = crate::attachments::get_global_attachment_store();
            for (hash, content_bytes) in attachments {
                let content = String::from_utf8(content_bytes)
                    .map_err(|err| Error::Api(format!("attachment {hash} is not valid UTF-8: {err}")))?;
                attachment_store.store_with_hash(&hash, &content)
                    .map_err(|err| Error::Api(format!("failed to store attachment {hash}: {err}")))?;
            }

            // Save the CAR file (preserve original format or convert to zip if it was json)
            let dest_path = if car_filename.ends_with(".car.zip") {
                dest_dir.join(format!("{}.car.zip", receipt.id.replace(':', "_")))
            } else {
                dest_dir.join(format!("{}.car.json", receipt.id))
            };
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
    _pool: &DbPool,
    car_path: &Path,
    base_dir: &Path,
) -> Result<CarImportResult, Error> {
    // Read the CAR file (could be .car.json or .car.zip)
    let car_bytes = fs::read(car_path)
        .map_err(|err| Error::Api(format!("failed to read CAR {}: {err}", car_path.display())))?;

    let car_filename = car_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Extract CAR JSON and attachments
    let (car, attachments) = extract_car_data(&car_bytes, car_filename)?;

    let verifying_key = decode_verifying_key(&car.signer_public_key)?;

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

    if let Some(process) = car.proof.process.as_ref() {
        for checkpoint in &process.sequential_checkpoints {
            let Some(encoded) = checkpoint.signature.strip_prefix("ed25519:") else {
                continue;
            };
            if !signature_valid(&verifying_key, &checkpoint.curr_chain, encoded)? {
                return Err(Error::Api(format!(
                    "checkpoint {} failed signature verification",
                    checkpoint.id
                )));
            }
        }
    }

    // Store attachments in the global attachment store
    let attachment_store = crate::attachments::get_global_attachment_store();
    for (hash, content_bytes) in attachments {
        let content = String::from_utf8(content_bytes)
            .map_err(|err| Error::Api(format!("attachment {hash} is not valid UTF-8: {err}")))?;
        attachment_store.store_with_hash(&hash, &content)
            .map_err(|err| Error::Api(format!("failed to store attachment {hash}: {err}")))?;
    }

    let cars_dir = base_dir.join("cars");
    fs::create_dir_all(&cars_dir).map_err(|err| {
        Error::Api(format!(
            "failed to create CAR storage dir {}: {err}",
            cars_dir.display()
        ))
    })?;

    let sanitized_id = sanitize_for_file(&car.id);
    let dest_path = if car_filename.ends_with(".car.zip") {
        cars_dir.join(format!("{}.car.zip", sanitized_id))
    } else {
        cars_dir.join(format!("{}.car.json", sanitized_id))
    };
    fs::write(&dest_path, &car_bytes)
        .map_err(|err| Error::Api(format!("failed to copy CAR to workspace: {err}")))?;

    let replay_report = replay::replay_car(&car)
        .map_err(|err| Error::Api(format!("failed to replay CAR {}: {err}", car.id)))?;

    let checkpoints = if let Some(process) = car.proof.process.clone() {
        process
            .sequential_checkpoints
            .into_iter()
            .map(|checkpoint| ImportedCarCheckpointSnapshot {
                id: checkpoint.id,
                parent_checkpoint_id: checkpoint.parent_checkpoint_id,
                turn_index: checkpoint.turn_index,
                prev_chain: Some(checkpoint.prev_chain),
                curr_chain: Some(checkpoint.curr_chain),
                signature: Some(checkpoint.signature),
            })
            .collect()
    } else {
        car.checkpoints
            .iter()
            .cloned()
            .map(|id| ImportedCarCheckpointSnapshot {
                id,
                parent_checkpoint_id: None,
                turn_index: None,
                prev_chain: None,
                curr_chain: None,
                signature: None,
            })
            .collect()
    };

    let budgets = ImportedCarBudgets {
        usd: car.budgets.usd,
        tokens: car.budgets.tokens,
        nature_cost: car.budgets.nature_cost,
    };

    let snapshot = ImportedCarSnapshot {
        car_id: car.id.clone(),
        run_id: car.run_id.clone(),
        created_at: car.created_at.to_rfc3339(),
        run: car.run.clone(),
        proof: car.proof.clone(),
        policy_ref: car.policy_ref.clone(),
        budgets,
        provenance: car.provenance.clone(),
        checkpoints,
        sgrade: car.sgrade.clone(),
        signer_public_key: car.signer_public_key.clone(),
    };

    Ok(CarImportResult {
        replay_report,
        snapshot,
    })
}
