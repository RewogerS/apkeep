use std::process::Command;

// A version-qualified app never reaches the network, even on the original PR.
// Only child-process environment is configured; the test process is not mutated.
fn run(options: &str, version: impl AsRef<std::ffi::OsStr>) -> String {
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_apkeep"))
        .args(["-a", "com.example.app@1", "-d", "ru-store", "-o", options])
        .arg(out.path())
        .env("APKEEP_RUSTORE_VER", version)
        .output()
        .unwrap();
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn cli_passes_options_to_rustore_before_processing_apps() {
    let output = run("sdk_version=0", "11500");
    assert!(
        output.contains("Invalid RuStore option sdk_version"),
        "{output}"
    );
    assert!(!output.contains("Specific versions"));
}

#[test]
fn cli_rejects_invalid_environment_without_echoing_it() {
    let output = run("", "secret-env-version");
    assert!(output.contains("APKEEP_RUSTORE_VER"), "{output}");
    assert!(!output.contains("secret-env-version"));
    assert!(!output.contains("Specific versions"));
}

#[test]
fn cli_explicit_version_overrides_invalid_environment() {
    let output = run("rustore_version=22000", "secret-env-version");
    assert!(output.contains("Specific versions"), "{output}");
    assert!(!output.contains("Invalid RuStore"));
}

#[cfg(unix)]
#[test]
fn cli_handles_non_unicode_environment_without_panicking() {
    use std::os::unix::ffi::OsStringExt;
    let version = std::ffi::OsString::from_vec(b"secret-env-\xff".to_vec());
    let output = run("", &version);
    assert!(output.contains("APKEEP_RUSTORE_VER"), "{output}");
    assert!(!output.contains("secret-env-"));
    let output = run("rustore_version=22000", &version);
    assert!(output.contains("Specific versions"), "{output}");
    assert!(!output.contains("Invalid RuStore"));
}
