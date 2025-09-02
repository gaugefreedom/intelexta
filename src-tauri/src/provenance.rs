// In src-tauri/src/provenance.rs

// Import SigningKey and the RngCore trait.
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore; // Required for the `fill_bytes` method
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct KeyPair {
    pub public_key: String, // Stored as base64
    pub secret_key: String,
}

pub fn generate_keypair() -> KeyPair {
    let mut csprng = OsRng;

    // 1. Generate 32 random bytes for our secret key.
    let mut secret_bytes = [0u8; 32];
    csprng.fill_bytes(&mut secret_bytes);

    // 2. Create a `SigningKey` from the random bytes.
    //    This represents the secret half of the keypair.
    let signing_key = SigningKey::from_bytes(&secret_bytes);

    // 3. Derive the `VerifyingKey` (the public half) from the `SigningKey`.
    let verifying_key: VerifyingKey = (&signing_key).into();

    // 4. Encode the secret part to base64.
    let secret_key_b64 = base64::Engine::encode(
        &base64::prelude::BASE64_STANDARD,
        signing_key.to_bytes(),
    );

    // 5. Encode the public part to base64.
    let public_key_b64 = base64::Engine::encode(
        &base64::prelude::BASE64_STANDARD,
        verifying_key.to_bytes(),
    );

    KeyPair {
        public_key: public_key_b64,
        secret_key: secret_key_b64,
    }
}

