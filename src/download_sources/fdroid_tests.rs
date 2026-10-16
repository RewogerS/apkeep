//! Offline fixtures exercise the real parsers, not public repositories.
//!
//! SIGNATURE_B64 is a real detached RSA/SHA256 CMS signature over SIGNED_FILE,
//! generated with OpenSSL (`cms -sign -binary -noattr -md sha256`) and a
//! self-signed test-only certificate. The private key was discarded. This proves
//! the pinned signature/digest chain for these bytes, not public F-Droid trust.
const MANIFEST: &str = "Manifest-Version: 1.0\r\n\r\nName: entry.json\r\nSHA-256-Digest: RBNvo1WzZ4oRRq0W9+hknpT7T8If536DEMBg9hyq/4o=\r\n\r\nName: index-v1.json\r\nSHA1-Digest: vyGp6PvFo4RvsFtPoIWeCReyIC8=\r\n\r\n";
const SIGNED_FILE: &str = "Signature-Version: 1.0\r\nSHA-256-Digest-Manifest: TRMvRy7qnA9kO8gC4F8RjM/qvK/qW5RAM0xY1zLXz8U=\r\nSHA1-Digest-Manifest: 8RXT7YZitDD6PePr+0FK/u2rHVA=\r\n\r\n";
const FINGERPRINT: &str = "a865115a6025a69cb3e2b28f590513d32a6ec5d9d1b0a6ee4e1b0f9fa7a51586";
const SIGNATURE_B64: &str = "MIIE0gYJKoZIhvcNAQcCoIIEwzCCBL8CAQExDTALBglghkgBZQMEAgEwCwYJKoZIhvcNAQcBoIIDMTCCAy0wggIVoAMCAQICFBCjWbR9MMkv3MWsbKQDL7Es8jnWMA0GCSqGSIb3DQEBCwUAMCYxJDAiBgNVBAMMG2Fwa2VlcCBvZmZsaW5lIHRlc3QgZml4dHVyZTAeFw0yNjEwMTUyMTU3MjBaFw0zNjEwMTIyMTU3MjBaMCYxJDAiBgNVBAMMG2Fwa2VlcCBvZmZsaW5lIHRlc3QgZml4dHVyZTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAN/BItmg1hxe7BjEPwhIm4LONGcGGJc2tCAh0+hwVTIN7SX/c8/bfAYrjzfOSrH5OBvZZ1ZjvNYQbZ/69qRkgXpBLm1bSrHft45F4dSidzcX8lpqUMIjvmUozIqHRLgr7X9ZJXxsD7YtQHi2vgou4fiJEM0wPGaPKtuJ1InVM9DIpp2OXf2BdaP5OeS/OqfwRhNUMya7APLRCReQG3CS013TXSHdIgVB8nhbcv+s0fIhLigIa0MiXdfG9EfIGVz5w2cMlPkCFhZPtDBC0JC3iBRgQa0AMMCTRw5oSwg3L9Au3tyEseuZORI13LLNjK997R32dcYmmj3kw+pnq3TAuQsCAwEAAaNTMFEwHQYDVR0OBBYEFCZ9RVVy+TKYVRWSAa61nO9NyJX4MB8GA1UdIwQYMBaAFCZ9RVVy+TKYVRWSAa61nO9NyJX4MA8GA1UdEwEB/wQFMAMBAf8wDQYJKoZIhvcNAQELBQADggEBAIPxbkvNEbrKS5lQiFpgRJhoUcEgSgkWFnRNggHDUAlPWFZSDc5SmSUywdangqbjfEwd7SB0oyJ1X5uvyMb8jK8zVpeK2/9OwKz8EzeVo1Kv3H5SB+lM3O45rFiD7ibeA9LtaA/Fm8gOylolukps79vqcFuRKrhZGR6/JprxwufWvfy3YNaR6gM4WWjs+j9asAST6xrSLnUVbOOlamRYPYPARk+9/WdNZgBBFIZ5vGOyKMgb8nMUzcL8Mx4Pv38+RLxE1KQRbXG0e1Za0B/odpBbZVE/tB3teAbXoRcVPz13MaHjrFGjLX9DZs+yDpsBdFlPXBGjN98dbvlcw7P7FNQxggFnMIIBYwIBATA+MCYxJDAiBgNVBAMMG2Fwa2VlcCBvZmZsaW5lIHRlc3QgZml4dHVyZQIUEKNZtH0wyS/cxaxspAMvsSzyOdYwCwYJYIZIAWUDBAIBMA0GCSqGSIb3DQEBAQUABIIBAFOXnnZCTCKTF5RuTWoO8SkkKB7/kLMJ7VF4y/t3OS8LjJFnfxomefl5TyLLHQJ8LyY/gPN5vlLa44woSXYrLktQzYuhgabbLZS25pN9v8Ln88h/+vH42oNQ67OxvAcsWikEf7eL9NxcQ3OW9b6+uVLWuwei0LUGq1+qd3VODVviio2Cy+mLMLDfis3AlE5CwaxbBHVmdcVIJ6kSEM2l3HpESd/08QKV2PU6wYnqKz5BLabbcOL+ZtBJ4vWm9LRX0G2vRv/UetVTmE42FpI6TCOOcO5qfHktBf18A8iCk8DYlszQrMkiIBx5AeSyAt8gRFaoCLNX8Wh+Dzp7c1Lgmjw=";
use super::*;

const APP: &str = "org.example.app";
const REPO: &str = "https://repository.invalid/repo";

fn signed_fixture() -> (TempDir, Vec<String>, Vec<u8>) {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("META-INF")).unwrap();
    fs::write(
        dir.path().join("META-INF/FIXTURE.RSA"),
        b64_general_purpose::STANDARD.decode(SIGNATURE_B64).unwrap(),
    )
    .unwrap();
    fs::write(dir.path().join("META-INF/FIXTURE.SF"), SIGNED_FILE).unwrap();
    fs::write(dir.path().join("META-INF/MANIFEST.MF"), MANIFEST).unwrap();
    fs::write(dir.path().join("entry.json"), "{}").unwrap();
    fs::write(dir.path().join("index-v1.json"), "{}").unwrap();
    (
        dir,
        vec!["META-INF/FIXTURE.RSA".to_owned()],
        hex::decode(FINGERPRINT).unwrap(),
    )
}

#[test]
fn signed_fixture_verifies_real_signature_pin_and_both_digest_chains() {
    let (dir, files, pin) = signed_fixture();
    for entry in [true, false] {
        assert_eq!(
            verify_and_return_json(&dir, &files, &pin, true, entry, progress()).unwrap(),
            "{}"
        );
    }
}

