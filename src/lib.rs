mod route_pattern;

use log::{debug, info};
use std::convert::TryInto;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;
use std::{env, fs, io};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::route_pattern::{RouteParams, RoutePattern};

pub const HTML_ROOT: &str = "html_root";

#[derive(Debug)]
pub struct Server {
    pub listener: TcpListener,
}

impl Server {
    pub async fn new() -> io::Result<Self> {
        let port = env::var("PORT").unwrap_or("8080".to_string());
        // TODO: Bind without string formatting
        let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind to port");

        let local_addr = listener.local_addr()?;
        info!("Listening on: {local_addr}");

        Ok(Self { listener })
    }

    pub async fn run(self, shutdown_token: CancellationToken) -> io::Result<()> {
        let handlers = TaskTracker::new();

        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    debug!("Stopped accepting new connections.");
                    break;
                }
                Ok((stream, peer_addr)) = self.listener.accept() => {
                    debug!("Connection from {peer_addr} established!");
                    handlers.spawn(handle_connection(stream));
                }
            }
        }

        debug!("Waiting for handlers to finish...");
        handlers.close();
        handlers.wait().await;

        Ok(())
    }
}

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>;
type HandlerRef = Box<
    dyn Fn(TcpStream, String, RouteParams) -> BoxFuture<io::Result<()>> + Send + Sync + 'static,
>;

fn wrap_handler<F, Fut>(f: F) -> HandlerRef
where
    F: Fn(TcpStream, String, RouteParams) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = io::Result<()>> + Send + Sync + 'static,
{
    Box::new(move |a, b, c| Box::pin(f(a, b, c)))
}

async fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let buf_reader = BufReader::new(&mut stream);

    let line = buf_reader.lines().next_line().await;

    let request_line = line?.unwrap();

    let request_line_parts: Vec<_> = request_line.split(" ").collect();
    let [_method, request_path, _http_version] = request_line_parts.try_into().unwrap();

    let routes: Vec<(RoutePattern, HandlerRef)> = vec![
        ("/sleep".parse().unwrap(), wrap_handler(sleep_handler)),
        ("/*".parse().unwrap(), wrap_handler(serve_file)),
    ];

    for (route, handler) in routes {
        if let Some(params) = route.matches(request_path) {
            debug!("route: {route:?} matches path: {request_path} with params: {params:?}");
            return handler(stream, request_path.to_string(), params).await;
        }
    }

    panic!("No routes matched!");
}

async fn sleep_handler(stream: TcpStream, _: String, p: RouteParams) -> io::Result<()> {
    tokio::time::sleep(Duration::from_secs(3)).await;
    serve_file(stream, "hello.html".to_string(), p).await
}

async fn serve_file(
    mut stream: TcpStream,
    request_path: String,
    _route_params: RouteParams,
) -> io::Result<()> {
    // Use hello.html as the index file
    let request_path = if request_path == "/" {
        "hello.html"
    } else {
        &request_path
    };

    // Canonicalize the path and check that we are still inside the html root
    let html_root = Path::new(HTML_ROOT).canonicalize()?;
    let file_path = Path::join(&html_root, request_path);

    let (status_line, content_path) = if exists_within(&html_root, &file_path) {
        ("HTTP/1.1 200 OK", file_path)
    } else {
        ("HTTP/1.1 404 NOT FOUND", Path::join(&html_root, "404.html"))
    };

    let contents = fs::read_to_string(content_path)?;
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).await
}

fn exists_within(canonical_root: &Path, path: &Path) -> bool {
    if let Ok(canonical_path) = path.canonicalize() {
        return canonical_path.starts_with(canonical_root);
    }

    false
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_exists_within() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().canonicalize().unwrap();

        // Create a test file
        let test_file = root.join("test.txt");
        fs::write(&test_file, "test").unwrap();

        // Test valid path
        assert!(exists_within(&root, &test_file));

        // Test path traversal
        let traversal_path = root.join("../test.txt");
        assert!(!exists_within(&root, &traversal_path));
    }
}
