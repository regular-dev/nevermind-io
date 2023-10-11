pub mod app;

use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};

use app::*;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Initialized");
    let mut listener = Listener::new(App::from_config_or_default());
    listener.run().await;
}
