use std::io::{Cursor, Read};

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

const ZIP_MAGIC: &[u8; 4] = b"PK\x03\x04";

mod model;
use model::{Car, ProcessCheckpointProof};

#[wasm_bindgen]
pub fn verify_car_bytes(bytes: &[u8]) -> Result<JsValue, JsError> {
    let decoded = decode_car(bytes).map_err(to_js_error)?;
    let report = verify_car(decoded).map_err(to_js_error)?;
    serde_wasm_bindgen::to_value(&report).map_err(|err| JsError::new(&err.to_string()))
}

#[wasm_bindgen]
pub fn verify_car_json(json: &str) -> Result<JsValue, JsError> {
    let decoded = decode_car(json.as_bytes()).map_err(to_js_error)?;
    let report = verify_car(decoded).map_err(to_js_error)?;
    serde_wasm_bindgen::to_value(&report).map_err(|err| JsError::new(&err.to_string()))
}

fn to_js_error(err: anyhow::Error) -> JsError {
    JsError::new(&err.to_string())
}

fn decode_car(bytes: &[u8]) -> Result<DecodedCar> {
    if bytes.len() >= ZIP_MAGIC.len() && &bytes[..ZIP_MAGIC.len()] == ZIP_MAGIC {
        load_car_from_zip(bytes)
    } else {
        load_car_from_json(bytes)
    }
}

fn load_car_from_json(bytes: &[u8]) -> Result<DecodedCar> {
    let car: Car = serde_json::from_slice(bytes).context("Failed to parse CAR JSON")?;
    let raw_json = String::from_utf8(bytes.to_vec()).context("Invalid UTF-8 in CAR JSON")?;
    Ok(DecodedCar {
        car,
        raw_json,
        attachments: Vec::new(),
    })
}

fn load_car_from_zip(bytes: &[u8]) -> Result<DecodedCar> {
    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).context("Failed to read CAR ZIP archive")?;

    let mut car_json = None;
    let mut attachments = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        if name == "car.json" {
            car_json = Some(buffer);
        } else if name.starts_with("attachments/") && !name.ends_with('/') {
            attachments.push(Attachment { name, data: buffer });
        }
    }

    let car_data = car_json.ok_or_else(|| anyhow!("CAR ZIP is missing car.json"))?;
    let car: Car =
        serde_json::from_slice(&car_data).context("Failed to parse car.json from ZIP")?;
    let raw_json = String::from_utf8(car_data).context("Invalid UTF-8 in car.json")?;

    Ok(DecodedCar { car, raw_json, attachments })
}

