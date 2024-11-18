# Toy Webserver in Rust

A lightweight web server written in Rust, originally inspired by the final chapter of the Rust
Book: [Building a Multithreaded Web Server](https://doc.rust-lang.org/book/ch20-00-final-project-a-web-server.html).
This project serves as both a learning tool and a practical example of web server fundamentals using Rust. It has been
extended with some additional features to enhance its capabilities.

> Disclaimer: This README has been partially LLM generated for structure and content, then customized for accuracy and
> project specifics.

## Features

Extended with some extras:

- **Asynchronous Runtime**: Powered by the [Tokio](https://tokio.rs/) async runtime for non-blocking IO operations,
  allowing for efficient handling of concurrent connections.
    - A custom event loop implementation can be found in the commit history, showcasing a "DIY" approach before
      switching to Tokio.
- **Static file hosting**: Serve static files from the `html_root` directory.
- **Simple Logging**

## Getting Started

### Prerequisites

- Rust (stable 1.82+ recommended)

### Running

The server can be started with `cargo run`.

- **Default Settings:**
    - The server listens at `127.0.0.1:8080` by default.
    - To change the port, set the PORT environment variable:
        ```bash
        PORT=8181 cargo run
        ```
- **Logging**
    - All logging is **disabled by default**.
    - To enable logs, set a log level using the `RUST_LOG` environment variable:
        ```bash
        RUST_LOG=DEBUG cargo run
        ```
    - For more details, refer to
      the [env_logger documentation](https://docs.rs/env_logger/latest/env_logger/#enabling-logging).
