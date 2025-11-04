use thiserror::Error;

#[derive(Debug, Error)]
pub enum JormError {
    #[error("DAG parsing error: {0}")]
    ParseError(String),

    #[error("Task execution failed: {0}")]
    ExecutionError(String),

    #[error("Natural language processing error: {0}")]
    NlpError(String),

    #[error("File operation error: {0}")]
    FileError(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Scheduler error: {0}")]
    SchedulerError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Server error: {0}")]
    ServerError(String),
}
