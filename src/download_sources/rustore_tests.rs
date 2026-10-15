use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn truncated_http_body_reports_a_redacted_read_error() {
    let client = local_client();
    let config = RustoreOptions::parse(&HashMap::new(), None).unwrap();
    for lookup in [true, false] {
        let (endpoint, task) = serve_raw_response("HTTP/1.1 200 OK\r\nContent-Length: 500\r\nConnection: close\r\n\r\nsecret-truncated-body".to_string()).await;
        let error = if lookup {
            get_app_id_at(&client, &config.headers, "com.example.app", &endpoint)
                .await
                .unwrap_err()
        } else {
            get_download_url_at(&client, &config, 42, &endpoint)
                .await
                .unwrap_err()
        };
        task.await.unwrap();
        assert!(
            error.contains("network error while reading response"),
            "{error}"
        );
        assert!(!error.contains("secret-"));
    }
}

#[tokio::test]
async fn multiple_download_urls_are_never_saved_as_a_complete_apk() {
    let client = local_client();
    for without_splits in ["false", "true"] {
        let config =
            RustoreOptions::parse(&HashMap::from([("without_splits", without_splits)]), None)
                .unwrap();
        let body = r#"{"code":"OK","body":{"downloadUrls":[{"url":"https://signed.test/base.apk?secret-token"},{"url":"https://signed.test/split.apk?secret-token"}]}}"#;
        let (endpoint, task) = serve_once("200 OK", body).await;
        let result = get_download_url_at(&client, &config, 42, &endpoint).await;
        let request = task.await.unwrap();
        let sent: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(sent["withoutSplits"], without_splits == "true");
        assert!(
            result.is_err(),
            "silently chose first URL from split APK set"
        );
        let error = result.unwrap_err();
        assert!(error.contains("split APKs are not supported"), "{error}");
        assert!(!error.contains("secret-token"));
        assert!(!error.contains("signed.test"));
    }
}

