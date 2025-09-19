use crate::keychain;
use anyhow::anyhow;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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
    keychain::store_secret(project_id, secret_key_b64)
}

pub fn load_secret_key(project_id: &str) -> anyhow::Result<SigningKey> {
    let b64 = keychain::load_secret(project_id)?;
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

pub fn canonical_json<T: Serialize>(t: &T) -> Vec<u8> {
    serde_jcs::to_vec(t).expect("canonical json")
}

pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(data))
}

pub fn semantic_digest(text: &str) -> String {
    const BITS: usize = 64;

    if text.trim().is_empty() {
        return format!("{:016x}", 0_u64);
    }

    let normalized = text.to_lowercase();
    let mut features: Vec<String> = Vec::new();

    let chars: Vec<char> = normalized.chars().collect();
    if chars.len() >= 3 {
        for window in chars.windows(3) {
            let gram: String = window.iter().collect();
            features.push(gram);
        }
    }

    if features.is_empty() {
        for token in normalized.split_whitespace() {
            if !token.is_empty() {
                features.push(token.to_string());
            }
        }
    }

    if features.is_empty() {
        features.push(normalized);
    }

    let mut weights = [0_i64; BITS];

    for feature in features {
        let mut hasher = DefaultHasher::new();
        feature.hash(&mut hasher);
        let hash = hasher.finish();
        for bit in 0..BITS {
            if (hash >> bit) & 1 == 1 {
                weights[bit] += 1;
            } else {
                weights[bit] -= 1;
            }
        }
    }

    let mut digest: u64 = 0;
    for bit in 0..BITS {
        if weights[bit] >= 0 {
            digest |= 1 << bit;
        }
    }

    format!("{:016x}", digest)
}

pub fn semantic_distance(a: &str, b: &str) -> Option<u32> {
    let left = u64::from_str_radix(a, 16).ok()?;
    let right = u64::from_str_radix(b, 16).ok()?;
    Some((left ^ right).count_ones())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Serialize)]
    struct S {
        b: u8,
        a: u8,
    }

    #[test]
    fn canon_same_struct_same_bytes() {
        let s1 = S { b: 2, a: 1 };
        let s2 = S { a: 1, b: 2 };
        let c1 = canonical_json(&s1);
        let c2 = canonical_json(&s2);
        assert_eq!(c1, c2, "JCS must produce identical bytes");
    }

    #[test]
    fn semantic_digest_close_texts_have_small_distance() {
        let original = "Hello world from intelexta";
        let variant = "hello world from Intelexta!";
        let digest_a = semantic_digest(original);
        let digest_b = semantic_digest(variant);
        let distance = semantic_distance(&digest_a, &digest_b).expect("valid digests");
        assert!(
            distance <= 8,
            "expected small hamming distance, got {distance}"
        );
    }

    #[test]
    fn semantic_digest_detects_large_difference() {
        let digest_a = semantic_digest("aaaaaaaaaa");
        let digest_b = semantic_digest("zzzzzzzzzz");
        let distance = semantic_distance(&digest_a, &digest_b).expect("valid digests");
        assert!(distance > 0);
    }
}
