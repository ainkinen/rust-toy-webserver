use log::{debug, info};
use std::io;
use std::time::Duration;
use tokio::signal;
use tokio_util::sync::CancellationToken;

use rust_toy_webserver::Server;

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    let shutdown_token = CancellationToken::new();
    let server = Server::new().await?;
    let server_handle = tokio::spawn(server.run(shutdown_token.clone()));

    let shutdown_handle = tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        info!("Ctrl-c received. Shutting down.");
        shutdown_token.cancel();
        tokio::time::sleep(Duration::from_secs(10)).await; // Graceful shutdown timeout
        debug!("Shutdown timeout ran out.")
    });

    tokio::select! {
        _ = shutdown_handle => {}
        _ = server_handle => {
            debug!("Server task finished cleanly.")
        },
    }

    info!("Exiting...");
    Ok(())
}
