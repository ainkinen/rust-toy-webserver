mod threadpool;

use crate::threadpool::ThreadPool;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::Duration;
use std::{env, fs, io, mem, ptr, thread};

const HTML_ROOT: &str = "html_root";

fn main() -> io::Result<()> {
    let port = env::var("PORT").unwrap_or("8080".to_string());
    // TODO: Bind without string formatting
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;
    listener.set_nonblocking(true)?;

    // Kqueue
    let kq = unsafe {
        let fd = libc::kqueue();
        if fd == -1 {
            return Err(io::Error::last_os_error());
        }
        fd
    };

    // Events to monitor
    let changes = [
        // TCP listener
        libc::kevent {
            ident: listener.as_raw_fd() as usize,
            filter: libc::EVFILT_READ,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: 0,
            data: 0,
            udata: ptr::null_mut(),
        },
        // SIGINT
        libc::kevent {
            ident: libc::SIGINT as usize,
            filter: libc::EVFILT_SIGNAL,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: 0,
            data: 0,
            udata: ptr::null_mut(),
        },
        // SIGTERM
        libc::kevent {
            ident: libc::SIGTERM as usize,
            filter: libc::EVFILT_SIGNAL,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: 0,
            data: 0,
            udata: ptr::null_mut(),
        },
    ];

    // Register for events
    unsafe {
        if libc::kevent(
            kq,
            changes.as_ptr(),
            changes.len() as i32,
            ptr::null_mut(),
            0,
            ptr::null(),
        ) == -1
        {
            return Err(io::Error::last_os_error());
        }

        // Signals
        let mut sigset: libc::sigset_t = mem::zeroed();
        libc::sigemptyset(&mut sigset);
        libc::sigaddset(&mut sigset, libc::SIGINT);
        libc::sigaddset(&mut sigset, libc::SIGTERM);
        libc::sigprocmask(libc::SIG_BLOCK, &sigset, ptr::null_mut());
    }

    // Event loop
    let mut events = vec![
        libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: ptr::null_mut(),
        };
        1024
    ];

    let thread_pool = ThreadPool::new(4);

    let local_addr = listener.local_addr()?;
    println!("Listening on: {local_addr}");

    'event_loop: loop {
        let num_events = unsafe {
            libc::kevent(
                kq,
                ptr::null(),
                0,
                events.as_mut_ptr(),
                events.len() as i32,
                ptr::null(), // TODO: Add timeout?
            )
        };

        if num_events == -1 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }

        for event in &events[0..num_events as usize] {
            match (event.filter, event.ident) {
                // Signals
                (libc::EVFILT_SIGNAL, sig_num) => match sig_num {
                    sig if sig == libc::SIGINT as usize => {
                        println!("SIGINT");
                        break 'event_loop;
                    }
                    sig if sig == libc::SIGTERM as usize => {
                        println!("SIGTERM");
                        break 'event_loop;
                    }
                    _ => println!("Received unexpected signal {}", sig_num),
                },
                // Connections
                (libc::EVFILT_READ, fd) if fd == listener.as_raw_fd() as usize => loop {
                    match listener.accept() {
                        Ok((stream, peer_addr)) => {
                            println!("Connection from {peer_addr} established!");

                            thread_pool.execute(move || {
                                handle_connection(stream);
                            });
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => {
                            eprintln!("Error accepting connection: {e}");
                            break;
                        }
                    };
                },
                (filter, ident) => {
                    println!(
                        "Received unexpected event: filter={}, ident={}",
                        filter, ident
                    );
                }
            }
        }
    }
    println!("Shutting down");
    Ok(())
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    let line = buf_reader.lines().next();
    if line.is_none() {
        // No data from the stream
        return;
    }

    let request_line = line.unwrap().unwrap();

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
