use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn headers_have_stable_defaults() {
    let headers = http_headers(&HashMap::new());
    assert_eq!(headers.len(), 4);
    assert_eq!(headers["x-cv"], "3172501");
    assert_eq!(headers["x-sv"], "29");
    assert_eq!(headers["x-gp"], "1");
    assert_eq!(
        headers["x-abis"],
        "arm64-v8a,armeabi-v7a,armeabi,x86,x86_64"
    );
}

#[test]
fn headers_translate_semicolon_separated_abis_without_reordering() {
    let headers = http_headers(&HashMap::from([("arch", "x86_64;arm64-v8a;x86")]));
    assert_eq!(headers["x-abis"], "x86_64,arm64-v8a,x86");
    assert_eq!(headers.len(), 4);
}

#[test]
fn invalid_header_values_are_omitted_without_panicking() {
    for arch in ["arm64-v8a\r\nx-injected: yes", "x86\n", "x86\0"] {
        let headers = http_headers(&HashMap::from([("arch", arch)]));
        assert!(!headers.contains_key("x-abis"));
        assert_eq!(headers.len(), 3);
        assert_eq!(headers["x-gp"], "1");
        assert!(!headers.contains_key("x-injected"));
    }
}

#[test]
fn header_parser_currently_preserves_empty_and_unvalidated_abi_values() {
    // Characterization, not an ABI-validation security guarantee.
    for (arch, expected) in [
        ("", ""),
        ("x86;;arm64-v8a", "x86,,arm64-v8a"),
        ("unknown", "unknown"),
    ] {
        assert_eq!(
            http_headers(&HashMap::from([("arch", arch)]))["x-abis"],
            expected
        );
    }
}

#[test]
fn production_url_regex_selects_first_apk_or_xapk_with_stable_capture_indices() {
    let re = download_regex(None);
    for kind in ["APKJ", "XAPKJ"] {
        let body = format!("{kind}..https://cdn.example.org/file.apk?token=fake&v=1\" APKJ..https://other.example.org/second.apk\"");
        let captures = re.captures(&body).unwrap();
        assert_eq!(&captures[1], kind);
        assert_eq!(
            &captures[2],
            "https://cdn.example.org/file.apk?token=fake&v=1"
        );
    }
    assert!(re
        .captures("APKJ..ftp://cdn.example.org/file.apk")
        .is_none());
    assert!(re.captures("no download record").is_none());
}

#[test]
fn production_url_regex_characterizes_unsafe_urls_and_accepts_ephemeral_ports() {
    // These assertions document inherited parsing gaps, not desired URL policy.
    let re = download_regex(None);
    for url in [
        "http://127.0.0.1/internal",
        "http://169.254.169.254/latest/meta-data/",
        "http://user:fake@example.org/file.apk",
    ] {
        assert_eq!(&re.captures(&format!("APKJ..{url}\"")).unwrap()[2], url);
    }
    assert_eq!(
        &re.captures("APKJ..http://127.0.0.1:12345/file.apk\"")
            .unwrap()[2],
        "http://127.0.0.1:12345/file.apk"
    );
    assert_eq!(
        &re.captures("APKJ..https://example.org/file.apk)").unwrap()[2],
        "https://example.org/file.apk)"
    );
}

#[test]
fn versions_url_preserves_normal_package_and_locale_without_network() {
    let url = versions_url("com.example.fixture");
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("api.pureapk.com"));
    assert_eq!(url.path(), "/m/v3/cms/app_version");
    let pairs = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect::<Vec<_>>();
    assert_eq!(
        pairs,
        [
            ("hl".to_owned(), "en-US".to_owned()),
            ("package_name".to_owned(), "com.example.fixture".to_owned())
        ]
    );
}

#[test]
fn versions_url_currently_allows_query_and_fragment_injection() {
    // URL parsing runs locally; these strings are never requested.
    let url = versions_url("com.example.fixture&hl=xx");
    let locales = url
        .query_pairs()
        .filter(|(key, _)| key == "hl")
        .map(|(_, value)| value.into_owned())
        .collect::<Vec<_>>();
    assert_eq!(locales, ["en-US", "xx"]);
    let url = versions_url("com.example.fixture#fragment");
    assert_eq!(url.fragment(), Some("fragment"));
    assert_eq!(
        url.query_pairs()
            .find(|(key, _)| key == "package_name")
            .unwrap()
            .1,
        "com.example.fixture"
    );
}

