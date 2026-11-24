use std::process::Command;

#[test]
fn shader_commands_report_missing_roots_and_scan_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    // Minimal DXBC header with no parts.
    let mut container = [0u8; 32];
    container[..4].copy_from_slice(b"DXBC");
    container[20] = 1; // major version
    container[24] = 32; // container size
    std::fs::write(dir.path().join("one"), container).unwrap();

    for command in ["shader-info", "translate"] {
        let output = Command::new(env!("CARGO_BIN_EXE_victory"))
            .current_dir(dir.path())
            .env_remove("VICTORY_PAIRS")
            .args([command, "first-input", "one", "second-input"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{command}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout
                .lines()
                .any(|line| line.starts_with("containers") && line.contains(" 1 (")),
            "{command}: {stdout}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        for name in ["first-input", "second-input", "traversal errors: 2"] {
            assert!(stderr.contains(name), "{command}: {stderr}");
        }
    }
}
