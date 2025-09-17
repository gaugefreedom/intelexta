use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

/// Simple verification utility for Intelexta JSON artifacts.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the JSON file that should be verified.
    #[arg(long)]
    path: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let contents = fs::read_to_string(&cli.path)
        .with_context(|| format!("failed to read file: {}", cli.path.display()))?;

    let _value: serde_json::Value = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse JSON from: {}", cli.path.display()))?;

    println!("Verified (stub)");

    Ok(())
}
