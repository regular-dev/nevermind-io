use std::{
    collections::BTreeMap,
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    thread::{self, JoinHandle},
};

use nevermind_neu::{dataloader::*, models::*, orchestra::*, util::DataVec};
use nnio_common::*;
use tokio::sync::mpsc;
use tokio::{sync::Mutex, task};
use serde_json::json;

pub enum ModelMessage {
    // requests
    Train(Vec<LabeledEntry>),
    Eval(Vec<DataVec>),
    Save(String), // version
    SetBatchSize(usize),
    Info, // model name

    // response
    ModelName(String),
    TrainResult(HashMap<String, f64>),
    EvalResult(Vec<DataVec>),
    RespInfo(String),
    Test,
    Stop,
}

struct LocalConnection {
    recver: mpsc::Receiver<ModelMessage>,
    sender: mpsc::Sender<ModelMessage>,
    handle: std::thread::JoinHandle<()>,
}

//pub type LocalConnection<T> = (mpsc::Receiver<T>, mpsc::Sender<T>);
pub type MutexedModelStorage = Arc<Mutex<ModelStorage>>;

#[derive(Default)]
pub struct ModelStorage {
    mdls: BTreeMap<String, Option<LocalConnection>>,
}

impl ModelStorage {
    pub fn from_dir(dir: PathBuf) -> Self {
        debug!("Creating ModelStorage from : {}", dir.to_str().unwrap());

        if !dir.exists() {
            std::fs::create_dir_all(dir.clone())
                .expect(format!("Failed to create dir : {}", dir.to_str().unwrap()).as_str());
        }

        let mut mdls = BTreeMap::new();

        let mdl_dirs = std::fs::read_dir(dir).expect("Couldn't read directory");

        for d in mdl_dirs {
            let entry = d.unwrap();
            let entry_type = entry.file_type().unwrap();

            if entry_type.is_dir() {
                match entry.file_name().into_string() {
                    // if EN string
                    Ok(good_string) => {
                        mdls.insert(good_string, None);
                    }
                    Err(_) => {}
                }
            }
        }

        Self { mdls }
    }

    pub fn get_availabel_models(&self) -> Vec<String> {
        self.mdls.keys().cloned().collect()
    }

    pub fn get_loaded_models(&self) -> Vec<String> {
        let mut v = Vec::with_capacity(self.mdls.len());

        for i in self.mdls.iter() {
            if i.1.is_some() {
                // if net was loaded than the channel was created
                v.push(i.0.clone());
            }
        }

        v
    }

    pub async fn get_model_info(&mut self, mdl_name: &String) -> Option<String> {
        if let Some(mdl_cfg) = self.mdls.get_mut(mdl_name) { // if model config exists ?
            if let Some(mdl_con) = mdl_cfg {
                mdl_con.sender.send(ModelMessage::Info).await.unwrap();
                if let ModelMessage::RespInfo(resp) = mdl_con.recver.recv().await.unwrap() {
                    return Some(resp);
                }
            }
        }
        
        None
    }

    pub async fn unload_model(&mut self, mdl_name: &String) {
        if !self.mdls.contains_key(mdl_name) {
            warn!("Attempt to unload non-existing model : {}", mdl_name);
            return;
        }

        let con = self.mdls.remove(mdl_name);

        match con {
            Some(con) => {
                // if model available
                match con {
                    Some(con) => {
                        // if model is loaded (there is a connection)
                        con.sender.send(ModelMessage::Stop).await;
                        con.handle.join().expect("Failed to join net thread"); // TODO : handle right
                    }
                    None => {}
                }
            }
            None => {}
        };
    }

    pub async fn create_model(&mut self, net_cfg: String) -> Result<(), NnioError> {
        let (tx_host, mut rx_mdl) = mpsc::channel(20); // TODO : param must be in configuration
        let (tx_mdl, mut rx_host) = mpsc::channel(20);

        let handle = std::thread::spawn(move || {
            debug!("Creating model with yaml cfg : {}", net_cfg);

            let mut orc = Orchestra::new(Sequential::from_yaml(net_cfg.as_str()).expect("Model yaml parse error"));

            tx_mdl
                .blocking_send(ModelMessage::ModelName(orc.name.clone()))
                .unwrap();

            while let Some(msg) = rx_mdl.blocking_recv() {
                match msg {
                    ModelMessage::Train(_) => {
                        todo!("Train model")
                    }
                    ModelMessage::SetBatchSize(batch_size) => {
                        // orc.set_train_batch_size(batch_size);
                    }
                    ModelMessage::Eval(_) => {
                        todo!("Eval")
                    }
                    ModelMessage::Info => {
                        let mdl = orc.train_model().unwrap();
                        let mut out = String::with_capacity(mdl.layers_count() * 2);

                        for l in 0..mdl.layers_count() {
                            out += format!("{}-", mdl.layer(l).size()).as_str();
                        }

                        out.pop();

                        tx_mdl.blocking_send(ModelMessage::RespInfo(out)).unwrap();
                    }
                    ModelMessage::Stop => {
                        debug!("Stopping model {}...", orc.name);
                        break;
                    }
                    _ => {
                        continue;
                    }
                }
            }
        });

        let mdl_name = rx_host.recv().await.unwrap();

        if let ModelMessage::ModelName(mdl_name) = mdl_name {
            self.mdls.insert(
                mdl_name,
                Some(LocalConnection {
                    recver: rx_host,
                    sender: tx_host,
                    handle,
                }),
            );
            Ok(())
        } else {
            Err(NnioError::CustomError(
                "Invalid net initialization".to_owned(),
            ))
        }
    }
}
