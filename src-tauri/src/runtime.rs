use std::sync::OnceLock;

use anyhow::Result;

static INITIALIZED: OnceLock<()> = OnceLock::new();

pub fn initialize() -> Result<()> {
    INITIALIZED.get_or_init(|| ());
    Ok(())
}