#[test]
fn requested_version_regex_escapes_metacharacters_and_ignores_other_versions() {
    let version = "1.2+[beta](rc)";
    let re = download_regex(Some(version));
    let body = format!(":9.9:(APKJ..https://example.org/other.apk\") :{version}:(XAPKJ..https://example.org/wanted.xapk\")");
    let captures = re.captures(&body).unwrap();
    assert_eq!(&captures[1], "XAPKJ");
    assert_eq!(&captures[2], "https://example.org/wanted.xapk");
    assert!(re
        .captures(":1x2beta:(APKJ..https://example.org/other.apk\")")
        .is_none());
    assert!(download_regex(Some("1.0"))
        .captures(":11.0:(APKJ..https://example.org/other.apk\")")
        .is_none());
}

#[test]
fn requested_version_regex_currently_crosses_record_boundaries() {
    // Known unresolved defect: do not confuse characterization with a fix.
    let re = download_regex(Some("1.0"));
    let body = ":1.0:(no URL in this record) :2.0:(APKJ..https://example.org/two.apk\")";
    assert_eq!(
        &re.captures(body).unwrap()[2],
        "https://example.org/two.apk"
    );
}

#[test]
fn versions_regex_requires_at_least_forty_hex_digits() {
    let re = versions_regex();
    let hex = "0123456789abcdef0123456789abcdef01234567";
    let body = format!(
        "1.0:({hex}) 2.0-beta:({hex}ab) 3.0:({}) 4.0:({})",
        &hex[..39],
        "z".repeat(40)
    );
    let found = re
        .captures_iter(&body)
        .map(|caps| caps[1].to_owned())
        .collect::<Vec<_>>();
    assert_eq!(found, ["1.0", "2.0-beta"]);
}

#[test]
fn versions_regex_currently_truncates_underscore_and_plus_prefixes() {
    // Characterizes the real Rust regex's suffix matching, not a Python model.
    let re = versions_regex();
    let hex = "a".repeat(40);
    let body = format!("1.0_rc:({hex}) 2.0+build:({hex})");
    let found = re
        .captures_iter(&body)
        .map(|caps| caps[1].to_owned())
        .collect::<Vec<_>>();
    assert_eq!(found, ["rc", "build"]);
}

