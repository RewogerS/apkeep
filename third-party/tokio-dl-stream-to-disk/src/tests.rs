use super::*;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::test]
async fn production_download_rejects_loopback_before_connecting() {
    let out = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/file", listener.local_addr().unwrap());
    let mut download = AsyncDownload::new(&url, out.path(), "artifact.bin");
    let result = tokio::time::timeout(Duration::from_millis(200), download.download(&None))
        .await
        .expect("production policy must reject loopback without waiting for HTTP");
    assert!(result.is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(20), listener.accept())
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}

#[test]
fn public_url_validation_rejects_special_addresses_and_userinfo() {
    for address in [
        "0.0.0.0",
        "0.1.2.3",
        "10.0.0.1",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "192.168.0.1",
        "192.0.0.9",
        "192.0.2.1",
        "192.88.99.1",
        "198.18.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "240.0.0.1",
        "255.255.255.255",
        "[::]",
        "[::1]",
        "[::ffff:127.0.0.1]",
        "[::ffff:8.8.8.8]",
        "[fc00::1]",
        "[fe80::1]",
        "[ff02::1]",
        "[2001:db8::1]",
        "[2002:7f00:1::]",
        "[64:ff9b::7f00:1]",
        "[100::1]",
        "[2001::1]",
        "[3fff::1]",
        "[fec0::1]",
    ] {
        let url = reqwest::Url::parse(&format!("http://{address}/file")).unwrap();
        assert!(validate_public_url(&url).is_err(), "accepted {address}");
    }
    for value in [
        "ftp://example.com/file",
        "file:///etc/passwd",
        "https://u:p@example.com/",
        "https://u@example.com/",
        "http://2130706433/",
        "http://0x7f000001/",
        "http://127.1/",
        "http://0177.0.0.1/",
    ] {
        assert!(validate_public_url(&reqwest::Url::parse(value).unwrap()).is_err());
    }
    for value in [
        "https://example.com/",
        "http://8.8.8.8/file",
        "https://[2606:4700:4700::1111]/",
    ] {
        validate_public_url(&reqwest::Url::parse(value).unwrap()).unwrap();
    }
    public_http_client_builder().build().unwrap();
}

// Only unit tests can select short deadlines. Production has no env override.
fn short_download(url: &str, out: &Path) -> AsyncDownload {
    let timeouts = Timeouts {
        connect: Duration::from_millis(80),
        read: Duration::from_millis(80),
        whole: Duration::from_millis(240),
    };
    let client = fixture_client(timeouts);
    let mut download = AsyncDownload::new(url, out, "artifact.bin").with_client(client);
    download.timeouts = timeouts;
    download
}

fn fixture_client(timeouts: Timeouts) -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(timeouts.connect)
        .read_timeout(timeouts.read)
        .timeout(timeouts.whole)
        .build()
        .unwrap()
}

struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn stalled(body: bool) -> (String, Server) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/file", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.windows(4).any(|w| w == b"\r\n\r\n") {
            let mut buf = [0; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            assert!(n > 0);
            headers.extend_from_slice(&buf[..n]);
        }
        if body {
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\npartial")
                .await
                .unwrap();
        }
        std::future::pending::<()>().await;
        drop(socket);
    });
    (url, Server(server))
}

#[tokio::test]
async fn default_budget_rejects_declared_oversize_before_tempfile_or_body_write() {
    for preflight in [false, true] {
        let out = tempfile::tempdir().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let _server = Server(tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            socket.read(&mut request).await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2147483649\r\n\r\nx")
                .await
                .unwrap();
            std::future::pending::<()>().await;
            drop(socket);
        }));
        let written = std::rc::Rc::new(std::cell::Cell::new(0));
        let observed = written.clone();
        let cb: Option<Box<dyn Fn(u64)>> = Some(Box::new(move |bytes| observed.set(bytes)));
        let mut download = short_download(&url, out.path());
        let failed = if preflight {
            download.get().await.is_err()
        } else {
            download.download(&cb).await.is_err()
        };
        assert!(
            failed,
            "oversized headers must fail even for header-only get"
        );
        assert_eq!(written.get(), 0, "must reject before the first write");
        assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn streaming_budget_checks_each_write_even_without_length_or_sha() {
    for (length, cap, chunks, expected_written, success) in [
        (None, 8, vec![4, 4, 2], 8, false),
        (None, 8, vec![9], 0, false),
        (Some(5), 8, vec![4, 2], 4, false),
        (Some(8), 8, vec![4, 4], 8, true),
        (None, 8, vec![4, 4], 8, true),
        (None, 0, vec![1], 0, false),
    ] {
        let out = tempfile::tempdir().unwrap();
        let mut download = short_download("http://unused.invalid/", out.path());
        download.max_bytes = cap;
        download.length = length;
        download.timeouts = Timeouts::default();
        download.response_stream = Some(Box::new(futures_util::stream::iter(
            chunks
                .into_iter()
                .map(|size| Ok(Bytes::from(vec![b'x'; size]))),
        )));
        let written = std::rc::Rc::new(std::cell::Cell::new(0));
        let observed = written.clone();
        let cb: Option<Box<dyn Fn(u64)>> = Some(Box::new(move |bytes| observed.set(bytes)));
        let result = download.download(&cb).await;
        assert_eq!(
            written.get(),
            expected_written,
            "bytes accepted by write_all must stay within cap AND advertised size"
        );
        assert_eq!(result.is_ok(), success);
        if success {
            assert_eq!(
                std::fs::metadata(out.path().join("artifact.bin"))
                    .unwrap()
                    .len(),
                expected_written
            );
            assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 1);
        } else {
            assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
        }
    }
}

