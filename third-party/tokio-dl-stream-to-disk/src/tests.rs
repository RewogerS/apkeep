use super::*;
use std::time::Duration;
use tokio::net::TcpListener;

// Only unit tests can select short deadlines. Production has no env override.
fn short_download(url: &str, out: &Path) -> AsyncDownload {
    let mut download = AsyncDownload::new(url, out, "artifact.bin");
    download.timeouts = Timeouts {
        connect: Duration::from_millis(80),
        read: Duration::from_millis(80),
        whole: Duration::from_millis(240),
    };
    download
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
    let result = tokio::time::timeout(Duration::from_secs(1), download.download(&None))
        .await
        .expect("whole-transfer deadline must stop a non-stalled trickle");
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}