#[test]
fn verifier_rejects_missing_or_multiple_signature_blocks() {
    let dir = tempdir().unwrap();
    let missing = verify_and_return_json(&dir, &[], &[], true, true, progress()).unwrap_err();
    assert!(missing.to_string().contains("no certificate"));
    let files = vec!["META-INF/A.RSA".to_owned(), "META-INF/B.EC".to_owned()];
    let multiple = verify_and_return_json(&dir, &files, &[], true, true, progress()).unwrap_err();
    assert!(multiple.to_string().contains("multiple certificate"));
}

#[test]
fn verifier_returns_error_for_missing_signature_manifest_or_json_files() {
    for missing in [
        "META-INF/FIXTURE.RSA",
        "META-INF/FIXTURE.SF",
        "META-INF/MANIFEST.MF",
        "entry.json",
    ] {
        let (dir, files, pin) = signed_fixture();
        fs::remove_file(dir.path().join(missing)).unwrap();
        assert!(
            verify_and_return_json(&dir, &files, &pin, true, true, progress()).is_err(),
            "missing: {missing}"
        );
    }
}

#[test]
fn verifier_rejects_wrong_pinned_fingerprint() {
    let (dir, files, _) = signed_fixture();
    let err = verify_and_return_json(&dir, &files, &[0; 32], true, true, progress()).unwrap_err();
    assert!(err.to_string().contains("Fingerprint"));
}

#[test]
fn verifier_rejects_malformed_cms_and_tampered_signed_file() {
    for corrupt_cms in [true, false] {
        let (dir, files, pin) = signed_fixture();
        if corrupt_cms {
            fs::write(dir.path().join(&files[0]), b"not a CMS signature").unwrap();
        } else {
            fs::write(
                dir.path().join("META-INF/FIXTURE.SF"),
                SIGNED_FILE.replace("Signature-Version: 1.0", "Signature-Version: 9.9"),
            )
            .unwrap();
        }
        assert!(verify_and_return_json(&dir, &files, &pin, true, true, progress()).is_err());
    }
}

#[test]
fn verifier_rejects_manifest_tampering_for_both_index_formats() {
    for entry in [true, false] {
        let (dir, files, pin) = signed_fixture();
        fs::write(
            dir.path().join("META-INF/MANIFEST.MF"),
            format!("{MANIFEST}tampered"),
        )
        .unwrap();
        let err = verify_and_return_json(&dir, &files, &pin, true, entry, progress()).unwrap_err();
        assert!(
            err.to_string()
                .contains("from the signed file does not match"),
            "{err}"
        );
    }
}

#[test]
fn verifier_rejects_json_tampering_for_both_index_formats() {
    for entry in [true, false] {
        let (dir, files, pin) = signed_fixture();
        let name = if entry { "entry.json" } else { "index-v1.json" };
        fs::write(dir.path().join(name), "{\"tampered\":true}").unwrap();
        let err = verify_and_return_json(&dir, &files, &pin, true, entry, progress()).unwrap_err();
        assert!(
            err.to_string()
                .contains("from the manifest file does not match"),
            "{err}"
        );
    }
}

#[test]
fn verification_disabled_is_an_explicit_bypass_not_a_signature_test() {
    let (dir, files, _) = signed_fixture();
    fs::write(dir.path().join(&files[0]), b"invalid signature").unwrap();
    fs::write(dir.path().join("META-INF/MANIFEST.MF"), b"no digest").unwrap();
    fs::write(dir.path().join("entry.json"), "{\"unsigned\":true}").unwrap();
    assert_eq!(
        verify_and_return_json(&dir, &files, &[], false, true, progress()).unwrap(),
        "{\"unsigned\":true}"
    );
}

#[test]
fn verifier_rejects_non_utf8_manifest_even_with_verification_disabled() {
    let (dir, files, pin) = signed_fixture();
    fs::write(dir.path().join("META-INF/MANIFEST.MF"), [0xff]).unwrap();
    assert!(verify_and_return_json(&dir, &files, &pin, false, true, progress()).is_err());
}

#[tokio::test]
async fn entry_rejects_unsafe_index_names_before_http_or_file_creation() {
    // Bound but deliberately unanswered: an HTTP attempt makes the timeout fail.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let repo = format!("http://{}", listener.local_addr().unwrap());
    let dir = tempdir().unwrap();
    for name in [
        "",
        "../escape.json",
        "nested/../../escape.json",
        "./index.json",
        "a//b.json",
        "..\\escape.json",
        "%2e%2e/escape.json",
        "index.json?query=1",
        "index.json#fragment",
        "https://evil.invalid/index.json",
    ] {
        let entry = json!({"index": {"name": name, "sha256": "ab".repeat(32)}}).to_string();
        let result = tokio::time::timeout(
            Duration::from_millis(200),
            verify_and_return_index_from_entry(
                &dir,
                &repo,
                &entry,
                true,
                progress(),
                OutputFormat::Plaintext,
            ),
        )
        .await;
        assert!(result.is_ok(), "must reject name before HTTP: {name}");
        let err = result.unwrap().unwrap_err();
        assert!(err.to_string().contains("index name"), "{err}");
        assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn entry_rejects_malformed_or_wrong_length_sha256_before_http() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let repo = format!("http://{}", listener.local_addr().unwrap());
    let dir = tempdir().unwrap();
    for hash in [
        "".to_owned(),
        "not hex".to_owned(),
        "00".repeat(20),
        "ab".repeat(31),
        "ab".repeat(33),
    ] {
        let entry = json!({"index": {"name": "/index-v2.json", "sha256": hash}}).to_string();
        let result = tokio::time::timeout(
            Duration::from_millis(200),
            verify_and_return_index_from_entry(
                &dir,
                &repo,
                &entry,
                true,
                progress(),
                OutputFormat::Plaintext,
            ),
        )
        .await;
        assert!(result.is_ok(), "must reject hash before HTTP: {hash}");
        assert!(result.unwrap().unwrap_err().to_string().contains("sha256"));
        assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn entry_downloads_safe_leaf_name_and_rejects_sha256_mismatch_on_loopback() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for name in ["/index-v2.json", "index-v2.json"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let repo = format!("http://{}/repo", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut buf = [0; 1024];
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buf[..n]);
                if request.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                assert!(request.len() < 8192);
            }
            assert!(String::from_utf8(request)
                .unwrap()
                .starts_with("GET /repo/index-v2.json HTTP/1.1\r\n"));
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .await
                .unwrap();
        });
        let dir = tempdir().unwrap();
        let entry = json!({"index": {"name": name, "sha256": "ab".repeat(32)}}).to_string();
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            verify_and_return_index_from_entry(
                &dir,
                &repo,
                &entry,
                true,
                progress(),
                OutputFormat::Plaintext,
            ),
        )
        .await
        .expect("loopback download timed out");
        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap();
        assert!(result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("sha256"));
        assert!(
            !dir.path().join("index-v2.json").exists(),
            "checksum mismatch must not publish an index"
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn entry_download_flushes_the_complete_index_before_reading() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let valid_hash = hex::encode(Sha256::digest(b"{}"));
    for (name, hash, verify, succeeds) in [
        ("/index-v2.json", valid_hash.as_str(), true, true),
        ("index-v2.json", valid_hash.as_str(), true, true),
        (
            "/index-v2.json",
            "abababababababababababababababababababababababababababababababab",
            true,
            false,
        ),
        ("/index-v2.json", "intentionally-unverified", false, true),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let repo = format!("http://{}/repo", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut buf = [0; 1024];
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buf[..n]);
                if request.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                assert!(request.len() < 8192);
            }
            assert!(String::from_utf8(request)
                .unwrap()
                .starts_with("GET /repo/index-v2.json HTTP/1.1\r\n"));
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .await
                .unwrap();
        });
        let dir = tempdir().unwrap();
        let entry = json!({"index": {"name": name, "sha256": hash}}).to_string();
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            verify_and_return_index_from_entry(
                &dir,
                &repo,
                &entry,
                verify,
                progress(),
                OutputFormat::Plaintext,
            ),
        )
        .await
        .expect("loopback download timed out");
        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap();
        if succeeds {
            assert_eq!(result.unwrap(), "{}");
        } else {
            assert!(result
                .unwrap_err()
                .to_string()
                .to_lowercase()
                .contains("sha256"));
        }
        if succeeds {
            assert_eq!(fs::read(dir.path().join("index-v2.json")).unwrap(), b"{}");
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        } else {
            assert!(!dir.path().join("index-v2.json").exists());
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
        }
    }
}

