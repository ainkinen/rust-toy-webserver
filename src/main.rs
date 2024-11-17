use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::time::Duration;
use std::{env, fs, thread};

const HTML_ROOT: &str = "html_root";

fn main() {
    let port = env::var("PORT").unwrap_or("8080".to_string());
    // TODO: Bind without string formatting
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();

    let local_addr = listener.local_addr().unwrap();
    println!("Listening on: {local_addr}");

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        let peer_addr = stream.peer_addr().unwrap();
        println!("Connection from {peer_addr} established!");

        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let request_line_parts: Vec<_> = request_line.split(" ").collect();
    let [_method, request_path, _http_version] = request_line_parts.try_into().unwrap();

    let request_path = match request_path {
        "/" => "hello.html", // Use hello as the default index page
        "/sleep" => {
            thread::sleep(Duration::from_secs(3));
            "hello.html"
        }
        path if path.starts_with("/") => &path[1..], // Strip leading /
        _ => panic!("Absolute request path not supported"),
    };

    // Canonicalize the path and check that we are still inside the html root
    let html_root = Path::new(HTML_ROOT).canonicalize().unwrap();
    let file_path = Path::join(&html_root, request_path);

    let (status_line, content_path) = if exists_within(&html_root, &file_path) {
        ("HTTP/1.1 200 OK", file_path)
    } else {
        ("HTTP/1.1 404 NOT FOUND", Path::join(&html_root, "404.html"))
    };

    let contents = fs::read_to_string(content_path).unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}

fn exists_within(canonical_root: &Path, path: &Path) -> bool {
    if let Ok(canonical_path) = path.canonicalize() {
        return canonical_path.starts_with(canonical_root);
    }

    false
}
