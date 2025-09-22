use std::path::PathBuf;
use std::process::Command;

#[test]
fn verifies_sample_json() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sample_path = manifest_dir.join("tests").join("data").join("sample.json");

    let binary_path = std::env::var("CARGO_BIN_EXE_intelexta-verify")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let target_base = std::env::var("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    manifest_dir
                        .parent()
                        .and_then(|p| p.parent())
                        .map(|p| p.join("target"))
                        .expect("failed to determine workspace target directory")
                });

            let mut path = target_base;
            path.push("debug");
            path.push(if cfg!(windows) {
                "intelexta-verify.exe"
            } else {
                "intelexta-verify"
            });

            path
        });

    assert!(
        binary_path.exists(),
        "expected intelexta-verify binary at {}",
        binary_path.display()
    );

    let output = Command::new(binary_path)
        .arg("--path")
        .arg(&sample_path)
        .output()
        .expect("failed to invoke intelexta-verify binary");

    assert!(
        output.status.success(),
        "binary failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Verified (stub)"),
        "unexpected stdout: {}",
        stdout
    );
}
