use std::any::Any;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, Once,
};

use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi, CredentialPersistence};
use keyring::Error as KeyringError;

/// Shared service name used for all keychain entries written by the app.
pub const KEYCHAIN_SERVICE_NAME: &str = "intelexta";

static KEYCHAIN_INITIALIZED: Once = Once::new();
static FALLBACK_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct EntryKey {
    target: Option<String>,
    service: String,
    user: String,
}

#[derive(Clone, Debug, Default)]
struct InMemoryCredentialBuilder {
    store: Arc<Mutex<HashMap<EntryKey, Vec<u8>>>>,
}

#[derive(Clone, Debug)]
struct InMemoryCredential {
    key: EntryKey,
    store: Arc<Mutex<HashMap<EntryKey, Vec<u8>>>>,
}

/// Ensure that the keyring backend is usable.
///
/// On systems where the OS keychain is unavailable (for example, because
/// the secret-service D-Bus daemon is not running), this falls back to an
/// in-memory credential store so that development can continue.
///
pub fn ensure_available() {
    if using_in_memory_fallback() {
        return;
    }

    if should_force_in_memory() {
        install_in_memory_keyring();
        return;
    }

    KEYCHAIN_INITIALIZED.call_once(|| {
        if let Err(err) = probe_system_keyring() {
            eprintln!([
                "[intelexta] Falling back to in-memory keyring because the",
                "system keychain is unavailable:",
                &err.to_string(),
            ]
            .join(" "));
            install_in_memory_keyring();
        }
    });
}

/// Force the use of the in-memory keyring. This is primarily used by tests.
pub fn force_in_memory_keyring() {
    install_in_memory_keyring();
}

/// Returns true when the process-wide in-memory keyring is being used.
pub fn using_in_memory_fallback() -> bool {
    FALLBACK_ACTIVE.load(Ordering::SeqCst)
}

fn should_force_in_memory() -> bool {
    std::env::var("INTELEXTA_USE_IN_MEMORY_KEYCHAIN")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True"))
        .unwrap_or(false)
}

fn probe_system_keyring() -> keyring::Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE_NAME, "__intelexta_keychain_probe__")?;
    let test_secret = "__probe_secret__";
    entry.set_password(test_secret)?;
    let retrieved = entry.get_password()?;

    if retrieved != test_secret {
        return Err(KeyringError::BadEncoding(retrieved.into_bytes()));
    }

    match entry.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(err) => Err(err),
    }
}

fn install_in_memory_keyring() {
    let was_active = FALLBACK_ACTIVE.swap(true, Ordering::SeqCst);
    keyring::set_default_credential_builder(Box::new(InMemoryCredentialBuilder::default()));

    if !was_active {
        eprintln!(
            "[intelexta] Using in-memory keyring backend. Secrets will not persist between app runs."
        );
    }
}

impl CredentialBuilderApi for InMemoryCredentialBuilder {
    fn build(
        &self,
        target: Option<&str>,
        service: &str,
        user: &str,
    ) -> keyring::Result<Box<Credential>> {
        let key = EntryKey {
            target: target.map(|value| value.to_string()),
            service: service.to_string(),
            user: user.to_string(),
        };

        Ok(Box::new(InMemoryCredential {
            key,
            store: Arc::clone(&self.store),
        }))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn persistence(&self) -> CredentialPersistence {
        CredentialPersistence::ProcessOnly
    }
}

impl CredentialApi for InMemoryCredential {
    fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
        let mut store = self
            .store
            .lock()
            .expect("in-memory keyring store poisoned during set");
        store.insert(self.key.clone(), secret.to_vec());
        Ok(())
    }

    fn get_secret(&self) -> keyring::Result<Vec<u8>> {
        let store = self
            .store
            .lock()
            .expect("in-memory keyring store poisoned during get");
        store.get(&self.key).cloned().ok_or(KeyringError::NoEntry)
    }

    fn delete_credential(&self) -> keyring::Result<()> {
        let mut store = self
            .store
            .lock()
            .expect("in-memory keyring store poisoned during delete");

        if store.remove(&self.key).is_some() {
            Ok(())
        } else {
            Err(KeyringError::NoEntry)
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
