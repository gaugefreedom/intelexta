// Integration test for model catalog loading
use intelexta::model_catalog::ModelCatalog;
use std::path::PathBuf;

#[test]
fn test_load_catalog_from_config() {
    let mut catalog_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    catalog_path.push("..");
    catalog_path.push("config");
    catalog_path.push("model_catalog.toml");

    println!("Loading catalog from: {:?}", catalog_path);

    let catalog = ModelCatalog::load_from_path(&catalog_path)
        .expect("Should load catalog from config/model_catalog.toml");

    // Verify catalog metadata
    assert_eq!(catalog.version(), "1.0.0");

    // Verify stub model exists
    let stub_model = catalog.get_model("stub-model")
        .expect("Stub model should exist");
    assert_eq!(stub_model.provider, "internal");
    assert_eq!(stub_model.cost_per_million_tokens, 0.0);

    // Verify Ollama models exist
    assert!(catalog.get_model("llama3.2:1b").is_some());
    assert!(catalog.get_model("llama3.2:3b").is_some());
    assert!(catalog.get_model("llama3.1:8b").is_some());

    // Test cost calculation for local model (should be free)
    let cost = catalog.calculate_usd_cost("llama3.2:1b", 1_000_000);
    assert_eq!(cost, 0.0, "Local Ollama models should have zero USD cost");

    // Test nature cost calculation
    let nature_cost = catalog.calculate_nature_cost("llama3.2:1b", 1_000_000);
    assert_eq!(nature_cost, 2.5, "Llama 3.2 1B should have 2.5 nature cost per million tokens");

    // Test energy calculation
    let energy = catalog.calculate_energy_kwh("llama3.2:1b", 1_000_000);
    assert_eq!(energy, 0.05, "Llama 3.2 1B should use 0.05 kWh per million tokens");

    println!("✅ Model catalog integration test passed!");
}

#[test]
fn test_catalog_with_unknown_model() {
    let mut catalog_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    catalog_path.push("..");
    catalog_path.push("config");
    catalog_path.push("model_catalog.toml");

    let catalog = ModelCatalog::load_from_path(&catalog_path)
        .expect("Should load catalog");

    // Unknown model should use fallback values
    let cost = catalog.calculate_usd_cost("unknown-model", 1_000_000);
    assert_eq!(cost, 10.0, "Unknown model should use fallback cost of $10 per million");

    let nature_cost = catalog.calculate_nature_cost("unknown-model", 1_000_000);
    assert_eq!(nature_cost, 5.0, "Unknown model should use fallback nature cost of 5.0");
}

#[test]
fn test_catalog_providers() {
    let mut catalog_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    catalog_path.push("..");
    catalog_path.push("config");
    catalog_path.push("model_catalog.toml");

    let catalog = ModelCatalog::load_from_path(&catalog_path)
        .expect("Should load catalog");

    // Check provider info
    let ollama = catalog.get_provider("ollama")
        .expect("Ollama provider should exist");
    assert_eq!(ollama.name, "Ollama");
    assert!(!ollama.requires_network);

    let openai = catalog.get_provider("openai")
        .expect("OpenAI provider should exist");
    assert_eq!(openai.name, "OpenAI");
    assert!(openai.requires_network);
    assert!(openai.requires_api_key);
}