#[tokio::test]
async fn configured_device_and_headers_are_sent_over_http() {
    let options = HashMap::from([
        ("arch", "x86;x86_64"),
        ("sdk_version", "34"),
        ("screen_density", "480"),
        ("locales", "ru_RU;en_US"),
        ("mobile_services", "HMS"),
        ("without_splits", "false"),
        ("rustore_version", "22000"),
        ("user_agent", "Custom/2"),
    ]);
    let config = RustoreOptions::parse(&options, Some("19000")).unwrap();
    let client = local_client();
    let (endpoint, task) = serve_once("200 OK", r#"[{"id":73}]"#).await;
    assert_eq!(
        get_app_id_at(&client, &config.headers, "com.example.app", &endpoint)
            .await
            .unwrap(),
        73
    );
    let lookup = task.await.unwrap().to_lowercase();
    assert!(lookup.contains("rustorevercode: 22000\r\n"));
    assert!(lookup.contains("user-agent: custom/2\r\n"));
    let (endpoint, task) = serve_once(
        "200 OK",
        r#"{"code":"OK","body":{"downloadUrls":[{"url":"https://example.test/app.apk"}]}}"#,
    )
    .await;
    assert_eq!(
        get_download_url_at(&client, &config, 73, &endpoint)
            .await
            .unwrap(),
        "https://example.test/app.apk"
    );
    let request = task.await.unwrap();
    let (headers, body) = request.split_once("\r\n\r\n").unwrap();
    assert!(headers.to_lowercase().contains("rustorevercode: 22000\r\n"));
    assert!(headers.to_lowercase().contains("user-agent: custom/2\r\n"));
    assert!(headers
        .to_lowercase()
        .contains("content-type: application/json\r\n"));
    let body: Value = serde_json::from_str(body).unwrap();
    assert_eq!(
        body,
        json!({
            "appId": 73, "firstInstall": true, "mobileServices": ["HMS"],
            "supportedAbis": ["x86", "x86_64"], "screenDensity": 480,
            "supportedLocales": ["ru_RU", "en_US"], "sdkVersion": 34,
            "withoutSplits": false, "signatureFingerprint": null
        })
    );
}

#[tokio::test]
async fn download_response_requires_a_nonempty_http_url() {
    let client = local_client();
    let config = RustoreOptions::parse(&HashMap::new(), None).unwrap();
    for body in [
        "",
        "secret-json-error",
        r#"{"code":"OK"}"#,
        r#"{"code":"OK","body":null}"#,
        r#"{"code":"OK","body":{"downloadUrls":null}}"#,
        r#"{"code":"OK","body":{"downloadUrls":{}}}"#,
        r#"{"code":"OK","body":{"downloadUrls":[]}}"#,
        r#"{"code":"OK","body":{"downloadUrls":[{}]}}"#,
        r#"{"code":"OK","body":{"downloadUrls":[{"url":42}]}}"#,
        r#"{"code":"OK","body":{"downloadUrls":[{"url":""}]}}"#,
        r#"{"code":"OK","body":{"downloadUrls":[{"url":" "}]}}"#,
        r#"{"code":"OK","body":{"downloadUrls":[{"url":"secret-invalid-url"}]}}"#,
        r#"{"code":"OK","body":{"downloadUrls":[{"url":"file:///secret-path"}]}}"#,
        r#"{"code":"OK","body":{"downloadUrls":[{"url":"https://"}]}}"#,
    ] {
        let (endpoint, task) = serve_once("200 OK", body).await;
        let result = get_download_url_at(&client, &config, 42, &endpoint).await;
        task.await.unwrap();
        assert!(result.is_err(), "accepted malformed download response");
        let error = result.unwrap_err();
        assert!(error.contains("malformed response"), "{error}");
        assert!(!error.contains("secret-"));
    }
}

#[tokio::test]
async fn download_response_code_is_validated_and_redacted() {
    let client = local_client();
    let config = RustoreOptions::parse(&HashMap::new(), None).unwrap();
    for (body, expected) in [
        ("{}", "malformed response"),
        ("null", "malformed response"),
        (r#"{"code":null}"#, "malformed response"),
        (r#"{"code":42}"#, "malformed response"),
        (r#"{"code":""}"#, "malformed response"),
        (
            r#"{"code":"DENIED","body":"secret-api-body"}"#,
            "API rejected",
        ),
        (
            r#"{"code":"https://signed.test/?token=secret-api-code"}"#,
            "API rejected",
        ),
    ] {
        let (endpoint, task) = serve_once("200 OK", body).await;
        let error = get_download_url_at(&client, &config, 42, &endpoint)
            .await
            .unwrap_err();
        task.await.unwrap();
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("secret-"));
        assert!(!error.contains("signed.test"));
    }
}

#[tokio::test]
async fn lookup_response_errors_distinguish_not_found_from_malformed() {
    let client = local_client();
    let config = RustoreOptions::parse(&HashMap::new(), None).unwrap();
    for (body, expected) in [
        ("[]", "not found"),
        ("", "malformed response"),
        ("secret-json-error", "malformed response"),
        ("null", "malformed response"),
        ("{}", "malformed response"),
        ("[{}]", "malformed response"),
        ("[null]", "malformed response"),
        (r#"[{"id":0}]"#, "malformed response"),
        (r#"[{"id":-1}]"#, "malformed response"),
        (r#"[{"id":1.5}]"#, "malformed response"),
        (r#"[{"id":"secret-id"}]"#, "malformed response"),
    ] {
        let (endpoint, task) = serve_once("200 OK", body).await;
        let result = get_app_id_at(&client, &config.headers, "com.example.app", &endpoint).await;
        task.await.unwrap();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("secret-"));
    }
}

#[tokio::test]
async fn network_failures_are_distinct_and_redacted() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    // Keep the port bound without accepting: both requests deterministically time out.
    let endpoint = format!(
        "http://{}/secret-endpoint?token=secret-token",
        listener.local_addr().unwrap()
    );
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(TokioDuration::from_millis(50))
        .build()
        .unwrap();
    let config = RustoreOptions::parse(&HashMap::new(), None).unwrap();
    let lookup = get_app_id_at(&client, &config.headers, "com.example.app", &endpoint)
        .await
        .unwrap_err();
    let download = get_download_url_at(&client, &config, 42, &endpoint)
        .await
        .unwrap_err();
    for error in [lookup, download] {
        assert!(error.contains("network error"), "{error}");
        assert!(!error.contains("secret-"));
        assert!(!error.contains("127.0.0.1"));
    }
}

#[tokio::test]
async fn http_failures_report_status_without_leaking_body() {
    let client = local_client();
    let config = RustoreOptions::parse(&HashMap::new(), None).unwrap();
    for status in [
        "400 Bad Request",
        "403 Forbidden",
        "404 Not Found",
        "429 Too Many Requests",
        "503 Service Unavailable",
    ] {
        let code = status.split_once(' ').unwrap().0;
        for lookup in [true, false] {
            let (endpoint, task) =
                serve_once(status, "https://signed.test/app?token=secret-body").await;
            let result = if lookup {
                format!(
                    "{:?}",
                    get_app_id_at(&client, &config.headers, "com.example.app", &endpoint).await
                )
            } else {
                format!(
                    "{:?}",
                    get_download_url_at(&client, &config, 42, &endpoint).await
                )
            };
            task.await.unwrap();
            assert!(result.contains(&format!("HTTP {code}")), "{result}");
            assert!(!result.contains("secret-body"));
            assert!(!result.contains("signed.test"));
        }
    }
}

async fn serve_once(status: &str, body: &str) -> (String, tokio::task::JoinHandle<String>) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    serve_raw_response(response).await
}

async fn serve_raw_response(response: String) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/api", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        tokio::time::timeout(TokioDuration::from_secs(5), async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let read = stream.read(&mut buffer).await.unwrap();
                assert!(read > 0, "request closed before complete body");
                request.extend_from_slice(&buffer[..read]);
                if let Some(header_end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= header_end + 4 + length {
                        break;
                    }
                }
            }
            stream.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8(request).unwrap()
        })
        .await
        .expect("loopback request timed out")
    });
    (endpoint, task)
}