#[tokio::test]
async fn entry_missing_or_wrong_shape_metadata_returns_error_without_http() {
    let dir = tempdir().unwrap();
    for entry in [
        Value::Null,
        json!([]),
        json!({}),
        json!({"index": []}),
        json!({"index": {"name": 1, "sha256": "ab".repeat(32)}}),
        json!({"index": {"name": "/index-v2.json", "sha256": null}}),
    ] {
        assert!(verify_and_return_index_from_entry(
            &dir,
            "http://127.0.0.1:9",
            &entry.to_string(),
            true,
            progress(),
            OutputFormat::Plaintext
        )
        .await
        .is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

fn progress() -> Rc<MultiProgress> {
    Rc::new(MultiProgress::with_draw_target(
        indicatif::ProgressDrawTarget::hidden(),
    ))
}

fn index(package: Value) -> Value {
    json!({"repo": {"address": REPO}, "packages": {APP: package}})
}

fn v1(version: &str, filename: &str, arch: &str) -> Value {
    json!({"versionName": version, "apkName": filename, "hash": "ab".repeat(32), "nativecode": [arch]})
}

fn v2(code: u64, version: &str, filename: &str, arch: &str) -> Value {
    json!({"manifest": {"versionCode": code, "versionName": version, "nativecode": [arch]}, "file": {"name": filename, "sha256": "ab".repeat(32)}})
}

fn parse(index: Value, version: Option<&str>, arch: Option<&str>) -> DownloadInformation {
    parse_json_for_download_information(
        index,
        vec![(APP.to_owned(), version.map(str::to_owned))],
        arch.map(str::to_owned),
        progress(),
    )
    .expect("valid index envelope")
}

fn assert_selected(info: DownloadInformation, version: Option<&str>, filename: &str) {
    assert_eq!(
        info,
        (
            vec![(
                APP.to_owned(),
                version.map(str::to_owned),
                filename.to_owned(),
                vec![0xab; 32]
            )],
            REPO.to_owned()
        )
    );
}

#[test]
fn reliability_rejects_unsafe_apk_repository_paths_in_both_formats() {
    for name in [
        "../escape.apk",
        "a/../escape.apk",
        "//escape.apk",
        "%2e%2e.apk",
        "app.apk?x=1",
        "a\\b.apk",
        "https://evil.invalid/a.apk",
    ] {
        for package in [
            json!([v1("1.0", name, "x86")]),
            json!({"versions": {"a": v2(1, "1.0", name, "x86")}}),
        ] {
            assert!(
                parse(index(package), None, None).0.is_empty(),
                "unsafe APK path selected: {name}"
            );
        }
    }
}

#[test]
fn reliability_list_empty_missing_and_partial_requests_are_errors() {
    for apps in [
        vec![],
        vec![("missing.app".to_string(), None)],
        vec![(APP.to_string(), None), ("missing.app".to_string(), None)],
    ] {
        assert!(parse_json_display_versions(
            index(json!([v1("1.0", "app.apk", "x86")])),
            apps,
            OutputFormat::Json
        )
        .is_err());
    }
}

#[tokio::test]
async fn reliability_invalid_public_requests_fail_before_http() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let repo = format!("http://{}", listener.local_addr().unwrap());
    let options = HashMap::from([("repo", repo.as_str())]);
    let dir = tempdir().unwrap();
    let bad = vec![("../escape".to_string(), None)];
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        download_apps(bad.clone(), 1, 0, dir.path(), options.clone()),
    )
    .await;
    assert!(result.is_ok(), "invalid input must be rejected before HTTP");
    assert!(!result.unwrap());
    assert!(!list_versions(bad, options).await);
    assert!(matches!(listener.accept(), Err(err) if err.kind() == io::ErrorKind::WouldBlock));
}

#[test]
fn v1_latest_uses_repository_order() {
    assert_selected(
        parse(
            index(json!([
                v1("2.0", "new.apk", "arm64-v8a"),
                v1("1.0", "old.apk", "x86")
            ])),
            None,
            None,
        ),
        None,
        "new.apk",
    );
}

#[test]
fn v1_pinned_version_and_abi_select_matching_variant() {
    let packages = json!([
        v1("2.0", "new.apk", "arm64-v8a"),
        v1("1.0", "arm.apk", "arm64-v8a"),
        v1("1.0", "x86.apk", "x86")
    ]);
    assert_selected(
        parse(index(packages), Some("1.0"), Some("x86")),
        Some("1.0"),
        "x86.apk",
    );
}

#[test]
fn v1_unavailable_pin_returns_no_download() {
    assert!(parse(
        index(json!([v1("1.0", "old.apk", "x86")])),
        Some("missing"),
        None
    )
    .0
    .is_empty());
}

#[test]
fn malformed_index_envelopes_return_error() {
    for bad in [
        Value::Null,
        json!([]),
        json!({}),
        json!({"repo": null, "packages": {}}),
        json!({"repo": {"address": 1}, "packages": {}}),
        json!({"repo": {"address": REPO}, "packages": []}),
    ] {
        assert!(parse_json_for_download_information(
            bad,
            vec![(APP.to_owned(), None)],
            None,
            progress()
        )
        .is_err());
    }
}

#[test]
fn missing_or_wrong_shape_app_returns_no_download() {
    for package in [Value::Null, json!(42), json!("wrong")] {
        assert!(parse(index(package), None, None).0.is_empty());
    }
    assert!(parse(
        json!({"repo": {"address": REPO}, "packages": {}}),
        None,
        None
    )
    .0
    .is_empty());
}

#[test]
fn v2_latest_uses_highest_version_code_not_map_order() {
    let versions =
        json!({"a": v2(20, "2.0", "new.apk", "arm64-v8a"), "z": v2(10, "1.0", "old.apk", "x86")});
    assert_selected(
        parse(index(json!({"versions": versions})), None, None),
        None,
        "new.apk",
    );
}

#[test]
fn v2_pin_does_not_require_version_code() {
    let mut old = v2(10, "1.0", "old.apk", "x86");
    old["manifest"]
        .as_object_mut()
        .unwrap()
        .remove("versionCode");
    let versions = json!({"a": v2(20, "2.0", "new.apk", "arm64-v8a"), "z": old});
    assert_selected(
        parse(index(json!({"versions": versions})), Some("1.0"), None),
        Some("1.0"),
        "old.apk",
    );
}

#[test]
fn v2_latest_filters_abi_before_comparing_version_codes() {
    let versions =
        json!({"a": v2(30, "3.0", "x86.apk", "x86"), "z": v2(20, "2.0", "arm.apk", "arm64-v8a")});
    assert_selected(
        parse(
            index(json!({"versions": versions})),
            None,
            Some("arm64-v8a"),
        ),
        None,
        "arm.apk",
    );
}

#[test]
fn v2_pin_filters_abi_between_same_version_variants() {
    let versions =
        json!({"a": v2(20, "2.0", "x86.apk", "x86"), "z": v2(21, "2.0", "arm.apk", "arm64-v8a")});
    assert_selected(
        parse(
            index(json!({"versions": versions})),
            Some("2.0"),
            Some("arm64-v8a"),
        ),
        Some("2.0"),
        "arm.apk",
    );
}

#[test]
fn v2_no_candidates_never_synthesizes_an_empty_download() {
    for package in [
        json!({}),
        json!({"versions": {}}),
        json!({"versions": {"a": null, "b": {}, "c": {"manifest": {}, "file": {}}}}),
    ] {
        assert!(
            parse(index(package.clone()), None, None).0.is_empty(),
            "package: {package}"
        );
        assert!(parse(index(package), Some("missing"), None).0.is_empty());
    }
}

#[test]
fn v2_no_matching_abi_returns_no_download() {
    for nativecode in [
        Value::Null,
        json!([]),
        json!(["x86"]),
        json!("arm64-v8a"),
        json!([42]),
    ] {
        let mut version = v2(10, "1.0", "app.apk", "x86");
        version["manifest"]["nativecode"] = nativecode;
        for pin in [None, Some("1.0")] {
            assert!(parse(
                index(json!({"versions": {"a": version}})),
                pin,
                Some("arm64-v8a")
            )
            .0
            .is_empty());
        }
    }
}

#[test]
fn v1_requires_exactly_32_sha256_bytes_for_latest_and_pinned() {
    for hash in [
        "".to_owned(),
        "not hex".to_owned(),
        "a".to_owned(),
        "00".repeat(20),
        "ab".repeat(31),
        "ab".repeat(33),
    ] {
        let mut candidate = v1("1.0", "bad.apk", "x86");
        candidate["hash"] = json!(hash);
        for pin in [None, Some("1.0")] {
            assert!(
                parse(index(json!([candidate])), pin, None).0.is_empty(),
                "hash: {hash}"
            );
        }
    }
}

#[test]
fn v2_requires_exactly_32_sha256_bytes_for_latest_and_pinned() {
    for hash in [
        "".to_owned(),
        "not hex".to_owned(),
        "a".to_owned(),
        "00".repeat(20),
        "ab".repeat(31),
        "ab".repeat(33),
    ] {
        let mut candidate = v2(10, "1.0", "bad.apk", "x86");
        candidate["file"]["sha256"] = json!(hash);
        for pin in [None, Some("1.0")] {
            assert!(
                parse(index(json!({"versions": {"a": candidate}})), pin, None)
                    .0
                    .is_empty(),
                "hash: {hash}"
            );
        }
    }
}

#[test]
fn empty_apk_names_are_not_download_candidates_in_either_schema() {
    for package in [
        json!([v1("1.0", "", "x86")]),
        json!({"versions": {"a": v2(10, "1.0", "", "x86")}}),
    ] {
        for pin in [None, Some("1.0")] {
            assert!(parse(index(package.clone()), pin, None).0.is_empty());
        }
    }
}

#[test]
fn malformed_newer_hash_does_not_hide_usable_older_version() {
    let mut bad_v1 = v1("2.0", "bad.apk", "x86");
    bad_v1["hash"] = json!("invalid");
    assert_selected(
        parse(
            index(json!([bad_v1, v1("1.0", "old.apk", "x86")])),
            None,
            None,
        ),
        None,
        "old.apk",
    );
    let mut bad_v2 = v2(20, "2.0", "bad.apk", "x86");
    bad_v2["file"]["sha256"] = json!("invalid");
    assert_selected(
        parse(
            index(json!({"versions": {"a": bad_v2, "b": v2(10, "1.0", "old.apk", "x86")}})),
            None,
            None,
        ),
        None,
        "old.apk",
    );
}

async fn fixture_server(responses: Vec<Vec<u8>>) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut buffer = [0; 1024];
                let count = socket.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
                if request.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                assert!(request.len() < 8192);
            }
            requests.push(String::from_utf8(request).unwrap());
            let _ = socket.write_all(&response).await;
        }
        requests
    });
    (address, server)
}