fn verify_car(decoded: DecodedCar) -> Result<VerificationReport> {
    let DecodedCar { car, raw_json, attachments } = decoded;

    let mut summary = SummaryMetrics {
        checkpoints_verified: 0,
        checkpoints_total: 0,
        provenance_verified: 0,
        provenance_total: car.provenance.len(),
        attachments_verified: 0,
        attachments_total: attachments
            .iter()
            .filter(|attachment| {
                attachment.name.starts_with("attachments/") && !attachment.name.ends_with('/')
            })
            .count(),
        hash_chain_valid: false,
        signatures_valid: false,
        content_integrity_valid: false,
    };

    let mut steps = Vec::new();
    let mut overall_error = None;

    let process = match &car.proof.process {
        Some(process) if !process.sequential_checkpoints.is_empty() => process,
        Some(_) => {
            let message = "CAR has no checkpoints to verify".to_string();
            steps.push(WorkflowStep::failure(
                "hash_chain",
                "Hash chain integrity",
                &message,
            ));
            steps.extend(skipped_steps(
                ["signatures", "provenance", "attachments"],
                [
                    "Signature validation",
                    "Provenance verification",
                    "Attachment integrity",
                ],
                &message,
            ));
            return Ok(build_report(car, summary, steps, Some(message)));
        }
        None => {
            let message = format!(
                "CAR has no process proof (match_kind: {}). This CAR was likely exported with an older version of Intelexta. Please re-export the CAR to include cryptographic signatures for verification.",
                car.proof.match_kind
            );
            steps.push(WorkflowStep::failure(
                "hash_chain",
                "Hash chain integrity",
                &message,
            ));
            steps.extend(skipped_steps(
                ["signatures", "provenance", "attachments"],
                [
                    "Signature validation",
                    "Provenance verification",
                    "Attachment integrity",
                ],
                &message,
            ));
            return Ok(build_report(car, summary, steps, Some(message)));
        }
    };

    summary.checkpoints_total = process.sequential_checkpoints.len();

    match verify_hash_chain(&process.sequential_checkpoints) {
        Ok(count) => {
            summary.hash_chain_valid = true;
            summary.checkpoints_verified = count;
            steps.push(WorkflowStep::success(
                "hash_chain",
                "Hash chain integrity",
                vec![StepDetail::new(
                    "Sequential checkpoints",
                    format!("{count}/{} verified", summary.checkpoints_total),
                )],
            ));
        }
        Err(err) => {
            let message = format!("Hash chain verification failed: {err}");
            steps.push(WorkflowStep::failure(
                "hash_chain",
                "Hash chain integrity",
                &message,
            ));
            steps.extend(skipped_steps(
                ["signatures", "provenance", "attachments"],
                [
                    "Signature validation",
                    "Provenance verification",
                    "Attachment integrity",
                ],
                &message,
            ));
            overall_error = Some(message);
            return Ok(build_report(car, summary, steps, overall_error));
        }
    }

    // Verify top-level body signature (if present)
    match verify_top_level_signature(&car, &raw_json) {
        Ok(_) => {
            // Top-level signature verified or not present (legacy format)
        }
        Err(err) => {
            let message = format!("Top-level body signature verification failed: {err}");
            steps.push(WorkflowStep::failure(
                "signatures",
                "Signature validation",
                &message,
            ));
            steps.extend(skipped_steps(
                ["provenance", "attachments"],
                ["Provenance verification", "Attachment integrity"],
                &message,
            ));
            overall_error = Some(message);
            return Ok(build_report(car, summary, steps, overall_error));
        }
    }

    match verify_signatures(&car.signer_public_key, &process.sequential_checkpoints) {
        Ok(_) => {
            summary.signatures_valid = true;
            steps.push(WorkflowStep::success(
                "signatures",
                "Signature validation",
                vec![StepDetail::new(
                    "Checkpoint signatures",
                    format!("{} verified", summary.checkpoints_total),
                )],
            ));
        }
        Err(err) => {
            let message = format!("Signature verification failed: {err}");
            steps.push(WorkflowStep::failure(
                "signatures",
                "Signature validation",
                &message,
            ));
            steps.extend(skipped_steps(
                ["provenance", "attachments"],
                ["Provenance verification", "Attachment integrity"],
                &message,
            ));
            overall_error = Some(message);
            return Ok(build_report(car, summary, steps, overall_error));
        }
    }

    match verify_provenance(&car, &process.sequential_checkpoints) {
        Ok(verified) => {
            summary.provenance_verified = verified;
            steps.push(WorkflowStep::success(
                "provenance",
                "Provenance verification",
                vec![StepDetail::new(
                    "Provenance claims",
                    format!("{verified}/{} verified", summary.provenance_total),
                )],
            ));
        }
        Err(err) => {
            let message = format!("Content integrity verification failed: {err}");
            steps.push(WorkflowStep::failure(
                "provenance",
                "Provenance verification",
                &message,
            ));
            steps.push(WorkflowStep::skipped(
                "attachments",
                "Attachment integrity",
                &message,
            ));
            overall_error = Some(message);
            return Ok(build_report(car, summary, steps, overall_error));
        }
    }

    match verify_all_attachments(&attachments) {
        Ok(verified) => {
            summary.attachments_verified = verified;
            steps.push(WorkflowStep::success(
                "attachments",
                "Attachment integrity",
                vec![StepDetail::new(
                    "Attachment files",
                    format!("{verified}/{} verified", summary.attachments_total),
                )],
            ));
        }
        Err(err) => {
            let message = format!("Attachment verification failed: {err}");
            steps.push(WorkflowStep::failure(
                "attachments",
                "Attachment integrity",
                &message,
            ));
            overall_error = Some(message);
            return Ok(build_report(car, summary, steps, overall_error));
        }
    }

    summary.content_integrity_valid = true;

    Ok(build_report(car, summary, steps, overall_error))
}

