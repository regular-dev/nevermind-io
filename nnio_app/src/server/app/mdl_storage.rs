use std::{
    collections::BTreeMap,
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    thread::{self, JoinHandle},
    error::Error
};

use nevermind_neu::{dataloader::*, models::*, orchestra::*, util::DataVec};
use nnio_common::*;
use serde_json::json;
use tokio::{sync::mpsc, io::AsyncWriteExt};
use tokio::{sync::Mutex, task};

use crate::app::App;

pub enum ModelMessage {
    // requests
    Train(Vec<LabeledEntry>),
    Eval(Vec<DataVec>),
    SaveCfg, // version
    SaveState(String),
    SetBatchSize(usize),
    Info, // model name

    // response
    ModelName(String),
    TrainResult(HashMap<String, f64>),
    EvalResult(Vec<DataVec>),
    RespInfo(String),
    RespSave(bool),
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
        if let Some(mdl_cfg) = self.mdls.get_mut(mdl_name) {
            // if model config exists ?
            if let Some(mdl_con) = mdl_cfg {
                mdl_con.sender.send(ModelMessage::Info).await.unwrap();
                if let ModelMessage::RespInfo(resp) = mdl_con.recver.recv().await.unwrap() {
                    return Some(resp);
                }
            }
        }

        None
    }

    pub async fn save_model_cfg(&mut self, mdl_name: &String) -> Result<bool, NnioError> {
        if let Some(mdl_cfg) = self.mdls.get_mut(mdl_name) {
            // if model available
            if let Some(mdl_con) = mdl_cfg {
                mdl_con.sender.send(ModelMessage::SaveCfg).await.unwrap();
                if let ModelMessage::RespSave(status) = mdl_con.recver.recv().await.unwrap() {
                    return Ok(status);
                } else {
                    return Err(NnioError::ModelCommunication);
                }
            } else {
                debug!("Trying to save not loaded model");
                return Err(NnioError::ModelNotLoaded);
            }
        } else {
            debug!("Trying to save non-existsing model {}", mdl_name);
            return Err(NnioError::ModelNotExists);
        }
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
                        con.sender.send(ModelMessage::Stop).await.unwrap();
                        con.handle.join().expect("Failed to join net thread"); // TODO : handle right
                    }
                    None => {}
                }
            }
            None => {}
        };
    }

    pub async fn load_model(&mut self, mdl_name: String) -> Result<(), NnioError> {
        if let Some(con) = self.mdls.get_mut(&mdl_name) { // if model is available
            if let Some(_) = con {
                warn!("Trying to load a loaded {} model", mdl_name);
                return Err(NnioError::ModelAlreadyLoaded);
            } else {
                let mut cfgfile = App::get_app_dir();
                cfgfile.push("models");
                cfgfile.push(mdl_name.clone());
                cfgfile.push("mdl.cfg");

                let mdl_yaml = tokio::fs::read_to_string(cfgfile).await.unwrap();

                debug!("Readed {} model yaml", mdl_name);

                let (tx_host, mut rx_mdl) = mpsc::channel(20); // TODO : param must be in configuration
                let (tx_mdl, rx_host) = mpsc::channel(20);

                info!("Loading {} model...", mdl_name);

                let handle = std::thread::spawn(move || {
                    debug!("Creating model with yaml cfg : {}", mdl_yaml);

                    let mut orc = Orchestra::new(
                        Sequential::from_yaml(mdl_yaml.as_str()).expect("Model yaml parse error"),
                    );

                    orc.name = mdl_name;

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
                            ModelMessage::SaveCfg => {
                                let mut app_path = App::get_app_dir();
                                app_path.push("models/");
                                app_path.push(orc.name.clone());

                                std::fs::create_dir_all(app_path.clone())
                                    .expect("Failed to create model dir");

                                app_path.push("net.cfg");

                                let train_model = orc.train_model().expect("No train model");

                                if let Ok(_) = train_model.to_file(app_path.to_str().unwrap()) {
                                    tx_mdl.blocking_send(ModelMessage::RespSave(true)).unwrap();
                                } else {
                                    tx_mdl.blocking_send(ModelMessage::RespSave(false)).unwrap();
                                }
                            }
                            _ => {
                                continue;
                            }
                        }
                    }
                });

                *con = Some(LocalConnection {
                    recver: rx_host,
                    sender: tx_host,
                    handle,
                });
            }
        }

        Ok(())
    }

    pub async fn create_model(
        &mut self,
        net_cfg: String,
        mdl_name: String,
        overwrite: bool,
    ) -> Result<(), NnioError> {
        // write yaml config to folder-file
        // create an entry
        let mut cfgfile = App::get_app_dir();
        cfgfile.push("models");
        cfgfile.push(mdl_name.clone());

        if !tokio::fs::try_exists(cfgfile.clone()).await.unwrap() { // model directory exists ?
            tokio::fs::create_dir_all(cfgfile.clone()).await.unwrap();
        }

        cfgfile.push("mdl.cfg");

        if self.mdls.contains_key(&mdl_name) {
            if !overwrite {
                return Err(NnioError::ModelAlreadyExists);
            } else {
                if tokio::fs::try_exists(cfgfile.clone()).await.unwrap() {
                    tokio::fs::remove_file(cfgfile.clone()).await.unwrap();
                }

                // TODO : impl rewrite model (stop -> delete file -> ...)
            }
        }

        let mut file = tokio::fs::File::create(cfgfile).await.unwrap();
        file.write(net_cfg.as_bytes()).await.unwrap();

        self.mdls.insert(mdl_name, None);

        Ok(())
    }
}