fn http_ok(body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

#[tokio::test]
async fn reliability_nested_entry_index_is_verified_before_publication() {
    let (repo, server) = fixture_server(vec![http_ok(b"{}")]).await;
    let dir = tempdir().unwrap();
    let entry = json!({"index": {"name": "/metadata/index.json", "sha256": hex::encode(Sha256::digest(b"{}"))}}).to_string();
    let result = verify_and_return_index_from_entry(
        &dir,
        &repo,
        &entry,
        true,
        progress(),
        OutputFormat::Plaintext,
    )
    .await;
    assert_eq!(result.unwrap(), "{}");
    let requests = tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
    assert!(requests[0].starts_with("GET /metadata/index.json HTTP/1.1"));
    assert_eq!(
        fs::read(dir.path().join("metadata/index.json")).unwrap(),
        b"{}"
    );
}

fn apk_index(repo: &str, packages: &[(&str, &str)]) -> Value {
    let packages: serde_json::Map<String, Value> = packages
        .iter()
        .map(|(id, name)| {
            let mut candidate = v1("1.0", name, "x86");
            candidate["hash"] = json!(hex::encode(Sha256::digest(b"good")));
            (id.to_string(), json!([candidate]))
        })
        .collect();
    json!({"repo": {"address": repo}, "packages": packages})
}

#[tokio::test]
async fn reliability_all_download_jobs_run_even_after_checksum_failure() {
    let (repo, server) = fixture_server(vec![
        http_ok(b"wrong"),
        http_ok(b"wrong"),
        http_ok(b"wrong"),
        http_ok(b"good"),
    ])
    .await;
    let dir = tempdir().unwrap();
    let result = download_from_index(
        apk_index(&repo, &[("bad.app", "bad.apk"), ("good.app", "good.apk")]),
        vec![("bad.app".into(), None), ("good.app".into(), None)],
        1,
        0,
        dir.path(),
        None,
        progress(),
    )
    .await;
    assert!(!result);
    let requests = tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(requests.len(), 4);
    assert!(!dir.path().join("bad.app.apk").exists());
    assert_eq!(fs::read(dir.path().join("good.app.apk")).unwrap(), b"good");
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        1,
        "no partial files may remain"
    );
}

