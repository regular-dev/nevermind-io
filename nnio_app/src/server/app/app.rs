use std::{
    error::Error,
    fs::File,
    io::prelude::*,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::mdl_storage::*;

use directories::*;
use serde::{Deserialize, Serialize};
use serde_yaml::from_reader;

use nevermind_neu::{
    models::{Model, Sequential},
    *,
};

use tokio::sync::Mutex;

use super::mdl_storage;

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub net_port: u16,
    pub net_ip: String,
    pub max_con: i32,
}

impl Configuration {
    fn from_file(filepath: &Path) -> Result<Self, Box<dyn Error>> {
        let cfg_file = File::open(filepath)?;
        let cfg: Configuration = serde_yaml::from_reader(cfg_file)?;
        Ok(cfg)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            net_port: 5569,
            net_ip: String::from("127.0.0.1"),
            max_con: 5,
        }
    }
}

pub struct App {
    pub cfg: Configuration,
    mdls: MutexedModelStorage,
}

impl Drop for App {
    fn drop(&mut self) {
        self.save_config(&App::get_config_path()).unwrap();
    }
}

impl Default for App {
    fn default() -> Self {
        App {
            cfg: Configuration::default(),
            mdls: MutexedModelStorage::default(),
        }
    }
}

impl App {
    pub fn get_config_path() -> PathBuf {
        let base_dirs = BaseDirs::new().expect("Couldn't retries directories info");
        let user_data_dir = base_dirs.data_local_dir();

        let mut app_dir = PathBuf::new();
        app_dir.push(user_data_dir);
        app_dir.push("nnio");

        let mut app_cfg_path = app_dir.clone();
        app_cfg_path.push("server.cfg");

        return app_cfg_path;
    }

    pub fn get_app_dir() -> PathBuf {
        let base_dirs = BaseDirs::new().expect("Couldn't retries directories info");
        let user_data_dir = base_dirs.data_local_dir();

        let mut app_dir = PathBuf::new();
        app_dir.push(user_data_dir);
        app_dir.push("nnio");

        return app_dir;
    }

    pub fn from_config_or_default() -> Self {
        let cfg = Configuration::from_file(&App::get_config_path());

        match cfg {
            Ok(cfg) => {
                debug!("Loaded configuration from file!");
                return App::from_config(cfg);
            }
            Err(_) => {
                debug!("Booting from default configuration!");
                return App::from_config(Configuration::default());
            }
        }
    }

    pub fn from_config(cfg: Configuration) -> Self {
        let mut app_dir = App::get_app_dir();
        app_dir.push("models");
        // Some initialization could be done here
        Self {
            cfg,
            mdls: Arc::new(Mutex::new(ModelStorage::from_dir(app_dir))),
        }
    }

    pub fn save_config(&mut self, filepath: &Path) -> Result<(), Box<dyn Error>> {
        let app_dir = App::get_app_dir();

        if !app_dir.exists() {
            std::fs::create_dir(app_dir).expect("Couldn't create application directory");
        }

        let mut f = File::create(filepath)?;
        let cfg_data = serde_yaml::to_string(&self.cfg)?;
        f.write_all(cfg_data.as_bytes())?;

        Ok(())
    }

    pub fn clone_model_storage(&self) -> MutexedModelStorage {
        self.mdls.clone()
    }
}