fn build_report(
    car: Car,
    mut summary: SummaryMetrics,
    steps: Vec<WorkflowStep>,
    error: Option<String>,
) -> VerificationReport {
    let status = if summary.hash_chain_valid
        && summary.signatures_valid
        && summary.content_integrity_valid
    {
        VerificationStatus::Verified
    } else {
        VerificationStatus::Failed
    };

    if !summary.hash_chain_valid {
        summary.content_integrity_valid = false;
    }

    let signer = if car.signer_public_key.is_empty() {
        None
    } else {
        Some(SignerSummary {
            public_key: car.signer_public_key.clone(),
        })
    };

    VerificationReport {
        status,
        car_id: car.id.clone(),
        run_id: car.run_id.clone(),
        created_at: car.created_at.to_rfc3339(),
        signer,
        model: ModelSummary {
            name: car.run.model.clone(),
            version: car.run.version.clone(),
            kind: car.run.kind.clone(),
        },
        steps,
        summary,
        error,
    }
}

fn verify_hash_chain(checkpoints: &[ProcessCheckpointProof]) -> Result<usize> {
    let mut verified = 0;

    for (index, checkpoint) in checkpoints.iter().enumerate() {
        let expected = compute_checkpoint_hash(checkpoint)?;
        if expected != checkpoint.curr_chain {
            return Err(anyhow!(
                "Hash chain broken at checkpoint #{index} (id: {})\nExpected: {expected}\nFound: {}",
                checkpoint.id,
                checkpoint.curr_chain
            ));
        }
        verified += 1;
    }

    Ok(verified)
}

fn compute_checkpoint_hash(checkpoint: &ProcessCheckpointProof) -> Result<String> {
    #[derive(Serialize)]
    struct CheckpointBody<'a> {
        run_id: &'a str,
        kind: &'a str,
        timestamp: &'a str,
        inputs_sha256: &'a Option<String>,
        outputs_sha256: &'a Option<String>,
        incident: Option<Value>,
        usage_tokens: u64,
        prompt_tokens: u64,
        completion_tokens: u64,
    }

    let body = CheckpointBody {
        run_id: &checkpoint.run_id,
        kind: &checkpoint.kind,
        timestamp: &checkpoint.timestamp,
        inputs_sha256: &checkpoint.inputs_sha256,
        outputs_sha256: &checkpoint.outputs_sha256,
        incident: None,
        usage_tokens: checkpoint.usage_tokens,
        prompt_tokens: checkpoint.prompt_tokens,
        completion_tokens: checkpoint.completion_tokens,
    };

    let body_json = serde_json::to_value(&body)?;
    let canonical = canonical_json(&body_json)?;

    let mut hasher = Sha256::new();
    hasher.update(checkpoint.prev_chain.as_bytes());
    hasher.update(&canonical);
    Ok(hex::encode(hasher.finalize()))
}

fn canonical_json(value: &Value) -> Result<Vec<u8>> {
    serde_jcs::to_vec(value).map_err(|err| anyhow!("Failed to canonicalize JSON: {err}"))
}

fn verify_top_level_signature(car: &Car, raw_json: &str) -> Result<()> {
    // Check if we have the new signature format (ed25519-body:...)
    if car.signatures.is_empty() {
        return Err(anyhow!("No signatures found in CAR"));
    }

    let first_sig = &car.signatures[0];

    // If it's the new format, verify top-level body signature
    if first_sig.starts_with("ed25519-body:") {
        if car.signer_public_key.is_empty() {
            return Err(anyhow!("Top-level signature present but signer_public_key is empty"));
        }

        // Extract signature
        let sig_b64 = first_sig.strip_prefix("ed25519-body:").unwrap();

        // Parse raw JSON as Value and remove signatures field
        let mut car_json: Value = serde_json::from_str(raw_json)
            .context("Failed to parse raw JSON")?;

        // Remove signatures field
        if let Some(obj) = car_json.as_object_mut() {
            obj.remove("signatures");
        }

        // Canonicalize the body (without re-serializing through Rust structs)
        let canonical = canonical_json(&car_json)?;

        // Verify signature
        let public_key_bytes = STANDARD
            .decode(&car.signer_public_key)
            .context("Invalid signer public key base64")?;

        let verifying_key = VerifyingKey::from_bytes(
            &public_key_bytes
                .try_into()
                .map_err(|_| anyhow!("Public key must be 32 bytes"))?,
        )
        .context("Invalid Ed25519 public key")?;

        let signature_bytes = STANDARD
            .decode(sig_b64)
            .context("Invalid top-level signature base64")?;

        let signature = Signature::from_bytes(
            &signature_bytes
                .try_into()
                .map_err(|_| anyhow!("Signature must be 64 bytes"))?,
        );

        verifying_key
            .verify(&canonical, &signature)
            .context("Top-level body signature verification failed")?;
    }
    // else: legacy format (no ed25519-body prefix), skip top-level verification

    Ok(())
}