#[tokio::test]
async fn reliability_missing_selection_still_downloads_valid_job_but_returns_false() {
    let (repo, server) = fixture_server(vec![http_ok(b"good")]).await;
    let dir = tempdir().unwrap();
    assert!(
        !download_from_index(
            apk_index(&repo, &[(APP, "app.apk")]),
            vec![("missing.app".into(), None), (APP.into(), None)],
            0,
            0,
            dir.path(),
            None,
            progress()
        )
        .await
    );
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        fs::read(dir.path().join(format!("{APP}.apk"))).unwrap(),
        b"good"
    );
}

#[tokio::test]
async fn reliability_successful_download_and_verified_existing_skip_are_true() {
    let (repo, server) = fixture_server(vec![http_ok(b"good")]).await;
    let dir = tempdir().unwrap();
    let index = apk_index(&repo, &[(APP, "app.apk")]);
    assert!(
        download_from_index(
            index.clone(),
            vec![(APP.into(), None)],
            1,
            0,
            dir.path(),
            None,
            progress()
        )
        .await
    );
    server.await.unwrap();
    // Listener is closed: a successful result now must be a checked local skip.
    assert!(
        download_from_index(
            index.clone(),
            vec![(APP.into(), None)],
            1,
            0,
            dir.path(),
            None,
            progress()
        )
        .await
    );
    fs::write(dir.path().join(format!("{APP}.apk")), b"bad").unwrap();
    assert!(
        !download_from_index(
            index,
            vec![(APP.into(), None)],
            1,
            0,
            dir.path(),
            None,
            progress()
        )
        .await
    );
    assert_eq!(
        fs::read(dir.path().join(format!("{APP}.apk"))).unwrap(),
        b"bad"
    );
}

#[tokio::test]
async fn reliability_http_errors_and_short_bodies_never_publish_an_apk() {
    for response in [
        b"HTTP/1.1 503 Unavailable\r\nContent-Length: 4\r\nConnection: close\r\n\r\ngood".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\nConnection: close\r\n\r\ngood".to_vec(),
    ] {
        let (repo, server) =
            fixture_server(vec![response.clone(), response.clone(), response]).await;
        let dir = tempdir().unwrap();
        assert!(
            !download_from_index(
                apk_index(&repo, &[(APP, "app.apk")]),
                vec![(APP.into(), None)],
                1,
                0,
                dir.path(),
                None,
                progress()
            )
            .await
        );
        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn reliability_symlink_and_nonregular_destinations_are_failures_without_http() {
    use std::os::unix::fs::symlink;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let repo = format!("http://{}", listener.local_addr().unwrap());
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("target"), b"good").unwrap();
    for kind in ["symlink", "dangling", "directory"] {
        let dir = tempdir().unwrap();
        let target = dir.path().join(format!("{APP}.apk"));
        match kind {
            "symlink" => symlink(outside.path().join("target"), &target).unwrap(),
            "dangling" => symlink(outside.path().join("missing"), &target).unwrap(),
            _ => fs::create_dir(&target).unwrap(),
        }
        assert!(
            !download_from_index(
                apk_index(&repo, &[(APP, "app.apk")]),
                vec![(APP.into(), None)],
                1,
                0,
                dir.path(),
                None,
                progress()
            )
            .await
        );
    }
    let link_dir = outside.path().join("linked-directory");
    symlink(outside.path(), &link_dir).unwrap();
    assert!(validate_output_directory(&link_dir).is_err());
    assert!(validate_output_directory(&outside.path().join("..")).is_err());
    assert!(matches!(listener.accept(), Err(err) if err.kind() == io::ErrorKind::WouldBlock));
    assert_eq!(fs::read(outside.path().join("target")).unwrap(), b"good");
}

#[tokio::test]
async fn reliability_empty_public_jobs_and_malformed_entry_return_false_or_error() {
    let dir = tempdir().unwrap();
    assert!(!download_apps(vec![], 1, 0, dir.path(), HashMap::new()).await);
    assert!(!list_versions(vec![], HashMap::new()).await);
    assert!(verify_and_return_index_from_entry(
        &dir,
        "http://127.0.0.1:9",
        "{invalid",
        true,
        progress(),
        OutputFormat::Plaintext
    )
    .await
    .is_err());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn reliability_list_valid_v1_and_v2_and_invalid_user_components() {
    for package in [
        json!([v1("1.0", "app.apk", "x86")]),
        json!({"versions": {"a": v2(1, "1.0", "/nested/app.apk", "x86")}}),
    ] {
        assert!(parse_json_display_versions(
            index(package.clone()),
            vec![(APP.into(), None)],
            OutputFormat::Json
        )
        .is_ok());
        for (id, version, arch) in [
            ("../escape", None, None),
            (APP, Some("../bad"), None),
            (APP, None, Some("../x86")),
            ("", None, None),
            (APP, Some(""), None),
        ] {
            let parsed = parse_json_for_download_information(
                index(package.clone()),
                vec![(id.into(), version.map(str::to_owned))],
                arch.map(str::to_owned),
                progress(),
            )
            .unwrap();
            assert!(parsed.0.is_empty());
        }
    }
}

#[tokio::test]
async fn reliability_metadata_status_size_and_deadline_fail_without_publication() {
    for response in [
        b"HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n".to_vec(),
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n8\r\ntoolarge\r\n0\r\n\r\n".to_vec(),
    ] {
        let (repo, server) = fixture_server(vec![response]).await;
        let dir = tempdir().unwrap();
        assert!(download_metadata(&format!("{repo}/index.json"), dir.path(), "index.json", None, 4, Duration::from_secs(2)).await.is_err());
        server.await.unwrap();
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/index.json", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{")
            .await
            .unwrap();
        std::future::pending::<()>().await;
    });
    let dir = tempdir().unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        download_metadata(
            &url,
            dir.path(),
            "index.json",
            None,
            100,
            Duration::from_millis(50),
        ),
    )
    .await
    .expect("internal deadline must fire");
    assert!(result.is_err());
    server.abort();
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn reliability_archive_http_errors_and_missing_etag_do_not_panic() {
    let (repo, server) = fixture_server(vec![
        b"HTTP/1.1 500 Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
    ])
    .await;
    let dir = tempdir().unwrap();
    assert!(download_repository_archive(&dir, &repo, true, None)
        .await
        .is_err());
    server.await.unwrap();
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    let (repo, server) = fixture_server(vec![http_ok(b"not a ZIP")]).await;
    assert!(download_repository_archive(&dir, &repo, true, None)
        .await
        .unwrap()
        .is_none());
    server.await.unwrap();
    assert!(extract_archive(&dir, &dir.path().join("jar.zip")).is_err());
}

