// In src-tauri/src/provenance.rs

use ed25519_dalek::SigningKey; // We only need SigningKey here now
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use base64::{Engine as _, engine::general_purpose::STANDARD};

pub struct Keypair {
    pub public_key: String, // base64
    pub secret_key: String, // base64
}

// Generates a new Ed25519 keypair using the modern v2 API correctly
pub fn generate_keypair() -> Keypair {
    let mut csprng = OsRng;
    // Generate the SigningKey directly. This is the correct method.
    let signing_key = SigningKey::generate(&mut csprng);
    let public_key = signing_key.verifying_key();

    Keypair {
        public_key: STANDARD.encode(public_key.as_bytes()),
        // The secret part is the bytes of the signing key itself.
        secret_key: STANDARD.encode(signing_key.to_bytes()),
    }
}

// Implements Normative Rule NR-01 for canonical JSON.
pub fn canonical_json<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_jcs::to_vec(value).expect("Failed to create canonical JSON")
}

// A standard SHA256 hex digest helper
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}