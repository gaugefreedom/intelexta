// Test binary to verify model catalog loading
use intelexta::model_catalog;

fn main() {
    println!("Testing model catalog loading...\n");

    println!("Current directory: {:?}", std::env::current_dir().unwrap());
    println!();

    match model_catalog::ModelCatalog::load_default() {
        Ok(catalog) => {
            println!("✓ Successfully loaded catalog!");
            println!("  Version: {}", catalog.version());
            println!("  Signature verified: {}", catalog.is_signature_verified());
            println!("  Number of models: {}", catalog.raw.models.len());
            println!();

            println!("Models:");
            for model in &catalog.raw.models {
                println!("  - {} ({})", model.id, model.provider);
                println!("    Display: {}", model.display_name);
                println!("    Network: {}, API Key: {}",
                    model.requires_network, model.requires_api_key);
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to load catalog: {}", e);
            eprintln!("  Using fallback catalog instead");

            let fallback = model_catalog::ModelCatalog::fallback_catalog();
            println!("\nFallback catalog has {} models", fallback.raw.models.len());
        }
    }
}
