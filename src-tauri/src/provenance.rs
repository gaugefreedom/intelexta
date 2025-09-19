// In src-tauri/src/provenance.rs
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::Serialize;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use anyhow::anyhow; // <-- Add this import

// Use a constant for the service name to prevent typos.
const KEYCHAIN_SERVICE_NAME: &str = "intelexta";

pub struct KeypairOut {
    pub public_key_b64: String,
    pub secret_key_b64: String,
}

pub fn generate_keypair() -> KeypairOut {
    let sk = SigningKey::generate(&mut OsRng);
    let pk = sk.verifying_key();
    KeypairOut {
        public_key_b64: STANDARD.encode(pk.as_bytes()),
        secret_key_b64: STANDARD.encode(sk.to_bytes()),
    }
}

pub fn store_secret_key(project_id: &str, secret_key_b64: &str) -> anyhow::Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE_NAME, project_id)?;
    entry.set_password(secret_key_b64)?;
    Ok(())
}

pub fn load_secret_key(project_id: &str) -> anyhow::Result<SigningKey> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE_NAME, project_id)?;
    
    // --- FIX STARTS HERE ---
    // Handle errors explicitly to resolve the compiler ambiguity and ensure
    // the `NoEntry` error is propagated correctly for the repair logic.
    let b64 = match entry.get_password() {
        Ok(password) => password,
        Err(keyring::Error::NoEntry) => {
            // This specific error is caught in orchestrator.rs to repair the key.
            // We need to wrap it in anyhow::Error but still allow it to be identified.
            return Err(anyhow!(keyring::Error::NoEntry));
        }
        Err(other_err) => {
            // Handle any other keychain errors.
            return Err(anyhow!(other_err));
        }
    };
    // --- FIX ENDS HERE ---

    let bytes = STANDARD.decode(b64)?;
    let sk = SigningKey::from_bytes(&bytes.try_into().map_err(|_| anyhow!("bad sk len"))?);
    Ok(sk)
}

pub fn public_key_from_secret(sk: &SigningKey) -> String {
    let pk: VerifyingKey = sk.verifying_key();
    STANDARD.encode(pk.as_bytes())
}

pub fn sign_bytes(sk: &SigningKey, bytes: &[u8]) -> String {
    let sig: Signature = sk.sign(bytes);
    STANDARD.encode(sig.to_bytes())
}

// === Canonicalization & hashing ===
pub fn canonical_json<T: Serialize>(t: &T) -> Vec<u8> {
    serde_jcs::to_vec(t).expect("canonical json")
}

pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Serialize)]
    struct S { b: u8, a: u8 }

    #[test]
    fn canon_same_struct_same_bytes() {
        let s1 = S { b: 2, a: 1 };
        let s2 = S { a: 1, b: 2 };
        let c1 = canonical_json(&s1);
        let c2 = canonical_json(&s2);
        assert_eq!(c1, c2, "JCS must produce identical bytes");
    }
}