fn local_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(TokioDuration::from_secs(5))
        .build()
        .unwrap()
}

#[tokio::test]
async fn characterizes_lookup_http_responses() {
    let client = local_client();
    let config = RustoreOptions::parse(&HashMap::new(), None).unwrap();
    for (status, body, expected) in [
        ("200 OK", "[{\"id\":42}]", Some(42)),
        ("200 OK", "[]", None),
        ("200 OK", "{}", None),
        ("200 OK", "[{\"id\":\"42\"}]", None),
        ("200 OK", "not JSON", None),
        ("404 Not Found", "secret", None),
    ] {
        let (endpoint, task) = serve_once(status, body).await;
        assert_eq!(
            get_app_id_at(&client, &config.headers, "com.example.app", &endpoint)
                .await
                .ok(),
            expected
        );
        let request = task.await.unwrap();
        assert!(request.starts_with("GET /api?packageNames=com.example.app HTTP/1.1\r\n"));
        assert!(request.to_lowercase().contains("rustorevercode: 11500\r\n"));
    }
}

#[tokio::test]
async fn characterizes_download_http_responses() {
    let client = local_client();
    let config = RustoreOptions::parse(&HashMap::new(), None).unwrap();
    for (status, body, expected) in [
        (
            "200 OK",
            r#"{"code":"OK","body":{"downloadUrls":[{"url":"https://example.test/app.apk?token=secret"}]}}"#,
            Some("https://example.test/app.apk?token=secret"),
        ),
        ("200 OK", r#"{"code":"DENIED"}"#, None),
        (
            "200 OK",
            r#"{"code":"OK","body":{"downloadUrls":[]}}"#,
            None,
        ),
        ("200 OK", r#"{"code":"OK"}"#, None),
        ("200 OK", "not JSON", None),
        ("400 Bad Request", "secret", None),
    ] {
        let (endpoint, task) = serve_once(status, body).await;
        assert_eq!(
            get_download_url_at(&client, &config, 42, &endpoint)
                .await
                .ok()
                .as_deref(),
            expected
        );
        let request = task.await.unwrap();
        assert!(request.starts_with("POST /api HTTP/1.1\r\n"));
        assert!(request.to_lowercase().contains("rustorevercode: 11500\r\n"));
        let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["appId"], 42);
        assert_eq!(body["withoutSplits"], true);
    }
}

#[test]
fn invalid_device_options_are_rejected_without_echoing_values() {
    for key in ["sdk_version", "screen_density"] {
        for value in [
            "0",
            "-1",
            "+1",
            "4294967296",
            "",
            "1.5",
            "secret-device-value",
        ] {
            let result = RustoreOptions::parse(&HashMap::from([(key, value)]), None);
            assert!(result.is_err(), "accepted invalid {key}");
            let error = result.err().unwrap();
            assert!(error.contains(key));
            assert!(!error.contains("secret-device-value"));
        }
    }
    for key in ["arch", "locales", "mobile_services"] {
        for value in [
            "",
            ";",
            "x86;",
            ";x86",
            "x86;;arm64-v8a",
            " ",
            "secret-list-value\r\n",
        ] {
            let result = RustoreOptions::parse(&HashMap::from([(key, value)]), None);
            assert!(result.is_err(), "accepted invalid {key}");
            let error = result.err().unwrap();
            assert!(error.contains(key));
            assert!(!error.contains("secret-list-value"));
        }
    }
    for value in ["", "TRUE", "1", "secret-bool-value"] {
        let error = RustoreOptions::parse(&HashMap::from([("without_splits", value)]), None)
            .err()
            .unwrap();
        assert!(error.contains("without_splits"));
        assert!(!error.contains("secret-bool-value"));
    }
}

#[test]
fn invalid_version_or_user_agent_never_falls_back_or_leaks_values() {
    for value in [
        "",
        "0",
        "000",
        "-1",
        "+1",
        "1.2",
        " 11500",
        "secret-header-value",
        "11500\r\nsecret-header-value",
    ] {
        for (options, env, source) in [
            (
                HashMap::from([("rustore_version", value)]),
                Some("19000"),
                "rustore_version",
            ),
            (HashMap::new(), Some(value), "APKEEP_RUSTORE_VER"),
        ] {
            let result = RustoreOptions::parse(&options, env);
            assert!(result.is_err(), "accepted invalid version from {source}");
            let error = result.err().unwrap();
            assert!(error.contains(source));
            assert!(!error.contains("secret-header-value"));
        }
    }
    for value in ["", "secret-agent\r\ninjected", "secret-agent\0"] {
        let result = RustoreOptions::parse(&HashMap::from([("user_agent", value)]), None);
        assert!(result.is_err());
        let error = result.err().unwrap();
        assert!(error.contains("user_agent"));
        assert!(!error.contains("secret-agent"));
    }
}

#[test]
fn header_options_override_environment_and_defaults() {
    for (options, env, version, ua) in [
        (
            HashMap::from([("rustore_version", "22000"), ("user_agent", "Custom/2")]),
            Some("19000"),
            "22000",
            "Custom/2",
        ),
        (
            HashMap::new(),
            Some("19000"),
            "19000",
            "RuStore/1.78.0.1 (Android 11; SDK 30; arm64-v8a; samsung SM-N935F; en)",
        ),
        (
            HashMap::from([("rustore_version", "22000")]),
            Some("bad-secret-env"),
            "22000",
            "RuStore/1.78.0.1 (Android 11; SDK 30; arm64-v8a; samsung SM-N935F; en)",
        ),
    ] {
        let config = RustoreOptions::parse(&options, env).unwrap();
        assert_eq!(config.headers["rustorevercode"], version);
        assert_eq!(config.headers["user-agent"], ua);
    }
}

#[test]
fn device_options_change_download_request() {
    let options = HashMap::from([
        ("arch", "x86;x86_64"),
        ("sdk_version", "34"),
        ("screen_density", "480"),
        ("locales", "ru_RU;en_US"),
        ("mobile_services", "HMS"),
        ("without_splits", "false"),
    ]);
    let config = RustoreOptions::parse(&options, None).unwrap();
    let body: Value = serde_json::from_str(&config.download_request_body(73)).unwrap();
    assert_eq!(
        body,
        json!({
            "appId": 73, "firstInstall": true, "mobileServices": ["HMS"],
            "supportedAbis": ["x86", "x86_64"], "screenDensity": 480,
            "supportedLocales": ["ru_RU", "en_US"], "sdkVersion": 34,
            "withoutSplits": false, "signatureFingerprint": null
        })
    );
}

#[test]
fn characterizes_default_device_request() {
    let config = RustoreOptions::parse(&HashMap::new(), None).unwrap();
    let body: Value = serde_json::from_str(&config.download_request_body(42)).unwrap();
    assert_eq!(
        body,
        json!({
            "appId": 42, "firstInstall": true, "mobileServices": ["GMS", "HMS"],
            "supportedAbis": ["arm64-v8a"], "screenDensity": 420,
            "supportedLocales": ["en_US"], "sdkVersion": 30,
            "withoutSplits": true, "signatureFingerprint": null
        })
    );
}

#[test]
fn characterizes_default_headers() {
    let headers = RustoreOptions::parse(&HashMap::new(), None)
        .unwrap()
        .headers;
    assert_eq!(
        headers["user-agent"],
        "RuStore/1.78.0.1 (Android 11; SDK 30; arm64-v8a; samsung SM-N935F; en)"
    );
    assert_eq!(headers["content-type"], "application/json");
}

#[test]
fn sends_required_rustore_version_header() {
    assert_eq!(
        RustoreOptions::parse(&HashMap::new(), None)
            .unwrap()
            .headers
            .get("ruStoreVerCode")
            .and_then(|v| v.to_str().ok()),
        Some("11500")
    );
}
