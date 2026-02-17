use std::process::Command;

fn tes_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tes"))
}

#[test]
fn local_add_ls_cat_export_rm() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("tesseras");
    let identity_flag = format!("--identity={}", data_dir.display());

    // Create a test file
    let test_file = tmp.path().join("photo.jpg");
    std::fs::write(&test_file, b"fake jpeg data for testing").unwrap();

    // Add
    let output = tes_cmd()
        .args([
            &identity_flag,
            "add",
            test_file.to_str().unwrap(),
            "--name",
            "Test Photo",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let hash = String::from_utf8(output.stdout).unwrap().trim().to_string();
    assert_eq!(hash.len(), 64, "expected 64-char hex hash, got: {hash}");

    // Ls
    let output = tes_cmd().args([&identity_flag, "ls"]).output().unwrap();
    assert!(output.status.success());
    let ls_out = String::from_utf8(output.stdout).unwrap();
    assert!(
        ls_out.contains("Test Photo"),
        "ls should show name: {ls_out}"
    );

    // Cat
    let output = tes_cmd()
        .args([&identity_flag, "cat", &hash])
        .output()
        .unwrap();
    assert!(output.status.success());
    let cat_out = String::from_utf8(output.stdout).unwrap();
    assert!(cat_out.contains("Test Photo"));
    assert!(cat_out.contains("photo.jpg"));

    // Export
    let export_dir = tmp.path().join("exported");
    let output = tes_cmd()
        .args([
            &identity_flag,
            "export",
            &hash,
            export_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "export failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let exported_file = export_dir.join("test-photo").join("photo.jpg");
    assert!(exported_file.exists(), "exported file should exist");
    assert_eq!(
        std::fs::read(&exported_file).unwrap(),
        b"fake jpeg data for testing"
    );

    // Rm
    let output = tes_cmd()
        .args([&identity_flag, "rm", &hash])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Ls should be empty now
    let output = tes_cmd().args([&identity_flag, "ls"]).output().unwrap();
    assert!(output.status.success());
    let ls_out = String::from_utf8(output.stdout).unwrap();
    assert!(!ls_out.contains("Test Photo"), "should be empty after rm");
}