#[cfg(unix)]
#[test]
fn reliability_cache_and_bounded_read_reject_symlinks_and_corruption() {
    use std::os::unix::fs::symlink;
    let dir = tempdir().unwrap();
    let real = dir.path().join("real");
    let link = dir.path().join("link");
    fs::write(&real, "original").unwrap();
    symlink(&real, &link).unwrap();
    assert!(regular_cache_exists(&link).is_err());
    assert!(read_bounded_string(&link, 100).is_err());
    assert!(write_cache_file(&link, b"overwrite").is_err());
    assert_eq!(fs::read(&real).unwrap(), b"original");
    assert!(read_bounded_string(&real, 2).is_err());
    fs::write(&real, [0xff]).unwrap();
    assert!(read_bounded_string(&real, 100).is_err());
    let absent = dir.path().join("absent");
    assert!(!regular_cache_exists(&absent).unwrap());
    write_cache_file(&absent, b"one").unwrap();
    write_cache_file(&absent, b"two").unwrap();
    assert_eq!(fs::read(&absent).unwrap(), b"two");
}

fn zip_fixture(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
    for (name, contents) in entries {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn reliability_archive_extracts_signed_fixture_and_rejects_traversal_and_collisions() {
    let (fixture, files, pin) = signed_fixture();
    let names = [
        "META-INF/FIXTURE.RSA",
        "META-INF/FIXTURE.SF",
        "META-INF/MANIFEST.MF",
        "entry.json",
        "index-v1.json",
    ];
    let contents: Vec<Vec<u8>> = names
        .iter()
        .map(|name| fs::read(fixture.path().join(name)).unwrap())
        .collect();
    let entries: Vec<_> = names
        .iter()
        .zip(contents.iter())
        .map(|(name, content)| (*name, content.as_slice()))
        .collect();
    let dir = tempdir().unwrap();
    let jar = dir.path().join("jar.zip");
    fs::write(&jar, zip_fixture(&entries)).unwrap();
    let extracted = extract_archive(&dir, &jar).unwrap();
    assert!(extracted.contains(&files[0]));
    assert_eq!(
        verify_and_return_json(&dir, &extracted, &pin, true, true, progress()).unwrap(),
        "{}"
    );
    for entries in [
        vec![("../escape", b"x".as_slice())],
        vec![
            ("parent", b"x".as_slice()),
            ("parent/child", b"y".as_slice()),
        ],
    ] {
        let dir = tempdir().unwrap();
        let jar = dir.path().join("jar.zip");
        fs::write(&jar, zip_fixture(&entries)).unwrap();
        assert!(extract_archive(&dir, &jar).is_err());
    }
}

#[test]
fn reliability_repository_options_reject_bad_urls_and_pin_lengths() {
    for repo in [
        "not a URL",
        "file:///tmp/repo",
        "https://user:password@example.com",
        "https://example.com/repo#fragment",
        "https://example.com/repo?query=1",
        "https://example.com/repo?fingerprint=zz",
        "https://example.com/repo?fingerprint=ab",
    ] {
        assert!(
            repository_options(&HashMap::from([("repo", repo)])).is_err(),
            "{repo}"
        );
    }
    let repo = format!("https://example.com/repo?fingerprint={}", "ab".repeat(32));
    let (_, pin, entry, verify) = repository_options(&HashMap::from([
        ("repo", repo.as_str()),
        ("use_entry", "false"),
    ]))
    .unwrap();
    assert_eq!(pin, vec![0xab; 32]);
    assert!(!entry);
    assert!(verify);
}

#[tokio::test]
async fn reliability_signed_cache_rechecks_pin_and_never_uses_legacy_json() {
    let (fixture, _, _) = signed_fixture();
    let names = [
        "META-INF/FIXTURE.RSA",
        "META-INF/FIXTURE.SF",
        "META-INF/MANIFEST.MF",
        "entry.json",
        "index-v1.json",
    ];
    let contents: Vec<Vec<u8>> = names
        .iter()
        .map(|name| fs::read(fixture.path().join(name)).unwrap())
        .collect();
    let entries: Vec<_> = names
        .iter()
        .zip(contents.iter())
        .map(|(name, content)| (*name, content.as_slice()))
        .collect();
    let jar = zip_fixture(&entries);
    let mut first = format!(
        "HTTP/1.1 200 OK\r\nETag: \"fixture\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        jar.len()
    )
    .into_bytes();
    first.extend_from_slice(&jar);
    let not_modified = b"HTTP/1.1 304 Not Modified\r\nConnection: close\r\n\r\n".to_vec();
    let (repo, server) = fixture_server(vec![
        first,
        not_modified.clone(),
        not_modified.clone(),
        not_modified,
    ])
    .await;
    let pinned_repo = format!("{repo}?fingerprint={FINGERPRINT}");
    let options = HashMap::from([("repo", pinned_repo.as_str()), ("use_entry", "false")]);
    let cache = tempdir().unwrap();
    fs::write(cache.path().join("index_v1.json"), "{\"unsigned\":true}").unwrap();
    fs::write(cache.path().join("latest_etag"), "\"fixture\"").unwrap();
    for _ in 0..2 {
        assert_eq!(
            retrieve_index_using_cache(&options, progress(), OutputFormat::Plaintext, cache.path())
                .await
                .unwrap(),
            json!({})
        );
    }
    let bad_pin = format!("{repo}?fingerprint={}", "00".repeat(32));
    let bad_options = HashMap::from([("repo", bad_pin.as_str()), ("use_entry", "false")]);
    assert!(retrieve_index_using_cache(
        &bad_options,
        progress(),
        OutputFormat::Plaintext,
        cache.path()
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("Fingerprint"));
    let cache_name = hex::encode(Sha256::digest(format!("{repo}:false")));
    fs::write(
        cache
            .path()
            .join("fdroid-signed-archives")
            .join(cache_name)
            .with_extension("jar"),
        b"corrupt cache",
    )
    .unwrap();
    assert!(retrieve_index_using_cache(
        &options,
        progress(),
        OutputFormat::Plaintext,
        cache.path()
    )
    .await
    .is_err());
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(!requests[0].to_ascii_lowercase().contains("if-none-match"));
    for request in &requests[1..] {
        assert!(request
            .to_ascii_lowercase()
            .contains("if-none-match: \"fixture\""));
    }
}

// Run the real public entry points/private APK helper in a fresh process so
// libtest's in-process output capture cannot hide stderr/JSON disclosures.
#[tokio::test]
async fn disclosure_child() {
    let Ok(mode) = std::env::var("FDROID_DISCLOSURE_CHILD") else {
        return;
    };
    let repo = std::env::var("FDROID_DISCLOSURE_REPO").unwrap();
    let out = PathBuf::from(std::env::var_os("FDROID_DISCLOSURE_OUT").unwrap());
    let use_entry = std::env::var("FDROID_DISCLOSURE_ENTRY").unwrap();
    let options = HashMap::from([
        ("repo", repo.as_str()),
        ("use_entry", use_entry.as_str()),
        // Only unsigned local fixtures use this bypass, never a production default.
        ("verify-index", "false"),
        (
            "output_format",
            if mode == "json" { "json" } else { "plaintext" },
        ),
    ]);
    let ok = match mode.as_str() {
        "controls-plaintext" | "controls-json" => {
            let version = "1.0\u{1b}]0;FDROID_REMOTE_MARKER\u{7}\r\n\t\u{85}";
            let package = if use_entry == "true" {
                json!([v1(version, "app.apk", "x86")])
            } else {
                json!({"versions": {"a": v2(1, version, "app.apk", "x86")}})
            };
            parse_json_display_versions(
                index(package),
                vec![(APP.into(), None)],
                if mode == "controls-json" {
                    OutputFormat::Json
                } else {
                    OutputFormat::Plaintext
                },
            )
            .is_ok()
        }
        "artifact" | "mixed" => {
            let mut packages = vec![(APP, "app.apk")];
            let mut apps = vec![(APP.into(), None)];
            if mode == "mixed" {
                packages.push(("good.app", "good.apk"));
                apps.push(("good.app".into(), None));
            }
            download_from_index(
                apk_index(&repo, &packages),
                apps,
                1,
                0,
                &out,
                None,
                progress(),
            )
            .await
        }
        "download" => download_apps(vec![(APP.into(), None)], 1, 0, &out, options).await,
        _ => list_versions(vec![(APP.into(), None)], options).await,
    };
    io::stdout().flush().unwrap();
    io::stderr().flush().unwrap();
    std::process::exit(if ok { 0 } else { 1 });
}

const DISCLOSURE_MARKER: &str = "FDROID_REDIRECT_FAKE_SECRET";

fn signed_url_failure_responses() -> Vec<Vec<u8>> {
    vec![
        format!("HTTP/1.1 302 Found\r\nLocation: /secret?token={DISCLOSURE_MARKER}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").into_bytes(),
        format!("HTTP/1.1 403 Forbidden\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{DISCLOSURE_MARKER}", DISCLOSURE_MARKER.len()).into_bytes(),
    ]
}

async fn captured_disclosure_child(
    mode: &str,
    repo: &str,
    entry: bool,
    dir: &TempDir,
) -> std::process::Output {
    tokio::time::timeout(
        Duration::from_secs(10),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!(
                    "{}::disclosure_child",
                    module_path!().split_once("::").unwrap().1
                ),
                "--nocapture",
            ])
            .env("FDROID_DISCLOSURE_CHILD", mode)
            .env("FDROID_DISCLOSURE_REPO", repo)
            .env(
                "FDROID_DISCLOSURE_ENTRY",
                if entry { "true" } else { "false" },
            )
            .env("FDROID_DISCLOSURE_OUT", dir.path().join("out"))
            .env("XDG_CONFIG_HOME", dir.path().join("config"))
            .env("NO_PROXY", "127.0.0.1,localhost")
            .env("no_proxy", "127.0.0.1,localhost")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("child deadline")
    .unwrap()
}

fn assert_safe_disclosure_failure(output: &std::process::Output, json_output: bool) {
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(!stderr.is_empty(), "failure must have a diagnostic");
    for forbidden in [DISCLOSURE_MARKER, "token=", "http://", "/secret?"] {
        assert!(
            !stdout.contains(forbidden),
            "stdout disclosed {forbidden}: {stdout}"
        );
        assert!(
            !stderr.contains(forbidden),
            "stderr disclosed {forbidden}: {stderr}"
        );
    }
    if json_output {
        let results: Vec<_> = stdout
            .lines()
            .filter(|line| line.starts_with('{'))
            .collect();
        assert_eq!(results.len(), 1, "one JSON result: {stdout}");
        let error: Value = serde_json::from_str(results[0]).unwrap();
        assert!(error["error"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
    }
}

#[tokio::test]
async fn disclosure_archive_redirect_is_safe_in_download_plaintext_and_json() {
    for entry in [true, false] {
        for mode in ["download", "plaintext", "json"] {
            let (repo, server) = fixture_server(signed_url_failure_responses()).await;
            let dir = tempdir().unwrap();
            fs::create_dir(dir.path().join("out")).unwrap();
            let output = captured_disclosure_child(mode, &repo, entry, &dir).await;
            let requests = server.await.unwrap();
            assert_eq!(requests.len(), 2);
            assert!(requests[0].contains(if entry {
                "/entry.jar "
            } else {
                "/index-v1.jar "
            }));
            assert!(requests[1].contains(DISCLOSURE_MARKER));
            assert_eq!(fs::read_dir(dir.path().join("out")).unwrap().count(), 0);
            assert_safe_disclosure_failure(&output, mode == "json");
        }
    }
}

fn unsigned_entry_archive() -> Vec<u8> {
    let entry = json!({"index": {"name": "index-v2.json", "sha256": "00".repeat(32)}}).to_string();
    zip_fixture(&[
        ("META-INF/FIXTURE.RSA", b"fixture"),
        ("META-INF/FIXTURE.SF", SIGNED_FILE.as_bytes()),
        ("META-INF/MANIFEST.MF", MANIFEST.as_bytes()),
        ("entry.json", entry.as_bytes()),
    ])
}

#[tokio::test]
async fn disclosure_entry_index_redirect_is_safe_in_download_plaintext_and_json() {
    for mode in ["download", "plaintext", "json"] {
        let mut responses = vec![http_ok(&unsigned_entry_archive())];
        responses.extend(signed_url_failure_responses());
        let (repo, server) = fixture_server(responses).await;
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("out")).unwrap();
        let output = captured_disclosure_child(mode, &repo, true, &dir).await;
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[1].starts_with("GET /index-v2.json "));
        assert!(requests[2].contains(DISCLOSURE_MARKER));
        assert_eq!(fs::read_dir(dir.path().join("out")).unwrap().count(), 0);
        assert_safe_disclosure_failure(&output, mode == "json");
    }
}

#[tokio::test]
async fn disclosure_apk_redirect_retries_are_safe_and_mixed_success_is_retained() {
    for mixed in [false, true] {
        let mut responses = Vec::new();
        for _ in 0..3 {
            responses.extend(signed_url_failure_responses());
        }
        if mixed {
            responses.push(http_ok(b"good"));
        }
        let (repo, server) = fixture_server(responses).await;
        let dir = tempdir().unwrap();
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let output =
            captured_disclosure_child(if mixed { "mixed" } else { "artifact" }, &repo, true, &dir)
                .await;
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), if mixed { 7 } else { 6 });
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.contains(DISCLOSURE_MARKER))
                .count(),
            3
        );
        assert_eq!(fs::read_dir(&out).unwrap().count(), usize::from(mixed));
        assert!(!out.join(format!("{APP}.apk")).exists());
        if mixed {
            assert_eq!(fs::read(out.join("good.app.apk")).unwrap(), b"good");
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        for attempt in 1..=3 {
            assert!(
                stderr.contains(&format!("attempt {attempt}/3: HTTP status 403")),
                "{stderr}"
            );
        }
        assert_safe_disclosure_failure(&output, false);
    }
}

