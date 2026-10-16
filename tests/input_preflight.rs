//! Offline child-process preflight: proxy is a closed loopback port, never a store.
use std::fs;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn run(args: &[&str], ini: Option<&str>) -> Output {
    run_with_setup(args, ini, false)
}

fn run_with_setup(args: &[&str], ini: Option<&str>, dangling_default: bool) -> Output {
    let dir = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    if dangling_default {
        let config = dir.path().join("config/apkeep");
        fs::create_dir_all(&config).unwrap();
        std::os::unix::fs::symlink(config.join("absent-target"), config.join("apkeep.ini"))
            .unwrap();
    }
    if let Some(contents) = ini {
        fs::write(dir.path().join("config.ini"), contents).unwrap();
    }
    let stdout = dir.path().join("stdout");
    let stderr = dir.path().join("stderr");
    #[cfg(unix)]
    let mut cmd = {
        let mut cmd = Command::new("setsid");
        cmd.arg(env!("CARGO_BIN_EXE_apkeep"));
        cmd
    };
    #[cfg(not(unix))]
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_apkeep"));
    let mut child = cmd
        .args(args)
        .current_dir(dir.path())
        .env_clear()
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout).unwrap())
        .stderr(fs::File::create(&stderr).unwrap())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("preflight hung");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: fs::read(stdout).unwrap(),
        stderr: fs::read(stderr).unwrap(),
    }
}

#[test]
fn malformed_explicit_ini_is_rejected_even_with_complete_cli_credentials_without_leaking_values() {
    let out = run(
        &[
            "-d",
            "google-play",
            "-a",
            "com.example.app",
            "-i",
            "config.ini",
            "-e",
            "fixture@example.invalid",
            "-t",
            "FAKE_TEST_CREDENTIAL",
            ".",
        ],
        Some("[SECRET_INI_MARKER\n"),
    );
    assert_eq!(out.status.code(), Some(1));
    let error = String::from_utf8_lossy(&out.stderr);
    assert!(error.contains("configuration"), "{error}");
    assert!(!error.contains("SECRET_INI_MARKER") && !error.contains("FAKE_TEST_CREDENTIAL"));
    assert!(out.stdout.is_empty());
}

#[test]
fn missing_default_ini_is_optional_but_existing_malformed_explicit_ini_is_not() {
    let out = run(
        &[
            "-d",
            "google-play",
            "-a",
            "com.example.app",
            "-i",
            "config.ini",
            ".",
        ],
        Some("[MALFORMED_PRIVATE_VALUE\n"),
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("configuration"));
    assert!(out.stdout.is_empty());
}

#[test]
fn credential_prompt_refuses_non_terminal_without_echo_or_login() {
    let out = run(
        &[
            "-d",
            "google-play",
            "-a",
            "com.example.app",
            "-e",
            "fixture@example.invalid",
            ".",
        ],
        None,
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("terminal"));
    assert!(out.stdout.is_empty());
}

#[test]
fn oauth_listing_is_an_explicit_error_not_silent_success() {
    let out = run(
        &[
            "--oauth-token",
            "FAKE_TEST_CREDENTIAL",
            "-e",
            "fixture@example.invalid",
            "-l",
        ],
        None,
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("OAuth"));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("FAKE_TEST_CREDENTIAL"));
    assert!(out.stdout.is_empty());
}

#[cfg(unix)]
#[test]
fn dangling_default_ini_is_an_error_not_absent_configuration() {
    let out = run_with_setup(
        &[
            "-d",
            "google-play",
            "-a",
            "com.example.app",
            "-e",
            "fixture@example.invalid",
            "-t",
            "FAKE_TEST_CREDENTIAL",
            ".",
        ],
        None,
        true,
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("configuration"));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("FAKE_TEST_CREDENTIAL"));
}

#[test]
fn fdroid_rejects_untrusted_app_without_echoing_it() {
    let out = run(
        &[
            "-d",
            "f-droid",
            "-a",
            "DO_NOT_ECHO_FIXTURE\norg.invalid",
            ".",
        ],
        None,
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("DO_NOT_ECHO_FIXTURE"));
    assert!(!String::from_utf8_lossy(&out.stdout).contains("DO_NOT_ECHO_FIXTURE"));
}

#[test]
fn oauth_missing_email_is_rejected_without_app_selection_or_network() {
    let out = run(&["--oauth-token", "FAKE_TEST_CREDENTIAL"], None);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("email"));
    assert!(out.stdout.is_empty());
}
