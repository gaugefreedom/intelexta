use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use clap::Parser;
use colored::*;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use intelexta::car::{Car, ProcessCheckpointProof};

/// Standalone verification utility for Intelexta CAR (Content-Addressed Receipt) files.
///
/// Verifies cryptographic integrity, hash chains, and digital signatures without requiring
/// the full Intelexta application or database.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the CAR file (.car.json or .car.zip)
    car_file: PathBuf,

    /// Output format (human or json)
    #[arg(long, default_value = "human")]
    format: OutputFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, serde::Serialize)]
struct VerificationReport {
    car_id: String,
    file_integrity: bool,
    hash_chain_valid: bool,
    signatures_valid: bool,
    content_integrity_valid: bool,
    checkpoints_verified: usize,
    checkpoints_total: usize,
    provenance_claims_verified: usize,
    provenance_claims_total: usize,
    overall_result: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load and parse the CAR file
    let (car, raw_json, car_path) = load_car_file(&cli.car_file)?;

    // Run verification (pass the path for attachment verification and raw JSON for signature verification)
    let report = verify_car(&car, &raw_json, &car_path)?;

    // Output results
    match cli.format {
        OutputFormat::Human => print_human_report(&report),
        OutputFormat::Json => print_json_report(&report)?,
    }

    // Exit with appropriate code
    if report.overall_result {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

/// Load CAR from either JSON or ZIP file
/// Returns the parsed CAR, the raw JSON string, and the path to use for attachment verification
fn load_car_file(path: &PathBuf) -> Result<(Car, String, PathBuf)> {
    let extension = path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let (car, raw_json) = match extension {
        "zip" => load_car_from_zip(path)?,
        "json" => load_car_from_json(path)?,
        _ => {
            // Try JSON first, then ZIP
            load_car_from_json(path)
                .or_else(|_| load_car_from_zip(path))
                .with_context(|| format!("Could not parse CAR file: {}", path.display()))?
        }
    };

    Ok((car, raw_json, path.clone()))
}

/// Load CAR from JSON file
fn load_car_from_json(path: &PathBuf) -> Result<(Car, String)> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    let car = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse CAR JSON from: {}", path.display()))?;

    Ok((car, contents))
}

/// Load CAR from ZIP file (extract car.json)
fn load_car_from_zip(path: &PathBuf) -> Result<(Car, String)> {
    let file = fs::File::open(path)
        .with_context(|| format!("Failed to open ZIP file: {}", path.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read ZIP archive: {}", path.display()))?;

    // Find and read car.json
    let mut car_file = archive.by_name("car.json")
        .with_context(|| "CAR ZIP must contain car.json")?;

    let mut contents = String::new();
    car_file.read_to_string(&mut contents)
        .context("Failed to read car.json from ZIP")?;

    let car = serde_json::from_str(&contents)
        .context("Failed to parse car.json from ZIP")?;

    Ok((car, contents))
}

/// Main verification logic
fn verify_car(car: &Car, raw_json: &str, car_path: &PathBuf) -> Result<VerificationReport> {
    let mut report = VerificationReport {
        car_id: car.id.clone(),
        file_integrity: true,
        hash_chain_valid: false,
        signatures_valid: false,
        content_integrity_valid: false,
        checkpoints_verified: 0,
        checkpoints_total: 0,
        provenance_claims_verified: 0,
        provenance_claims_total: 0,
        overall_result: false,
        error: None,
    };

    // Get process proof checkpoints
    let checkpoints = match &car.proof.process {
        Some(process) => &process.sequential_checkpoints,
        None => {
            report.error = Some(format!(
                "CAR has no process proof (match_kind: {}). This CAR was likely exported with an older version of Intelexta. \
                 Please re-export the CAR to include cryptographic signatures for verification.",
                car.proof.match_kind
            ));
            return Ok(report);
        }
    };

    report.checkpoints_total = checkpoints.len();

    if checkpoints.is_empty() {
        report.error = Some("CAR has no checkpoints to verify".to_string());
        return Ok(report);
    }

    // Verify hash chain
    match verify_hash_chain(checkpoints) {
        Ok(verified_count) => {
            report.hash_chain_valid = true;
            report.checkpoints_verified = verified_count;
        }
        Err(e) => {
            report.error = Some(format!("Hash chain verification failed: {}", e));
            return Ok(report);
        }
    }

    // Verify top-level body signature (if present)
    if let Err(e) = verify_top_level_signature(car, raw_json) {
        report.error = Some(format!("Top-level body signature verification failed: {}", e));
        return Ok(report);
    }

    // Verify signatures
    match verify_signatures(&car.signer_public_key, checkpoints) {
        Ok(_) => {
            report.signatures_valid = true;
        }
        Err(e) => {
            report.error = Some(format!("Signature verification failed: {}", e));
            return Ok(report);
        }
    }

    // Verify content integrity (provenance claims + attachments)
    match verify_content_integrity(car, car_path) {
        Ok(verified_count) => {
            report.content_integrity_valid = true;
            report.provenance_claims_verified = verified_count;
            report.provenance_claims_total = car.provenance.len();
        }
        Err(e) => {
            report.error = Some(format!("Content integrity verification failed: {}", e));
            report.provenance_claims_total = car.provenance.len();
            return Ok(report);
        }
    }

    // Overall result
    report.overall_result = report.file_integrity
        && report.hash_chain_valid
        && report.signatures_valid
        && report.content_integrity_valid
        && report.checkpoints_verified == report.checkpoints_total;

    Ok(report)
}

/// Checkpoint body structure used for hash computation (must match orchestrator.rs)
#[derive(serde::Serialize)]
struct CheckpointBody<'a> {
    run_id: &'a str,
    kind: &'a str,
    timestamp: &'a str,
    inputs_sha256: &'a Option<String>,
    outputs_sha256: &'a Option<String>,
    incident: Option<serde_json::Value>,
    usage_tokens: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
}

/// Verify the hash chain across all checkpoints
fn verify_hash_chain(checkpoints: &[ProcessCheckpointProof]) -> Result<usize> {
    let mut verified_count = 0;

    for (i, checkpoint) in checkpoints.iter().enumerate() {
        // Compute expected curr_chain from prev_chain + canonical checkpoint body
        let expected_curr = compute_checkpoint_hash(checkpoint)?;

        if expected_curr != checkpoint.curr_chain {
            return Err(anyhow!(
                "Hash chain broken at checkpoint #{} (id: {})\nExpected: {}\nFound: {}",
                i,
                checkpoint.id,
                expected_curr,
                checkpoint.curr_chain
            ));
        }

        verified_count += 1;
    }

    Ok(verified_count)
}

/// Compute checkpoint hash: SHA256(prev_chain || canonical_json(checkpoint_body))
fn compute_checkpoint_hash(checkpoint: &ProcessCheckpointProof) -> Result<String> {
    // Reconstruct the checkpoint body exactly as it was signed
    let body = CheckpointBody {
        run_id: &checkpoint.run_id,
        kind: &checkpoint.kind,
        timestamp: &checkpoint.timestamp,
        inputs_sha256: &checkpoint.inputs_sha256,
        outputs_sha256: &checkpoint.outputs_sha256,
        incident: None, // Incidents are not included in process checkpoints
        usage_tokens: checkpoint.usage_tokens,
        prompt_tokens: checkpoint.prompt_tokens,
        completion_tokens: checkpoint.completion_tokens,
    };

    // Convert to JSON value and canonicalize
    let body_json = serde_json::to_value(&body)?;
    let canonical = canonical_json(&body_json)?;

    // Compute SHA256(prev_chain || canonical_body)
    let mut hasher = Sha256::new();
    hasher.update(checkpoint.prev_chain.as_bytes());
    hasher.update(&canonical);
    Ok(hex::encode(hasher.finalize()))
}

/// Canonical JSON implementation (must match provenance::canonical_json)
/// Uses JCS (JSON Canonicalization Scheme) for deterministic encoding
fn canonical_json(value: &serde_json::Value) -> Result<Vec<u8>> {
    serde_jcs::to_vec(value).map_err(|e| anyhow!("Failed to canonicalize JSON: {}", e))
}

/// Verify Ed25519 signatures on all checkpoints
fn verify_signatures(
    public_key_b64: &str,
    checkpoints: &[ProcessCheckpointProof],
) -> Result<()> {
    // Parse public key from base64
    let public_key_bytes = STANDARD
        .decode(public_key_b64)
        .context("Invalid public key base64")?;

    let public_key = VerifyingKey::from_bytes(
        &public_key_bytes
            .try_into()
            .map_err(|_| anyhow!("Public key must be 32 bytes"))?,
    )
    .context("Invalid Ed25519 public key")?;

    // Verify each checkpoint signature
    for (i, checkpoint) in checkpoints.iter().enumerate() {
        // Parse signature from base64
        let sig_bytes = STANDARD
            .decode(&checkpoint.signature)
            .with_context(|| format!("Invalid signature base64 at checkpoint #{}", i))?;

        let signature = Signature::from_bytes(
            &sig_bytes
                .try_into()
                .map_err(|_| anyhow!("Signature must be 64 bytes at checkpoint #{}", i))?,
        );

        // The message being signed is the curr_chain hash
        let message = checkpoint.curr_chain.as_bytes();

        // Verify signature
        public_key
            .verify(message, &signature)
            .with_context(|| format!("Signature verification failed at checkpoint #{}", i))?;
    }

    Ok(())
}

/// Verify top-level body signature (if present in new format)
///
/// New CAR format includes dual signatures:
/// - ed25519-body:<sig> - covers entire CAR body (prevents tampering with created_at, budgets, etc.)
/// - ed25519-checkpoint:<sig> - covers checkpoint chain hash (verified by verify_signatures)
fn verify_top_level_signature(car: &Car, raw_json: &str) -> Result<()> {
    if car.signatures.is_empty() {
        return Err(anyhow!("No signatures found in CAR"));
    }

    let first_sig = &car.signatures[0];

    // If it's the new format with top-level body signature, verify it
    if first_sig.starts_with("ed25519-body:") {
        if car.signer_public_key.is_empty() {
            return Err(anyhow!("Top-level signature present but signer_public_key is empty"));
        }

        let sig_b64 = first_sig.strip_prefix("ed25519-body:").unwrap();

        // Parse raw JSON as Value and remove signatures field
        let mut car_json: serde_json::Value = serde_json::from_str(raw_json)
            .context("Failed to parse raw JSON")?;

        // Remove signatures field
        if let Some(obj) = car_json.as_object_mut() {
            obj.remove("signatures");
        }

        // Canonicalize the body (without re-serializing through Rust structs)
        let canonical = canonical_json(&car_json)?;

        // Parse public key
        let public_key_bytes = STANDARD
            .decode(&car.signer_public_key)
            .context("Invalid signer public key base64")?;

        let public_key = VerifyingKey::from_bytes(
            &public_key_bytes
                .try_into()
                .map_err(|_| anyhow!("Public key must be 32 bytes"))?,
        )
        .context("Invalid Ed25519 public key")?;

        // Parse signature
        let signature_bytes = STANDARD
            .decode(sig_b64)
            .context("Invalid top-level signature base64")?;

        let signature = Signature::from_bytes(
            &signature_bytes
                .try_into()
                .map_err(|_| anyhow!("Signature must be 64 bytes"))?,
        );

        // Verify signature
        public_key
            .verify(&canonical, &signature)
            .context("Top-level body signature verification failed")?;
    }
    // else: legacy format without top-level signature, skip this check

    Ok(())
}

/// Verify content integrity by checking provenance claims and attachment files
fn verify_content_integrity(car: &Car, car_path: &PathBuf) -> Result<usize> {
    let mut verified_count = 0;

    // Step 1: Verify provenance claims (config hash)
    for (i, claim) in car.provenance.iter().enumerate() {
        // Extract the hash from the claim (format: "sha256:...")
        let expected_hash = claim
            .sha256
            .strip_prefix("sha256:")
            .ok_or_else(|| anyhow!("Invalid provenance claim #{}: hash must start with 'sha256:'", i))?;

        match claim.claim_type.as_str() {
            "config" => {
                // Verify run specification hash
                let spec_json = serde_json::to_value(&car.run.steps)?;
                let canonical = canonical_json(&spec_json)?;
                let computed_hash = hex::encode(Sha256::digest(&canonical));

                if computed_hash != expected_hash {
                    return Err(anyhow!(
                        "Config hash mismatch at provenance claim #{}\nExpected: {}\nComputed: {}",
                        i,
                        expected_hash,
                        computed_hash
                    ));
                }
                verified_count += 1;
            }
            "input" | "output" => {
                // For inputs/outputs, verify the hash appears in checkpoints
                // Actual content verification happens in Step 2
                let hash_exists = car
                    .proof
                    .process
                    .as_ref()
                    .map(|p| {
                        p.sequential_checkpoints.iter().any(|ck| {
                            ck.inputs_sha256.as_deref() == Some(expected_hash)
                                || ck.outputs_sha256.as_deref() == Some(expected_hash)
                        })
                    })
                    .unwrap_or(false);

                if !hash_exists {
                    return Err(anyhow!(
                        "{} hash not found in checkpoints at provenance claim #{}",
                        claim.claim_type,
                        i
                    ));
                }
                verified_count += 1;
            }
            _ => {
                // Unknown claim type - skip for forward compatibility
                continue;
            }
        }
    }

    // Step 2: Verify all attachment files in the CAR
    // Attachments are self-verifying: filename = hash of content
    // We verify that every attachment file's content matches its filename hash
    verify_all_attachments(car_path)?;

    Ok(verified_count)
}

/// Verify all attachment files in the CAR
/// Attachments are self-verifying: the filename is the hash of the content
fn verify_all_attachments(car_path: &PathBuf) -> Result<()> {
    // Determine if we're working with a ZIP or JSON file
    let extension = car_path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if extension != "zip" {
        // For standalone JSON, skip attachment verification
        // (attachments would need to be in a sibling directory)
        return Ok(());
    }

    let file = fs::File::open(car_path)
        .with_context(|| format!("Failed to open ZIP file: {}", car_path.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read ZIP archive: {}", car_path.display()))?;

    // Find all files in the attachments/ directory
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        // Only process files in attachments/ directory
        if !name.starts_with("attachments/") || !name.ends_with(".txt") {
            continue;
        }

        // Extract the expected hash from the filename
        // Format: attachments/{hash}.txt
        let expected_hash = name
            .strip_prefix("attachments/")
            .and_then(|s| s.strip_suffix(".txt"))
            .ok_or_else(|| anyhow!("Invalid attachment filename format: {}", name))?;

        // Read the file content
        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .with_context(|| format!("Failed to read attachment file: {}", name))?;

        // Compute SHA256 hash of the content
        let computed_hash = hex::encode(Sha256::digest(&content));

        // Verify the hash matches the filename
        if computed_hash != expected_hash {
            return Err(anyhow!(
                "Attachment content mismatch\nFile: {}\nExpected hash (from filename): {}\nComputed hash (from content): {}\n\nThis indicates the attachment file has been tampered with!",
                name,
                expected_hash,
                computed_hash
            ));
        }
    }

    Ok(())
}

/// Print human-readable report
fn print_human_report(report: &VerificationReport) {
    println!("\n{}", "Intelexta CAR Verification".bold().cyan());
    println!("{}", "=".repeat(50));
    println!();

    println!("CAR ID: {}", report.car_id.bright_black());
    println!();

    // File integrity
    print_check("File Integrity", report.file_integrity);

    // Hash chain
    print_check(
        &format!(
            "Hash Chain ({}/{} checkpoints)",
            report.checkpoints_verified, report.checkpoints_total
        ),
        report.hash_chain_valid,
    );

    // Signatures
    print_check(
        &format!("Signatures ({} checkpoints)", report.checkpoints_total),
        report.signatures_valid,
    );

    // Content integrity
    print_check(
        &format!(
            "Content Integrity ({}/{} provenance claims)",
            report.provenance_claims_verified, report.provenance_claims_total
        ),
        report.content_integrity_valid,
    );

    println!();
    println!("{}", "-".repeat(50));

    // Overall result
    if report.overall_result {
        println!(
            "{} {}",
            "✓ VERIFIED:".green().bold(),
            "This CAR is cryptographically valid and has not been tampered with.".green()
        );
    } else {
        println!("{} {}", "✗ FAILED:".red().bold(), "Verification failed.".red());
        if let Some(error) = &report.error {
            println!("{} {}", "Error:".red(), error);
        }
    }

    println!();
}

/// Print JSON report
fn print_json_report(report: &VerificationReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{}", json);
    Ok(())
}

/// Helper to print a check result
fn print_check(label: &str, passed: bool) {
    if passed {
        println!("  {} {}", "✓".green(), label);
    } else {
        println!("  {} {}", "✗".red(), label);
    }
}