#[tokio::test]
async fn disclosure_cached_archive_errors_are_safe_in_all_user_channels() {
    for mode in ["download", "plaintext", "json"] {
        let (repo, server) = fixture_server(vec![
            b"HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        ])
        .await;
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("out")).unwrap();
        let cache = dir.path().join("config/apkeep/fdroid-signed-archives");
        fs::create_dir_all(&cache).unwrap();
        let cache_name = hex::encode(Sha256::digest(format!("{repo}:false")));
        fs::write(
            cache.join(&cache_name).with_extension("jar"),
            format!("http://secret.invalid/?token={DISCLOSURE_MARKER}"),
        )
        .unwrap();
        fs::write(cache.join(cache_name).with_extension("etag"), "\"fixture\"").unwrap();
        let output = captured_disclosure_child(mode, &repo, false, &dir).await;
        let requests = tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("if-none-match: \"fixture\""));
        assert_eq!(fs::read_dir(dir.path().join("out")).unwrap().count(), 0);
        assert_safe_disclosure_failure(&output, mode == "json");
        assert!(String::from_utf8_lossy(&output.stderr).contains("invalid repository archive"));
    }
}

#[tokio::test]
async fn remote_version_controls_are_escaped_only_in_plaintext_for_both_schemas() {
    let original = "1.0\u{1b}]0;FDROID_REMOTE_MARKER\u{7}\r\n\t\u{85}";
    for entry in [true, false] {
        for mode in ["controls-plaintext", "controls-json"] {
            let dir = tempdir().unwrap();
            let output = captured_disclosure_child(mode, "http://127.0.0.1:9", entry, &dir).await;
            assert_eq!(output.status.code(), Some(0));
            assert!(output.stderr.is_empty());
            let stdout = String::from_utf8(output.stdout).unwrap();
            if mode == "controls-plaintext" {
                assert!(
                    !stdout.chars().any(|ch| ch.is_control() && ch != '\n'),
                    "unescaped control: {stdout:?}"
                );
            } else {
                assert!(
                    !stdout.contains(['\u{1b}', '\u{7}', '\r', '\t']),
                    "JSON must retain serde escaping"
                );
            }
            if mode == "controls-json" {
                let value: Value = serde_json::from_str(
                    stdout.lines().find(|line| line.starts_with('{')).unwrap(),
                )
                .unwrap();
                assert_eq!(
                    value["apps"][APP]["available_versions"][0]["version"],
                    original
                );
            } else {
                assert!(
                    stdout.contains("1.0\\u{1b}]0;FDROID_REMOTE_MARKER\\u{7}\\r\\n\\t\\u{85}"),
                    "{stdout:?}"
                );
            }
        }
    }
}

