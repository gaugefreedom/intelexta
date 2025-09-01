// In src-tauri/src/provenance.rs
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct KeyPair {
    pub public_key: String, // Stored as base64
    // The secret key is returned ONCE on creation and should be handled by the caller.
    // We do not store it in the database in plaintext.
    pub secret_key: String,
}

pub fn generate_keypair() -> KeyPair {
    let mut csprng = OsRng;
    let signing_key: SigningKey = SigningKey::generate(&mut csprng);
    let secret_key_b64 = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, signing_key.to_bytes());
    let public_key_b64 = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, signing_key.verifying_key().to_bytes());

    KeyPair {
        public_key: public_key_b64,
        secret_key: secret_key_b64,
    }
}