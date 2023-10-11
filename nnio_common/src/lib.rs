pub enum MessageType {
    GetAvailableModels,
    GetLoadedModels,
    ModelInfo,
    CreateModel,
    DeleteModel,
    LoadModel,
    UnloadModel,
    SaveModel,
    TrainModel,
    EvaluateData,

    // Response
    RespModelCreateSuccess,
    RespModelCreateFailure,
    RespModelInfoSuccess,
    RespModelInfoFailure,
    RespModelInfo,
}

impl TryFrom<&str> for MessageType {
    type Error = Box<dyn std::error::Error + Send>;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        if value == "get_available_models" {
            return Ok(MessageType::GetAvailableModels);
        } else if value == "get_loaded_models" {
            return Ok(MessageType::GetLoadedModels);
        } else if value == "model_info" {
            return Ok(MessageType::ModelInfo);
        } else if value == "create_model" {
            return Ok(MessageType::CreateModel);
        } else if value == "delete_model" {
            return Ok(MessageType::DeleteModel);
        } else if value == "train_model" {
            return Ok(MessageType::TrainModel);
        } else if value == "eval_model" {
            return Ok(MessageType::EvaluateData);
        } else if value == "reset_model" {
            return Ok(MessageType::UnloadModel);
        } else if value == "load_model" {
            return Ok(MessageType::LoadModel);
        } else {
            todo!("error");
        }
    }
}

impl TryFrom<u64> for MessageType {
    type Error = Box<dyn std::error::Error + Send>;

    fn try_from(value: u64) -> std::result::Result<Self, Self::Error> {
        if value == MessageType::GetAvailableModels as u64 {
            return Ok(MessageType::GetAvailableModels);
        } else if value == MessageType::GetLoadedModels as u64 {
            return Ok(MessageType::GetLoadedModels);
        } else if value == MessageType::ModelInfo as u64 {
            return Ok(MessageType::ModelInfo);
        } else if value == MessageType::CreateModel as u64 {
            return Ok(MessageType::CreateModel);
        } else if value == MessageType::DeleteModel as u64  {
            return Ok(MessageType::DeleteModel);
        } else if value == MessageType::TrainModel as u64 {
            return Ok(MessageType::TrainModel);
        } else if value == MessageType::EvaluateData as u64 {
            return Ok(MessageType::EvaluateData);
        } else if value == MessageType::UnloadModel as u64 {
            return Ok(MessageType::UnloadModel);
        } else if value == MessageType::LoadModel as u64 {
            return Ok(MessageType::LoadModel);
        } else {
            todo!("error");
        }
    }
}

use std::fmt;

#[derive(Debug)]
pub enum NnioError {
    CustomError(String),
}

impl fmt::Display for NnioError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NnioError::CustomError(msg) => {
                write!(f, "Custom Error : {}", msg)
            },
            _ => {
                write!(f, "{}", "Other")
            },
        }
    }
}

