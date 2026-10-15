//! Contract tests for the shared downloader dependency, not public store probes.
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_dl_stream_to_disk::AsyncDownload;

async fn serve(body: Vec<u8>, declared_extra: usize) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/file", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            let mut buf = [0; 1024];
            let count = socket.read(&mut buf).await.unwrap();
            assert!(count > 0 && request.len() < 8192);
            request.extend_from_slice(&buf[..count]);
        }
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len() + declared_extra
        );
        socket.write_all(header.as_bytes()).await.unwrap();
        socket.write_all(&body).await.unwrap();
    });
    (endpoint, task)
}

async fn download_case(with_hash: bool, preflight: bool, body: Vec<u8>) {
    let out = tempfile::tempdir().unwrap();
    let (url, server) = serve(body.clone(), 0).await;
    let mut downloader = AsyncDownload::new(&url, out.path(), "artifact.bin");
    if preflight {
        downloader = downloader.get().await.unwrap();
    }
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        if with_hash {
            let hash = downloader
                .download_and_return_sha256sum(&None)
                .await
                .unwrap();
            assert_eq!(hash, Sha256::digest(&body).to_vec());
        } else {
            downloader.download(&None).await.unwrap();
        }
    })
    .await
    .expect("bounded loopback download");
    server.await.unwrap();
    let actual = std::fs::read(out.path().join("artifact.bin")).unwrap();
    assert!(
        actual == body,
        "success returned before all output bytes were visible: actual={}, expected={}",
        actual.len(),
        body.len()
    );
}

#[tokio::test]
async fn plain_download_flushes_complete_output_before_returning() {
    for size in [0, 1, 8193, 1024 * 1024] {
        download_case(false, false, (0..size).map(|i| (i % 251) as u8).collect()).await;
    }
}

#[tokio::test]
async fn sha_download_without_preflight_is_initialized_and_flushed() {
    download_case(true, false, vec![0x5a; 24579]).await;
}

#[tokio::test]
async fn sha_download_with_preflight_flushes_complete_output() {
    download_case(true, true, vec![0xa5; 24579]).await;
}

#[tokio::test]
async fn existing_output_is_not_overwritten_in_either_download_mode() {
    for with_hash in [false, true] {
        let out = tempfile::tempdir().unwrap();
        std::fs::write(out.path().join("artifact.bin"), b"keep").unwrap();
        let (url, server) = serve(b"replacement".to_vec(), 0).await;
        let mut downloader = AsyncDownload::new(&url, out.path(), "artifact.bin")
            .get()
            .await
            .unwrap();
        let failed = if with_hash {
            downloader
                .download_and_return_sha256sum(&None)
                .await
                .is_err()
        } else {
            downloader.download(&None).await.is_err()
        };
        assert!(failed);
        server.await.unwrap();
        assert_eq!(
            std::fs::read(out.path().join("artifact.bin")).unwrap(),
            b"keep"
        );
    }
}

#[tokio::test]
async fn truncated_body_does_not_report_success() {
    let out = tempfile::tempdir().unwrap();
    let (url, server) = serve(b"partial".to_vec(), 1024).await;
    let mut downloader = AsyncDownload::new(&url, Path::new(out.path()), "artifact.bin");
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        downloader.download(&None),
    )
    .await
    .unwrap();
    server.await.unwrap();
    assert!(result.is_err());
}
