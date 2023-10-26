use dialoguer::console::Color;
use dialoguer::{theme::ColorfulTheme, Input, Select};

use std::{fs, str, string};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use serde_json::{json, Number, Value};

use strum::IntoEnumIterator;

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

    let mut cmds_v = Vec::with_capacity(15);
    for i in MessageType::iter() {
        let c = i.to_string();

        if !c.contains("Resp") {
            cmds_v.push(c);
        }
    }

    let mut buffer = [0; 8192];

    loop {
        let cmd = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose command")
            .default(0)
            .items(&cmds_v[..])
            .interact()
            .unwrap();

        if cmd == MessageType::LoadModel as usize {
            break;
        } else if cmd == MessageType::GetAvailableModels as usize {
            let msg_req = json!({
                "type": MessageType::GetAvailableModels as usize,
            });

            stream
                .write_all(msg_req.to_string().as_bytes())
                .await
                .unwrap();

            if let Ok(bytes_read) = stream.read(&mut buffer).await {
                if let Ok(json_recv) = serde_json::from_slice::<Value>(&buffer[0..bytes_read]) {
                    if let Some(json_obj) = json_recv.as_object() {
                        if let Value::Array(list_mdls) = json_obj.get("available_mdls").unwrap() {
                            println!("Available models : ");
                            for (idx, i) in list_mdls.iter().enumerate() {
                                println!("{} : {}", idx, i.as_str().unwrap());
                            }
                        }
                    }
                }
            }
        } else if cmd == MessageType::ModelInfo as usize {
            let mdl_name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter loaded model name")
                .interact_text()
                .unwrap();

            let msg_req = json!({
                "type": MessageType::ModelInfo as usize,
                "mdl_name": mdl_name,
            });

            stream
                .write_all(msg_req.to_string().as_bytes())
                .await
                .unwrap();

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
        } else if cmd == MessageType::CreateModel as usize {
            let net_cfg_filepath: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Input filepath of network configuration")
                .interact_text()
                .unwrap();

            let mdl_name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Input model name")
                .interact_text()
                .unwrap();

            let cfg_file = fs::read_to_string(net_cfg_filepath).expect("Failed to read file");

            let msg = json!({
                "type": MessageType::CreateModel as usize,
                "net_cfg": cfg_file,
                "name": mdl_name,
            });

            let msg_serialized = serde_json::to_string_pretty(&msg).unwrap();
            std::fs::write("debug.cfg", msg_serialized.clone()).unwrap();

            stream.write_all(msg_serialized.as_bytes()).await.unwrap();

            if let Ok(bytes_read) = stream.read(&mut buffer).await {
                if let Ok(json_recv) = serde_json::from_slice(&buffer[0..bytes_read]) {
                    if let Value::Object(m) = json_recv {
                        let msg_resp = m.get("type").unwrap();

                        if let Value::Number(msg_resp) = msg_resp {
                            if msg_resp.as_u64().unwrap()
                                == MessageType::RespModelCreateSuccess as u64
                            {
                                info!("Resp: model created successfully");
                            }
                        } else {
                            warn!("Resp: model creation failure");
                        }
                    }
                }
            }
        } else if cmd == MessageType::SaveModelCfg as usize {
            let mdl_name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter model name")
                .interact_text()
                .unwrap();

            let msg = json!({
                "type": MessageType::SaveModelCfg as usize,
                "mdl_name": mdl_name,
            });

            stream.write_all(msg.to_string().as_bytes()).await.unwrap();

            if let Ok(bytes_read) = stream.read(&mut buffer).await {
                let json_recv: Value =
                    serde_json::from_slice(&buffer[0..bytes_read]).expect("Failed to parse json");

                if let Value::Object(m) = json_recv {
                    let msg_resp = m.get("type").unwrap().as_number().unwrap();

                    if msg_resp.as_u64().unwrap() == MessageType::RespModelSaveCfg as u64 {
                        let msg_status = m.get("status").unwrap().as_u64().unwrap();

                        if msg_status == 0 {
                            info!("Model {} cfg saved !", mdl_name);
                        }
                    }
                }
            }
        } else if cmd == MessageType::Exit as usize {
            info!("Exiting...");
            break;
        }
    }
}