fn verify_signatures(public_key_b64: &str, checkpoints: &[ProcessCheckpointProof]) -> Result<()> {
    let public_key_bytes = STANDARD
        .decode(public_key_b64)
        .context("Invalid signer public key base64")?;

    let verifying_key = VerifyingKey::from_bytes(
        &public_key_bytes
            .try_into()
            .map_err(|_| anyhow!("Public key must be 32 bytes"))?,
    )
    .context("Invalid Ed25519 public key")?;

    for (index, checkpoint) in checkpoints.iter().enumerate() {
        let signature_bytes = STANDARD
            .decode(&checkpoint.signature)
            .with_context(|| format!("Invalid signature base64 at checkpoint #{index}"))?;

        let signature = Signature::from_bytes(
            &signature_bytes
                .try_into()
                .map_err(|_| anyhow!("Signature must be 64 bytes at checkpoint #{index}"))?,
        );

        verifying_key
            .verify(checkpoint.curr_chain.as_bytes(), &signature)
            .with_context(|| format!("Signature verification failed at checkpoint #{index}"))?;
    }

    Ok(())
}

fn verify_provenance(car: &Car, checkpoints: &[ProcessCheckpointProof]) -> Result<usize> {
    let mut verified = 0;

    for (index, claim) in car.provenance.iter().enumerate() {
        let expected_hash = claim.sha256.strip_prefix("sha256:").ok_or_else(|| {
            anyhow!(
                "Invalid provenance claim #{}: hash must start with 'sha256:'",
                index
            )
        })?;

        match claim.claim_type.as_str() {
            "config" => {
                let spec_json = serde_json::to_value(&car.run.steps)?;
                let canonical = canonical_json(&spec_json)?;
                let computed = hex::encode(Sha256::digest(&canonical));

                if computed != expected_hash {
                    return Err(anyhow!(
                        "Config hash mismatch at provenance claim #{}\nExpected: {}\nComputed: {}",
                        index,
                        expected_hash,
                        computed
                    ));
                }
                verified += 1;
            }
            "input" | "output" => {
                let exists = checkpoints.iter().any(|checkpoint| {
                    checkpoint
                        .inputs_sha256
                        .as_deref()
                        .map(|hash| hash == expected_hash)
                        .unwrap_or(false)
                        || checkpoint
                            .outputs_sha256
                            .as_deref()
                            .map(|hash| hash == expected_hash)
                            .unwrap_or(false)
                });

                if !exists {
                    return Err(anyhow!(
                        "{} hash not found in checkpoints at provenance claim #{}",
                        claim.claim_type,
                        index
                    ));
                }
                verified += 1;
            }
            _ => {}
        }
    }

    Ok(verified)
}

fn verify_all_attachments(attachments: &[Attachment]) -> Result<usize> {
    let mut verified = 0;

    for attachment in attachments
        .iter()
        .filter(|att| att.name.starts_with("attachments/") && !att.name.ends_with('/'))
    {
        let expected = attachment
            .name
            .strip_prefix("attachments/")
            .ok_or_else(|| anyhow!("Invalid attachment path: {}", attachment.name))?;

        let (hash, _extension) = expected
            .split_once('.')
            .ok_or_else(|| anyhow!("Invalid attachment filename format: {}", attachment.name))?;

        let computed = hex::encode(Sha256::digest(&attachment.data));

        if computed != hash {
            return Err(anyhow!(
                "Attachment content mismatch\nFile: {}\nExpected hash: {}\nComputed hash: {}",
                attachment.name,
                hash,
                computed
            ));
        }

        verified += 1;
    }

    Ok(verified)
}

