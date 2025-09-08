// In src-tauri/src/provenance.rs
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::Serialize;
// ADD THIS LINE to import the new Base64 Engine
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub struct KeypairOut {
    pub public_key_b64: String,
    pub secret_key_b64: String,
}

pub fn generate_keypair() -> KeypairOut {
    let sk = SigningKey::generate(&mut OsRng);
    let pk = sk.verifying_key();
    KeypairOut {
        // Use the new Engine API
        public_key_b64: STANDARD.encode(pk.as_bytes()),
        secret_key_b64: STANDARD.encode(sk.to_bytes()),
    }
}

pub fn store_secret_key(project_id: &str, secret_key_b64: &str) -> anyhow::Result<()> {
    let entry = keyring::Entry::new("intelexta", project_id)?;
    entry.set_password(secret_key_b64)?;
    Ok(())
}

pub fn load_secret_key(project_id: &str) -> anyhow::Result<SigningKey> {
    let entry = keyring::Entry::new("intelexta", project_id)?;
    let b64 = entry.get_password()?;
    // Use the new Engine API
    let bytes = STANDARD.decode(b64)?;
    let sk = SigningKey::from_bytes(&bytes.try_into().map_err(|_| anyhow::anyhow!("bad sk len"))?);
    Ok(sk)
}

pub fn public_key_from_secret(sk: &SigningKey) -> String {
    let pk: VerifyingKey = sk.verifying_key();
    // Use the new Engine API
    STANDARD.encode(pk.as_bytes())
}

pub fn sign_bytes(sk: &SigningKey, bytes: &[u8]) -> String {
    let sig: Signature = sk.sign(bytes);
    // Use the new Engine API
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