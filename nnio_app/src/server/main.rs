pub mod app;

use std::{error::Error, thread};

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

use app::*;

async fn handle_shutdown(shutdown_tx: mpsc::Sender<()>) -> Result<(), Box<dyn Error>> {
    let mut sigint = signal(SignalKind::interrupt())?;
    sigint.recv().await;
    shutdown_tx.send(()).await.unwrap();

    Ok(())
}

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Initialized");

    let (cancel_tx, cancel_rx) = mpsc::channel(1);

    tokio::spawn(async move {
        if let Err(err) = handle_shutdown(cancel_tx).await {
            error!("Error during shutdown: {}", err);
        }
    });

    let mut listener = Listener::new(App::from_config_or_default());
    listener.run(cancel_rx).await;

    Ok(())
}
