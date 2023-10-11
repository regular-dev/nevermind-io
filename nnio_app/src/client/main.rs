use dialoguer::{theme::ColorfulTheme, Input, Select};

use std::{fs, str};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use serde_json::{json, Value, Number};

use nnio_common::*;

#[macro_use]
extern crate log;

use log::debug;

use std::str::FromStr;

#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Welcome to nevermind_io client !");

    let server_addr: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Input server ip address and port like 127.0.0.1:5569")
        .with_initial_text("127.0.0.1:5569")
        .interact_text()
        .unwrap();

    let addr_split: Vec<&str> = server_addr.split(':').collect();

    if addr_split.len() != 2 {
        error!("Invalid address");
        return;
    }

    let server_ip = String::from(addr_split[0]);
    let server_port: u16 = u16::from_str(addr_split[1]).expect("Invalid port number");

    let mut stream = TcpStream::connect(server_addr)
        .await
        .expect("Couldn't connect to server");

    let cmds = &[
        "get_available_models", // 0
        "get_loaded_models",    // 1
        "model_info",           // 2
        "create_model",         // 3
        "delete_model",         // 4
        "load_model",           // 5
        "unload_model",         // 6
        "save_model",           // 7
        "train_model",          // 8
        "evaluate_data",        // 9
        "exit",                 // 10
    ];

    let mut buffer = [0; 8192];

    loop {
        let cmd = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose command")
            .default(0)
            .items(&cmds[..])
            .interact()
            .unwrap();

        if cmd == MessageType::LoadModel as usize {
            break;
        }

        if cmd == MessageType::ModelInfo as usize {
            let mdl_name: String = Input::with_theme(&ColorfulTheme::default()).with_prompt("Enter loaded model name").interact_text().unwrap();

            let msg_req = json!({
                "type": MessageType::ModelInfo as usize,
                "mdl_name": mdl_name,
            });

            stream.write_all(msg_req.to_string().as_bytes()).await.unwrap();

            if let Ok(bytes_read) = stream.read(&mut buffer).await {
                let json_recv: Value =
                    serde_json::from_slice(&buffer[0..bytes_read]).expect("Failed to parse json");

                if let Some(json_obj) = json_recv.as_object() {
                    if !json_obj.contains_key("mdl_info") {
                        warn!("Failed to retrieve model {} info!", mdl_name);
                        continue;
                    }

                    if let Value::String(mdl_info) = json_obj.get("mdl_info").unwrap() {
                        info!("Model {} | Info : {}", mdl_name, mdl_info);
                    }
                }
            }
        }

        if cmd == MessageType::CreateModel as usize {
            let net_cfg_filepath: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Input filepath of network configuration")
                .interact_text()
                .unwrap();

            let cfg_file = fs::read_to_string(net_cfg_filepath).expect("Failed to read file");

            let msg = json!({
                "type": MessageType::CreateModel as usize,
                "net_cfg": cfg_file,
            });

            let msg_serialized = serde_json::to_string_pretty(&msg).unwrap();
            std::fs::write("debug.cfg", msg_serialized.clone()).unwrap();

            stream.write_all(msg_serialized.as_bytes()).await.unwrap();

            if let Ok(bytes_read) = stream.read(&mut buffer).await {
                let json_recv: Value =
                    serde_json::from_slice(&buffer[0..bytes_read]).expect("Failed to parse json");

                if let Value::Object(m) = json_recv {
                    let msg_resp = m.get("type").unwrap();

                    if let Value::Number(msg_resp) = msg_resp {
                        if msg_resp.as_u64().unwrap() == MessageType::RespModelCreateSuccess as u64 {
                            info!("Resp: model created successfully");
                        }
                    } else {
                        warn!("Resp: model creation failure");
                    }
                }
            }
        }
    }
}
