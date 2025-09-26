//! Error types for the native executor

use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during task execution
#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("Task execution failed: {task_id}")]
    TaskExecutionFailed {
        task_id: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Dependency cycle detected in DAG")]
    DependencyCycle,

    #[error("Task timeout: {task_id} exceeded {timeout:?}")]
    TaskTimeout { task_id: String, timeout: Duration },

    #[error("Resource limit exceeded: {resource}")]
    ResourceLimitExceeded { resource: String },

    #[error("State persistence error")]
    StatePersistenceError(#[from] sqlx::Error),

    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Unsupported task type: {task_type}")]
    UnsupportedTaskType { task_type: String },

    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: String },

    #[error("Execution context error: {message}")]
    ExecutionContextError { message: String },

    #[error("Retry limit exceeded for task: {task_id}")]
    RetryLimitExceeded { task_id: String, attempts: u32 },

    #[error("Semaphore acquisition failed")]
    SemaphoreError(#[from] tokio::sync::AcquireError),

    #[error("IO error: {message}")]
    IoError {
        message: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Serialization error")]
    SerializationError(#[from] serde_json::Error),

    #[error("Task registry error: {message}")]
    TaskRegistryError { message: String },
}

/// Result type for executor operations
pub type ExecutorResult<T> = Result<T, ExecutorError>;

impl From<std::io::Error> for ExecutorError {
    fn from(error: std::io::Error) -> Self {
        ExecutorError::IoError {
            message: error.to_string(),
            source: error,
        }
    }
}

impl ExecutorError {
    /// Check if this error should trigger a retry
    pub fn should_retry(&self) -> bool {
        matches!(
            self,
            ExecutorError::TaskTimeout { .. }
                | ExecutorError::ResourceLimitExceeded { .. }
                | ExecutorError::IoError { .. }
                | ExecutorError::TaskExecutionFailed { .. }
        )
    }

    /// Get the task ID associated with this error, if any
    pub fn task_id(&self) -> Option<&str> {
        match self {
            ExecutorError::TaskExecutionFailed { task_id, .. } => Some(task_id),
            ExecutorError::TaskTimeout { task_id, .. } => Some(task_id),
            ExecutorError::TaskNotFound { task_id } => Some(task_id),
            ExecutorError::RetryLimitExceeded { task_id, .. } => Some(task_id),
            _ => None,
        }
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        !matches!(
            self,
            ExecutorError::DependencyCycle
                | ExecutorError::UnsupportedTaskType { .. }
                | ExecutorError::ConfigurationError { .. }
                | ExecutorError::SerializationError(_)
        )
    }
}