fn skipped_steps<const N: usize>(
    keys: [&'static str; N],
    labels: [&'static str; N],
    reason: &str,
) -> Vec<WorkflowStep> {
    keys.into_iter()
        .zip(labels)
        .map(|(key, label)| WorkflowStep::skipped(key, label, reason))
        .collect()
}

struct DecodedCar {
    car: Car,
    raw_json: String,
    attachments: Vec<Attachment>,
}

struct Attachment {
    name: String,
    data: Vec<u8>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum VerificationStatus {
    Verified,
    Failed,
}

#[derive(Serialize)]
pub struct VerificationReport {
    pub status: VerificationStatus,
    pub car_id: String,
    pub run_id: String,
    pub created_at: String,
    pub signer: Option<SignerSummary>,
    pub model: ModelSummary,
    pub steps: Vec<WorkflowStep>,
    pub summary: SummaryMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SignerSummary {
    pub public_key: String,
}

#[derive(Serialize)]
pub struct ModelSummary {
    pub name: String,
    pub version: String,
    pub kind: String,
}

#[derive(Serialize, Default)]
pub struct SummaryMetrics {
    pub checkpoints_verified: usize,
    pub checkpoints_total: usize,
    pub provenance_verified: usize,
    pub provenance_total: usize,
    pub attachments_verified: usize,
    pub attachments_total: usize,
    pub hash_chain_valid: bool,
    pub signatures_valid: bool,
    pub content_integrity_valid: bool,
}

#[derive(Serialize)]
pub struct WorkflowStep {
    pub key: &'static str,
    pub label: &'static str,
    pub status: StepStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<StepDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WorkflowStep {
    fn success(key: &'static str, label: &'static str, details: Vec<StepDetail>) -> Self {
        Self {
            key,
            label,
            status: StepStatus::Passed,
            details,
            error: None,
        }
    }

    fn failure(key: &'static str, label: &'static str, message: &str) -> Self {
        Self {
            key,
            label,
            status: StepStatus::Failed,
            details: Vec::new(),
            error: Some(message.to_string()),
        }
    }

    fn skipped(key: &'static str, label: &'static str, reason: &str) -> Self {
        Self {
            key,
            label,
            status: StepStatus::Skipped,
            details: Vec::new(),
            error: Some(reason.to_string()),
        }
    }
}

#[derive(Serialize)]
pub struct StepDetail {
    pub label: String,
    pub value: String,
}

impl StepDetail {
    fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Passed,
    Failed,
    Skipped,
}

#[cfg(test)]
mod tests {
    use super::*;

    use base64::{engine::general_purpose::STANDARD, Engine as _};

    const SAMPLE_JSON: &[u8] = include_bytes!("../tests/fixtures/sample.car.json");
    const SAMPLE_ZIP_BASE64: &str = concat!(
        "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",
    );

    fn sample_zip_bytes() -> Vec<u8> {
        STANDARD
            .decode(SAMPLE_ZIP_BASE64.as_bytes())
            .expect("valid base64 ZIP fixture")
    }

    #[test]
    fn verify_sample_json() {
        let decoded = decode_car(SAMPLE_JSON).expect("decode json");
        let report = verify_car(decoded).expect("verify json");
        assert!(matches!(report.status, VerificationStatus::Verified));
        assert!(report.summary.hash_chain_valid);
        assert!(report.summary.signatures_valid);
        assert!(report.summary.content_integrity_valid);
        assert_eq!(
            report.summary.checkpoints_verified,
            report.summary.checkpoints_total
        );
    }

    #[test]
    fn verify_sample_zip() {
        let zip_bytes = sample_zip_bytes();
        let decoded = decode_car(&zip_bytes).expect("decode zip");
        let report = verify_car(decoded).expect("verify zip");
        assert!(matches!(report.status, VerificationStatus::Verified));
        assert_eq!(
            report.summary.attachments_verified,
            report.summary.attachments_total
        );
    }
}
