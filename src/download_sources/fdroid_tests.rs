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
            .contains("does not match the actual index sha256sum"));
        assert!(dir.path().join("index-v2.json").is_file());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
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
                .contains("does not match the actual index sha256sum"));
        }
        assert_eq!(fs::read(dir.path().join("index-v2.json")).unwrap(), b"{}");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
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