// A real loopback HTTP server. Nonblocking accept, bounded socket IO and Drop join
// make both a missing request and a regression safe for the test runner.
struct Server {
    base: String,
    stop: Arc<std::sync::atomic::AtomicBool>,
    paths: Arc<Mutex<Vec<String>>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn new(
        metadata_status: u16,
        metadata: impl FnOnce(&str) -> String,
        artifact_status: u16,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let metadata = metadata(&base);
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let paths = Arc::new(Mutex::new(Vec::new()));
        let worker_stop = Arc::clone(&stop);
        let worker_paths = Arc::clone(&paths);
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            while !worker_stop.load(std::sync::atomic::Ordering::SeqCst)
                && Instant::now() < deadline
            {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        let Some(path) = read_path(&mut socket) else {
                            continue;
                        };
                        worker_paths.lock().unwrap().push(path.clone());
                        let (status, body) = if path == "/metadata" {
                            (metadata_status, metadata.as_bytes())
                        } else {
                            (
                                artifact_status,
                                b"offline synthetic artifact\0\xff".as_slice(),
                            )
                        };
                        let head = format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                        let _ = socket.write_all(head.as_bytes());
                        let _ = socket.write_all(body);
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(err) => panic!("fixture accept: {err}"),
                }
            }
        });
        Self {
            base,
            stop,
            paths,
            worker: Some(worker),
        }
    }

    fn paths(&self) -> Vec<String> {
        self.paths.lock().unwrap().clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn read_path(socket: &mut TcpStream) -> Option<String> {
    socket
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buf = [0; 1024];
    while !bytes.windows(4).any(|w| w == b"\r\n\r\n") && bytes.len() < 8192 {
        match socket.read(&mut buf) {
            Ok(0) | Err(_) => return None,
            Ok(n) => bytes.extend_from_slice(&buf[..n]),
        }
    }
    String::from_utf8(bytes)
        .ok()?
        .lines()
        .next()?
        .split_whitespace()
        .nth(1)
        .map(str::to_owned)
}

async fn consume(server: &Server, out: &Path, app: &str) {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let response = client
        .get(format!("{}/metadata", server.base))
        .send()
        .await
        .unwrap();
    // Use the shipped regex, including its real capture numbering. The existing
    // private Response argument is the only seam; endpoints are never overridden.
    let re = download_regex(None);
    tokio::time::timeout(
        Duration::from_secs(5),
        download_from_response(
            response,
            Box::new(Box::new(re)),
            app.to_owned(),
            out,
            Rc::new(MultiProgress::with_draw_target(
                indicatif::ProgressDrawTarget::hidden(),
            )),
        ),
    )
    .await
    .expect("bounded loopback download");
}

#[tokio::test(flavor = "current_thread")]
async fn response_downloads_apk_and_xapk_with_eventually_exact_fixture_bytes() {
    for (kind, extension) in [("APKJ", "apk"), ("XAPKJ", "xapk")] {
        let server = Server::new(200, |base| format!("{kind}..{base}/artifact"), 200);
        let out = tempfile::tempdir().unwrap();
        consume(&server, out.path(), "com.example.fixture@1").await;
        let path = out
            .path()
            .join(format!("com.example.fixture@1.{extension}"));
        // AsyncDownload drops tokio::fs::File without flushing. Tokio documents
        // that in-flight writes may outlive Drop. This characterizes eventual
        // byte correctness, NOT immediate visibility/atomic publication.
        let expected = b"offline synthetic artifact\0\xff";
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if std::fs::read(&path).unwrap() == expected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("artifact bytes must become visible within two seconds");
        assert_eq!(std::fs::read(&path).unwrap(), expected);
        assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 1);
        assert_eq!(server.paths(), ["/metadata", "/artifact"]);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn response_metadata_http_error_creates_no_artifact() {
    let server = Server::new(503, |base| format!("APKJ..{base}/artifact"), 200);
    let out = tempfile::tempdir().unwrap();
    consume(&server, out.path(), "com.example.fixture").await;
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
    assert_eq!(server.paths(), ["/metadata"]);
}

#[tokio::test(flavor = "current_thread")]
async fn response_without_url_creates_no_artifact() {
    let server = Server::new(200, |_| "no matching download URL".to_owned(), 200);
    let out = tempfile::tempdir().unwrap();
    consume(&server, out.path(), "com.example.fixture").await;
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
    assert_eq!(server.paths(), ["/metadata"]);
}

#[tokio::test(flavor = "current_thread")]
async fn response_artifact_http_error_creates_no_file() {
    let server = Server::new(200, |base| format!("APKJ..{base}/artifact"), 404);
    let out = tempfile::tempdir().unwrap();
    consume(&server, out.path(), "com.example.fixture").await;
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
    assert_eq!(server.paths(), ["/metadata", "/artifact"]);
}

#[tokio::test(flavor = "current_thread")]
async fn response_preserves_existing_directory_when_all_three_writes_fail() {
    let server = Server::new(200, |base| format!("APKJ..{base}/artifact"), 200);
    let out = tempfile::tempdir().unwrap();
    let destination = out.path().join("com.example.fixture.apk");
    std::fs::create_dir(&destination).unwrap();
    std::fs::write(destination.join("marker"), b"preserved").unwrap();
    consume(&server, out.path(), "com.example.fixture").await;
    assert!(destination.is_dir());
    assert_eq!(
        std::fs::read(destination.join("marker")).unwrap(),
        b"preserved"
    );
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 1);
    assert_eq!(
        server.paths(),
        ["/metadata", "/artifact", "/artifact", "/artifact"]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn response_preserves_existing_file_even_when_network_has_new_bytes() {
    let server = Server::new(200, |base| format!("APKJ..{base}/artifact"), 200);
    let out = tempfile::tempdir().unwrap();
    let file = out.path().join("com.example.fixture.apk");
    std::fs::write(&file, b"original marker").unwrap();
    consume(&server, out.path(), "com.example.fixture").await;
    assert_eq!(std::fs::read(file).unwrap(), b"original marker");
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 1);
    // Current implementation fetches headers before discovering FileExists.
    assert_eq!(server.paths(), ["/metadata", "/artifact"]);
}
