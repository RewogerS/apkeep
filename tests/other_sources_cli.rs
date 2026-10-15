use std::fs;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn run(args: &[&str], files: &[(&str, &str)]) -> Output {
    let root = tempfile::tempdir().unwrap();
    for (name, contents) in files {
        fs::write(root.path().join(name), contents).unwrap();
    }
    let stdout_path = root.path().join("stdout");
    let stderr_path = root.path().join("stderr");
    let mut child = Command::new(env!("CARGO_BIN_EXE_apkeep"))
        .args(args)
        .current_dir(root.path())
        .env_clear()
        .env("HOME", root.path())
        .env("XDG_CONFIG_HOME", root.path().join("config"))
        .env("TMPDIR", root.path())
        .env("LANG", "C")
        .env("TZ", "UTC")
        .env("NO_PROXY", "*")
        .stdin(Stdio::null())
        // Files, rather than unconsumed pipes, cannot deadlock on full buffers.
        .stdout(Stdio::from(fs::File::create(&stdout_path).unwrap()))
        .stderr(Stdio::from(fs::File::create(&stderr_path).unwrap()))
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
            panic!("offline CLI exceeded deadline: {args:?}");
        }
        thread::sleep(Duration::from_millis(10));
    };
    let output = Output {
        status,
        stdout: fs::read(stdout_path).unwrap(),
        stderr: fs::read(stderr_path).unwrap(),
    };
    assert!(!root.path().join("com.example.fixture.apk").exists());
    output
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).unwrap()
}

#[test]
fn google_play_listing_is_latest_only_and_does_not_require_credentials_or_outpath() {
    let output = run(
        &["-d", "google-play", "-l", "-a", "com.example.fixture@1.2.3"],
        &[],
    );
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(stdout(&output), "Versions available for com.example.fixture on Google Play:\n| Google Play does not make old versions of apps available.\n");
}

#[test]
fn google_play_csv_listing_preserves_order_without_login() {
    let output = run(
        &["-d", "google-play", "-l", "-c", "apps.csv", "-v", "2"],
        &[("apps.csv", "com.example.first,1\ncom.example.second,2\n")],
    );
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(stdout(&output), "Versions available for com.example.first on Google Play:\n| Google Play does not make old versions of apps available.\nVersions available for com.example.second on Google Play:\n| Google Play does not make old versions of apps available.\n");
}

#[test]
fn google_play_listing_does_not_read_ini_or_apply_json_option() {
    // Characterize current dispatch: listing is plaintext even with json option,
    // and an invalid explicit INI is ignored because there is no login.
    let output = run(
        &[
            "-d",
            "google-play",
            "-l",
            "-a",
            "com.example.fixture",
            "-i",
            "missing.ini",
            "-o",
            "output_format=json",
        ],
        &[],
    );
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(stdout(&output), "Versions available for com.example.fixture on Google Play:\n| Google Play does not make old versions of apps available.\n");
}

#[test]
fn google_play_rejects_conflicting_token_switches_without_echoing_values() {
    let output = run(
        &[
            "-d",
            "google-play",
            "-l",
            "-a",
            "com.example.fixture",
            "--aas-token",
            "FAKE_AAS_SECRET",
            "--auth-token",
            "FAKE_AUTH_SECRET",
        ],
        &[],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with"), "{stderr}");
    assert!(!stderr.contains("FAKE_AAS_SECRET"));
    assert!(!stderr.contains("FAKE_AUTH_SECRET"));
}

#[test]
fn google_play_invalid_explicit_ini_is_rejected_before_prompt_or_login() {
    for ini in ["missing.ini", "."] {
        let output = run(
            &[
                "-d",
                "google-play",
                "-a",
                "com.example.fixture",
                "-i",
                ini,
                ".",
            ],
            &[],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(stdout(&output).contains("Specified ini is not a valid file"));
        assert!(!stdout(&output).contains("Email:"));
        assert!(!stdout(&output).contains("AAS Token"));
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn google_play_oauth_missing_value_is_a_parser_error_not_a_login() {
    let output = run(&["--oauth-token"], &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--oauth-token"));
}

#[test]
fn google_play_oauth_list_currently_has_no_apps_and_never_logs_in() {
    // OAuth suppresses app/CSV selection in main, even when -a is present.
    // This is characterization of questionable CLI semantics, not endorsement.
    let output = run(
        &[
            "--oauth-token",
            "FAKE_OAUTH_NOT_A_CREDENTIAL",
            "-l",
            "-a",
            "com.example.fixture",
            "-d",
            "apk-pure",
        ],
        &[],
    );
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn apkpure_download_requires_explicit_danger_acknowledgement_before_network() {
    for options in ["", "acknowledge_dangers=false", "acknowledge_dangers=0"] {
        let output = run(
            &[
                "-d",
                "apk-pure",
                "-a",
                "com.example.fixture",
                "-o",
                options,
                ".",
            ],
            &[],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("must be explicitly acknowledged"),
            "{stderr}"
        );
        assert!(!stderr.contains("panicked"));
    }
}