#[test]
fn safe_error_categories_never_format_opaque_errors_or_their_sources() {
    #[derive(Debug)]
    struct Opaque;
    impl std::fmt::Display for Opaque {
        fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            panic!("opaque error must never be formatted")
        }
    }
    impl Error for Opaque {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            panic!("do not traverse untrusted chains")
        }
    }
    assert_eq!(
        safe_error_summary(&Opaque),
        "repository data validation or verification failed"
    );
    let io = io::Error::other(format!("http://secret.invalid/?token={DISCLOSURE_MARKER}"));
    assert_eq!(safe_error_summary(&io), "filesystem or transfer I/O failed");
    let json = serde_json::from_str::<u32>(&format!(
        "\"http://secret.invalid/?token={DISCLOSURE_MARKER}\""
    ))
    .unwrap_err();
    assert_eq!(safe_error_summary(&json), "invalid repository JSON");
    let zip = zip::ZipArchive::new(io::Cursor::new(b"invalid archive")).unwrap_err();
    assert_eq!(safe_error_summary(&zip), "invalid repository archive");
}

#[test]
fn v1_latest_no_match_returns_no_download_without_panic() {
    // Every case used to reach app_version.unwrap() with None.
    for packages in [
        json!([]),
        json!([null, 42, {}]),
        json!([v1("1.0", "x86.apk", "x86")]),
    ] {
        assert!(parse(index(packages), None, Some("arm64-v8a")).0.is_empty());
    }
}
