use strum_macros::EnumIter;

#[derive(Debug, EnumIter)]
pub enum MessageType {
    GetAvailableModels,
    GetLoadedModels,
    ModelInfo,
    CreateModel,
    DeleteModel,
    LoadModel,
    UnloadModel,
    SaveModelCfg,
    SaveModelState,
    TrainModel,
    EvaluateData,
    Exit,

    // Response
    RespModelCreateSuccess,
    RespModelCreateFailure,
    RespModelInfoSuccess,
    RespModelInfoFailure,
    RespAvailableModels,
    RespModelInfo,
    RespModelSaveCfg,
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<&str> for MessageType {
    type Error = Box<dyn std::error::Error + Send>;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        if value == MessageType::GetAvailableModels.to_string() {
            return Ok(MessageType::GetAvailableModels);
        } else if value == MessageType::GetAvailableModels.to_string() {
            return Ok(MessageType::GetLoadedModels);
        } else if value == MessageType::ModelInfo.to_string() {
            return Ok(MessageType::ModelInfo);
        } else if value == MessageType::CreateModel.to_string() {
            return Ok(MessageType::CreateModel);
        } else if value == MessageType::DeleteModel.to_string() {
            return Ok(MessageType::DeleteModel);
        } else if value == MessageType::TrainModel.to_string() {
            return Ok(MessageType::TrainModel);
        } else if value == MessageType::EvaluateData.to_string() {
            return Ok(MessageType::EvaluateData);
        } else if value == MessageType::UnloadModel.to_string() {
            return Ok(MessageType::UnloadModel);
        } else if value == MessageType::LoadModel.to_string() {
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
        } else if value == MessageType::SaveModelCfg as u64 {
            return Ok(MessageType::SaveModelCfg);
        } else if value == MessageType::SaveModelState as u64 {
            return Ok(MessageType::SaveModelState);
        }  else {
            todo!("error");
        }
    }
}

use std::fmt;

#[derive(Debug)]
pub enum NnioError {
    ModelNotExists,
    ModelAlreadyExists,
    ModelCommunication,
    ModelNotLoaded,
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

