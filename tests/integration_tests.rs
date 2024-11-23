use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use tokio::io;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use rust_toy_webserver::{Server, HTML_ROOT};

// Helper struct to manage the test environment
struct TestServer {
    addr: SocketAddr,
    _shutdown_token: CancellationToken,
    _server_handle: JoinHandle<io::Result<()>>,
}

impl TestServer {
    async fn new() -> Self {
        // Start server on random port
        std::env::set_var("PORT", "0");
        let shutdown_token = CancellationToken::new();

        let real_server = Server::new().await.unwrap();
        let addr = real_server.listener.local_addr().unwrap();
        let server_handle = tokio::task::spawn(real_server.run(shutdown_token.clone()));

        TestServer {
            addr,
            _shutdown_token: shutdown_token,
            _server_handle: server_handle,
        }
    }

    async fn send_request(&self, path: &str) -> reqwest::Response {
        let host = self.addr.to_string();
        reqwest::get(format!("http://{host}{path}")).await.unwrap()
    }
}

#[tokio::test]
async fn test_basic_request() {
    let server = TestServer::new().await;
    let response = server.send_request("/").await;
    let body = response.text().await.unwrap();
    assert!(body.contains("Hello"));
}

#[tokio::test]
async fn test_404_not_found() {
    let server = TestServer::new().await;
    assert!(!fs::exists(Path::new(HTML_ROOT).join("nonexistent.html")).unwrap());

    let response = server.send_request("/nonexistent.html").await;
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_path_traversal_attempt() {
    let server = TestServer::new().await;
    let response = server.send_request("/../../../etc/passwd").await;
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_content_length_accuracy() {
    let server = TestServer::new().await;
    let response = server.send_request("/").await;

    let expected_file = fs::File::open(Path::new(HTML_ROOT).join("hello.html")).unwrap();
    let file_length = expected_file.metadata().unwrap().len() as usize;

    // Content-Length header
    let content_length_header = response.content_length().unwrap() as usize;

    // Actual content
    let body_len = response.text().await.unwrap().len();

    assert_eq!(body_len, content_length_header);
    assert_eq!(body_len, file_length);
}

#[tokio::test]
#[ignore]
async fn test_absolute_path_request() {
    let server = TestServer::new().await;
    let mut stream = TcpStream::connect(server.addr).await.unwrap();

    // Send malformed request
    stream
        .write_all(b"GET localhost/some_path HTTP/1.1\r\n\r\n")
        .await
        .unwrap();

    let reader = BufReader::new(stream);
    let response_line = reader.lines().next_line().await.unwrap().unwrap();

    assert!(response_line.contains("HTTP/1.1 400 BAD REQUEST"));
}

#[tokio::test]
#[ignore]
async fn test_malformed_request() {
    let server = TestServer::new().await;
    let mut stream = TcpStream::connect(server.addr).await.unwrap();

    // Send malformed request
    stream.write_all(b"INVALID REQUEST\r\n\r\n").await.unwrap();

    let reader = BufReader::new(stream);
    let response_line = reader.lines().next_line().await.unwrap().unwrap();

    assert!(response_line.contains("HTTP/1.1 400 BAD REQUEST"));
}

#[tokio::test]
#[ignore]
async fn test_sleep_endpoint() {
    let server = TestServer::new().await;
    let start = std::time::Instant::now();
    let _response = server.send_request("/sleep").await;
    let duration = start.elapsed();

    assert!(duration >= Duration::from_secs(3));
}
