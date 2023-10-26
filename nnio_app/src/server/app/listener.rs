use nevermind_neu::{models::Sequential, orchestra::Orchestra};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::{
    io::*,
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

use serde_json::{json, Value};

use crate::app::*;
use nnio_common::*;

pub struct Listener {
    app: App,
    is_running: AtomicBool,
}

impl Listener {
    pub fn new(app: App) -> Self {
        Self {
            app,
            is_running: AtomicBool::new(false),
        }
    }

    pub async fn run(&mut self, mut shutdown_rx: mpsc::Receiver<()>) {
        if self.is_running.load(Ordering::SeqCst) {
            return;
        }

        let addr = format!("{}:{}", self.app.cfg.net_ip, self.app.cfg.net_port);

        let listener = TcpListener::bind(addr.clone()).await.unwrap();
        info!("Server is listening on {}", addr);

        loop {
            tokio::select! {
                vals = listener.accept() => {
                    let (socket, _) = vals.unwrap();

                    let mdls = self.app.clone_model_storage();

                    tokio::spawn(async move {
                        Listener::handle_client(socket, mdls).await;
                    });
                },
                _ = shutdown_rx.recv() => {
                    info!("Graceful shutdown");
                    break;
                }
            }
        }
    }

    // TODO : splitup each enum entry per func, increase readability
    async fn handle_client(mut stream: TcpStream, mdls: MutexedModelStorage) {
        let mut buffer = [0; 8192];
        while let Ok(bytes_read) = stream.read(&mut buffer).await {
            if bytes_read == 0 {
                // Connection closed
                break;
            }

            // Process the received data
            let input_slice = &buffer[..bytes_read];
            let mut json_msg: Value = serde_json::from_slice(input_slice).unwrap();

            if !json_msg.is_object() {
                continue;
            }

            let json_obj = json_msg.as_object_mut().unwrap();

            if let Some(str_msg_type) = json_obj.get("type").unwrap().as_u64() {
                let msg_type_res = MessageType::try_from(str_msg_type);

                debug!("Received message : {}", str_msg_type);

                if let Ok(msg_type) = msg_type_res {
                    match msg_type {
                        MessageType::CreateModel => {
                            debug!("in create model");
                            let mut lock = mdls.lock().await;

                            let mdl_name = json_obj
                                .remove("name")
                                .unwrap()
                                .as_str()
                                .unwrap()
                                .to_owned();

                            if mdl_name.is_empty() {
                                warn!("Received model name is empty, ignoring...");
                                continue;
                            }

                            if let Value::String(net_cfg) = json_obj.remove("net_cfg").unwrap() {
                                // TODO : handle rewrite flag (third arg)
                                if let Ok(_) = lock.create_model(net_cfg, mdl_name, false).await {
                                    let resp = json!({
                                        "type": MessageType::RespModelCreateSuccess as usize
                                    });

                                    let resp_ser = serde_json::to_string(&resp).unwrap();
                                    stream.write_all(resp_ser.as_bytes()).await.unwrap();
                                } else {
                                    let resp = json!({
                                        "type": MessageType::RespModelCreateFailure as usize
                                    });

                                    let resp_ser = serde_json::to_string(&resp).unwrap();
                                    stream.write_all(resp_ser.as_bytes()).await.unwrap();
                                }
                            }
                        }
                        MessageType::DeleteModel => {}
                        MessageType::GetAvailableModels => {
                            let lock = mdls.lock().await;
                            let out = lock.get_availabel_models();

                            let json_mdls =
                                out.iter().map(|s| json!(s)).collect::<serde_json::Value>();

                            let json_resp = json!({
                                "type": MessageType::RespAvailableModels as usize,
                                "available_mdls": json_mdls,
                            });

                            stream
                                .write_all(json_resp.to_string().as_bytes())
                                .await
                                .unwrap();
                        }
                        MessageType::GetLoadedModels => {
                            let lock = mdls.lock().await;
                            let out = lock.get_loaded_models();

                            let json_mdls =
                                out.iter().map(|s| json!(s)).collect::<serde_json::Value>();

                            let json_resp = json!({
                                "type": MessageType::RespLoadedModels as usize,
                                "loaded_mdls": json_mdls
                            });

                            stream
                                .write_all(json_resp.to_string().as_bytes())
                                .await
                                .unwrap();
                        }
                        MessageType::UnloadModel => {
                            let mut lock = mdls.lock().await;
                        }
                        MessageType::LoadModel => {
                            Listener::handle_load_model(&mut stream, mdls.clone(), json_obj).await;
                        }
                        MessageType::TrainModel => {}
                        MessageType::ModelInfo => {
                            if let Value::String(mdl_name) = json_obj.get("mdl_name").unwrap() {
                                let mut lock = mdls.lock().await;
                                if let Some(mdl_info) = lock.get_model_info(mdl_name).await {
                                    let json_resp = json!({
                                        "type": MessageType::RespModelInfoSuccess as usize,
                                        "mdl_info": mdl_info
                                    });

                                    stream
                                        .write_all(json_resp.to_string().as_bytes())
                                        .await
                                        .unwrap();
                                } else {
                                    let json_resp = json!({
                                        "type": MessageType::RespModelCreateFailure as usize,
                                    });

                                    stream
                                        .write_all(json_resp.to_string().as_bytes())
                                        .await
                                        .unwrap();
                                }
                            }
                        }
                        MessageType::SaveModelCfg => {
                            let mdl_name = json_obj
                                .get("mdl_name")
                                .unwrap()
                                .as_str()
                                .unwrap()
                                .to_owned();

                            debug!("Trying to save cfg of model {}", mdl_name);

                            let mut lock = mdls.lock().await;

                            let res = lock.save_model_cfg(&mdl_name).await;

                            match res {
                                Ok(status) => {
                                    let json_resp = json!({
                                        "type": MessageType::RespModelSaveCfg as usize,
                                        "status": status as usize,
                                    });

                                    stream
                                        .write_all(json_resp.to_string().as_bytes())
                                        .await
                                        .unwrap();
                                }
                                Err(_) => {
                                    // TODO : handle err type
                                    let json_resp = json!({
                                        "type": MessageType::RespModelSaveCfg as usize,
                                        "status": 0,
                                    });

                                    stream
                                        .write_all(json_resp.to_string().as_bytes())
                                        .await
                                        .unwrap();
                                }
                            };
                        }
                        MessageType::EvaluateData => {}
                        _ => {
                            todo!()
                        }
                    }
                }
            }

            // Echo the data back to the client
            // stream.write_all(received_data).unwrap();
        }
    }

    async fn handle_load_model(
        stream: &mut TcpStream,
        mdls: MutexedModelStorage,
        json_obj: &mut serde_json::Map<String, Value>,
    ) {
        let mut lock = mdls.lock().await;
        let mdl_name = json_obj
            .get("mdl_name")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();

        if let Ok(_) = lock.load_model(mdl_name).await {
            stream
                .write_all(
                    json!({
                        "type": MessageType::RespLoadModel as usize,
                        "status": 1,
                    })
                    .to_string()
                    .as_bytes(),
                )
                .await;
        } else {
            stream
                .write_all(
                    json!({
                        "type": MessageType::RespLoadModel as usize,
                        "status": 0
                    })
                    .to_string()
                    .as_bytes(),
                )
                .await;
        }
    }
}

#[derive(Default)]
struct ConnectionState {
    pub orc: Option<Orchestra<Sequential>>,
    // orc_ocl ...
}

impl ConnectionState {}
