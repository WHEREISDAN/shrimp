//! Model downloads against a local HTTP server: progress, checksum
//! verification and the rename-into-place rule.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use sha2::{Digest, Sha256};
use shrimp_lib::stt::models::{ModelError, ModelFile, ModelStore};

/// Serves `body` once on a local port and returns its URL.
fn serve_once(body: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/model.bin", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap() > 2 {
            line.clear();
        }
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(header.as_bytes()).unwrap();
        stream.write_all(&body).unwrap();
        // Close gracefully: dropping a socket with unread input resets the
        // connection, and the client would see an error instead of the body.
        let _ = stream.shutdown(std::net::Shutdown::Write);
        let _ = std::io::copy(&mut reader, &mut std::io::sink());
    });
    url
}

fn model(url: String, body: &[u8], sha256: String) -> ModelFile {
    ModelFile {
        id: "test",
        label: "Test model",
        file_name: "ggml-test.bin",
        url: Box::leak(url.into_boxed_str()),
        size: body.len() as u64,
        sha256: Box::leak(sha256.into_boxed_str()),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[tokio::test]
async fn a_verified_download_is_kept_and_reused() {
    let body: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    let model = model(serve_once(body.clone()), &body, sha256_hex(&body));
    let dir = tempfile::tempdir().unwrap();
    let store = ModelStore::new(dir.path());

    let mut progress = Vec::new();
    let path = store
        .download(&reqwest::Client::new(), &model, |received, total| {
            progress.push((received, total));
        })
        .await
        .unwrap();

    assert_eq!(std::fs::read(&path).unwrap(), body);
    assert!(store.is_downloaded(&model));
    assert_eq!(progress.first(), Some(&(0, body.len() as u64)));
    assert_eq!(
        progress.last(),
        Some(&(body.len() as u64, body.len() as u64))
    );
}

#[tokio::test]
async fn a_download_with_the_wrong_checksum_is_discarded() {
    let body = b"not the model you pinned".repeat(1_000);
    let model = model(serve_once(body.clone()), &body, "00".repeat(32));
    let dir = tempfile::tempdir().unwrap();
    let store = ModelStore::new(dir.path());

    let result = store
        .download(&reqwest::Client::new(), &model, |_, _| {})
        .await;

    assert!(matches!(result, Err(ModelError::Checksum)));
    assert!(!store.is_downloaded(&model));
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "no partial file left"
    );
}