#[tokio::test]
async fn configured_budget_is_inclusive_and_cannot_raise_absolute_default() {
    for limit in [8, u64::MAX] {
        let out = tempfile::tempdir().unwrap();
        let mut download =
            short_download("http://unused.invalid/", out.path()).with_max_bytes(limit);
        assert_eq!(download.max_bytes, limit.min(DEFAULT_MAX_BYTES));
        download.length = Some(9);
        download.response_stream = Some(Box::new(futures_util::stream::iter(vec![Ok(
            Bytes::from_static(b"123456789"),
        )])));
        let result = download.download(&None).await;
        assert_eq!(result.is_ok(), limit >= 9);
        assert_eq!(
            std::fs::read_dir(out.path()).unwrap().count(),
            usize::from(limit >= 9)
        );
    }
}

#[tokio::test]
async fn whole_transfer_timeout_cleans_temp_even_without_a_reqwest_timer() {
    let out = tempfile::tempdir().unwrap();
    let mut download = short_download("http://unused.invalid/", out.path());
    download.timeouts.whole = Duration::from_millis(80);
    download.response_stream = Some(Box::new(
        futures_util::stream::iter(vec![Ok(Bytes::from_static(b"partial"))])
            .chain(futures_util::stream::pending()),
    ));
    let result = tokio::time::timeout(Duration::from_secs(1), download.download(&None))
        .await
        .expect("outer deadline must cover non-HTTP transfer work too");
    assert!(
        matches!(result.unwrap_err().kind(), TDSTDErrorKind::IO(e) if e.kind() == ErrorKind::TimedOut)
    );
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn stalled_headers_are_bounded_for_download_and_get() {
    for preflight in [false, true] {
        let out = tempfile::tempdir().unwrap();
        let (url, _server) = stalled(false).await;
        let mut download = short_download(&url, out.path());
        let failed = tokio::time::timeout(Duration::from_secs(1), async {
            if preflight {
                download.get().await.is_err()
            } else {
                download.download(&None).await.is_err()
            }
        })
        .await
        .expect("library must bound stalled headers, not its caller");
        assert!(failed);
        assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn stalled_body_is_bounded_and_private_output_is_cleaned() {
    for mode in 0..3 {
        let out = tempfile::tempdir().unwrap();
        let (url, _server) = stalled(true).await;
        let mut download = short_download(&url, out.path());
        let failed = tokio::time::timeout(Duration::from_secs(1), async {
            match mode {
                #[cfg(feature = "sha256sum")]
                1 => download.download_and_return_sha256sum(&None).await.is_err(),
                #[cfg(feature = "sha256sum")]
                2 => download
                    .download_and_verify_sha256(&[0; 32], &None)
                    .await
                    .is_err(),
                _ => download.download(&None).await.is_err(),
            }
        })
        .await
        .expect("library must bound a stalled body");
        assert!(failed);
        assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn stalled_tls_handshake_is_bounded_by_connect_deadline() {
    let out = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("https://{}/file", listener.local_addr().unwrap());
    let _server = Server(tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
        drop(socket);
    }));
    let mut download = short_download(&url, out.path());
    download.timeouts.read = Duration::from_secs(3);
    download.timeouts.whole = Duration::from_secs(3);
    download.client = Some(fixture_client(download.timeouts));
    let result = tokio::time::timeout(Duration::from_secs(1), download.download(&None))
        .await
        .expect("TLS handshake must obey connect deadline independently");
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn whole_deadline_covers_preflight_and_body_without_reset() {
    let out = tempfile::tempdir().unwrap();
    let (url, _server) = stalled(true).await;
    let mut download = short_download(&url, out.path());
    download.timeouts.read = Duration::from_secs(3);
    download.client = Some(fixture_client(download.timeouts));
    let mut download = download.get().await.unwrap();
    assert_eq!(download.length(), Some(1000));
    tokio::time::sleep(Duration::from_millis(300)).await;
    let result = tokio::time::timeout(Duration::from_millis(100), download.download(&None))
        .await
        .expect("preflight must not reset the HTTP whole-transfer deadline");
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn whole_transfer_deadline_bounds_a_trickling_body() {
    let out = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/file", listener.local_addr().unwrap());
    let _server = Server(tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        let _ = socket.read(&mut request).await.unwrap();
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10000\r\n\r\n")
            .await
            .unwrap();
        loop {
            if socket.write_all(b"x").await.is_err() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }));
    let mut download = short_download(&url, out.path());
    download.timeouts.read = Duration::from_secs(2);
    download.client = Some(fixture_client(download.timeouts));
    let result = tokio::time::timeout(Duration::from_secs(1), download.download(&None))
        .await
        .expect("whole-transfer deadline must stop a non-stalled trickle");
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}
