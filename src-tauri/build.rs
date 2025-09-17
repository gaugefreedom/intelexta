use std::path::Path;

fn main() {
    let schema_path = Path::new("../schemas/car-v0.2.schema.json");
    if !schema_path.exists() {
        panic!("missing required schema: {}", schema_path.display());
    }

    println!("cargo:rerun-if-changed=../schemas/car-v0.2.schema.json");

    tauri_build::build()
}
