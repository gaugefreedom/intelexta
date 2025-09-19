use anyhow::{anyhow, Context};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

const KEYCHAIN_SERVICE_NAME: &str = "intelexta";

static USING_FALLBACK: AtomicBool = AtomicBool::new(false);
static INIT: Once = Once::new();

/// Initialize the keychain backend. This probes the system keyring and records whether
/// the application should fall back to the filesystem-based store.
pub fn initialize_backend() {
    // If the fallback has already been forced (e.g. by tests), avoid resetting it.
    if USING_FALLBACK.load(Ordering::SeqCst) {
        return;
    }

    INIT.call_once(|| {
        match probe_system_keyring() {
            Ok(()) => {
                println!("[intelexta] System keychain is available and working correctly.");
                USING_FALLBACK.store(false, Ordering::SeqCst);
            }
            Err(err) => {
                eprintln!(
                    "[intelexta] WARNING: System keychain failed probe ({}). Falling back to filesystem.",
                    err
                );
                USING_FALLBACK.store(true, Ordering::SeqCst);
            }
        }
    });
}

/// Store a secret for the provided project identifier.
pub fn store_secret(project_id: &str, secret_b64: &str) -> anyhow::Result<()> {
    initialize_backend();

    if !USING_FALLBACK.load(Ordering::SeqCst) {
        let entry =
            keyring::Entry::new(KEYCHAIN_SERVICE_NAME, project_id).map_err(|err| anyhow!(err))?;
        match entry.set_password(secret_b64) {
            Ok(()) => return Ok(()),
            Err(err) => {
                eprintln!(
                    "[intelexta] WARNING: Failed to store secret in system keychain ({}). Falling back to filesystem.",
                    err
                );
                USING_FALLBACK.store(true, Ordering::SeqCst);
            }
        }
    }

    let path = get_fallback_path(project_id)?;
    fs::write(&path, secret_b64).with_context(|| fallback_write_error(&path))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, permissions)
            .with_context(|| fallback_permissions_error(&path))?;
    }

    Ok(())
}

/// Load the stored secret for the provided project identifier.
pub fn load_secret(project_id: &str) -> anyhow::Result<String> {
    initialize_backend();

    if !USING_FALLBACK.load(Ordering::SeqCst) {
        let entry =
            keyring::Entry::new(KEYCHAIN_SERVICE_NAME, project_id).map_err(|err| anyhow!(err))?;
        match entry.get_password() {
            Ok(secret) => return Ok(secret),
            Err(keyring::Error::NoEntry) => return Err(anyhow!(keyring::Error::NoEntry)),
            Err(err) => {
                eprintln!(
                    "[intelexta] WARNING: Failed to read secret from system keychain ({}). Falling back to filesystem.",
                    err
                );
                USING_FALLBACK.store(true, Ordering::SeqCst);
            }
        }
    }

    let path = get_fallback_path(project_id)?;
    let secret = fs::read_to_string(&path).with_context(|| fallback_read_error(&path))?;
    Ok(secret)
}

fn probe_system_keyring() -> keyring::Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE_NAME, "__intelexta_probe__")?;
    let secret = "test_secret";
    entry.set_password(secret)?;
    let retrieved = entry.get_password()?;
    if retrieved != secret {
        let _ = entry.delete_credential();
        return Err(keyring::Error::NoEntry);
    }
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err),
    }
}

fn fallback_base_dir() -> anyhow::Result<PathBuf> {
    if let Ok(dir) = std::env::var("INTELEXTA_KEYCHAIN_DIR") {
        return Ok(PathBuf::from(dir));
    }

    dirs::data_local_dir()
        .ok_or_else(|| anyhow!("cannot find user local data directory"))
        .map(|path| path.join("com.intelexta.dev").join("keys"))
}

fn get_fallback_path(project_id: &str) -> anyhow::Result<PathBuf> {
    let base = fallback_base_dir()?;
    fs::create_dir_all(&base).with_context(|| fallback_dir_error(&base))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir_permissions = fs::Permissions::from_mode(0o700);
        fs::set_permissions(&base, dir_permissions)
            .with_context(|| fallback_permissions_error(&base))?;
    }
    Ok(base.join(format!("{}.key", project_id)))
}

fn fallback_dir_error(path: &Path) -> String {
    format!(
        "unable to create key fallback directory at {}",
        path.display()
    )
}

fn fallback_write_error(path: &Path) -> String {
    format!("unable to write fallback key file at {}", path.display())
}

fn fallback_permissions_error(path: &Path) -> String {
    format!(
        "unable to update permissions on fallback key path at {}",
        path.display()
    )
}

fn fallback_read_error(path: &Path) -> String {
    format!("unable to read fallback key file at {}", path.display())
}

#[cfg(test)]
pub(crate) fn force_fallback_for_tests() {
    USING_FALLBACK.store(true, Ordering::SeqCst);
}
