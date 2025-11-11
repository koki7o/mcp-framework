use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Invalid params: {0}")]
    InvalidParams(String),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Request error: {0}")]
    RequestError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("LLM error: {0}")]
    LLMError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl Error {
    pub fn error_code(&self) -> i64 {
        match self {
            Error::InvalidRequest(_) => -32600,
            Error::InvalidParams(_) => -32602,
            Error::MethodNotFound(_) => -32601,
            Error::InternalError(_) => -32603,
            Error::ServerError(_) => -32000,
            Error::ToolNotFound(_) => -32001,
            Error::ResourceNotFound(_) => -32002,
            Error::SerializationError(_) => -32603,
            Error::RequestError(_) => -32603,
            Error::Timeout => -32604,
            Error::ConnectionError(_) => -32605,
            Error::LLMError(_) => -32606,
            Error::Unknown(_) => -32603,
        }
    }
}
