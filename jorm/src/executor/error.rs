//! Error types for the native executor
//!
//! This module defines comprehensive error types for the JORM native executor,
//! providing detailed error information and context for debugging and error handling.
//!
//! # Error Categories
//!
//! The executor defines several categories of errors:
//!
//! - **Execution Errors**: Task execution failures, timeouts, dependency issues
//! - **Configuration Errors**: Invalid configuration values or missing settings
//! - **Resource Errors**: Resource limit violations, system resource issues
//! - **State Errors**: State persistence and recovery failures
//! - **System Errors**: I/O errors, serialization failures, system-level issues
//!
//! # Error Handling Patterns
//!
//! ## Basic Error Handling
//!
//! ```rust
//! use jorm::executor::{NativeExecutor, ExecutorError};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = NativeExecutor::new(Default::default());
//! 
//! match executor.execute_dag(&dag).await {
//!     Ok(result) => println!("Success: {:?}", result.status),
//!     Err(ExecutorError::TaskExecutionFailed { task_id, source }) => {
//!         eprintln!("Task {} failed: {}", task_id, source);
//!     }
//!     Err(ExecutorError::DependencyCycle) => {
//!         eprintln!("DAG contains dependency cycles");
//!     }
//!     Err(e) => eprintln!("Other error: {}", e),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Error Context
//!
//! Many errors include additional context for debugging:
//!
//! ```rust
//! use jorm::executor::{ExecutorError, ErrorContext};
//!
//! fn handle_error_with_context(error: ExecutorError) {
//!     match error {
//!         ExecutorError::TaskExecutionFailed { task_id, source } => {
//!             eprintln!("Task execution failed:");
//!             eprintln!("  Task ID: {}", task_id);
//!             eprintln!("  Error: {}", source);
//!             
//!             // Additional context may be available in the source error
//!             if let Some(context) = source.downcast_ref::<ErrorContext>() {
//!                 eprintln!("  DAG: {:?}", context.dag_id);
//!                 eprintln!("  Execution: {:?}", context.execution_id);
//!                 eprintln!("  Timestamp: {}", context.timestamp);
//!             }
//!         }
//!         _ => eprintln!("Other error: {}", error),
//!     }
//! }
//! ```
//!
//! # Error Recovery
//!
//! Some errors support automatic recovery:
//!
//! ```rust
//! use jorm::executor::{ExecutorError, NativeExecutor};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = NativeExecutor::new(Default::default());
//!
//! match executor.execute_dag(&dag).await {
//!     Err(ExecutorError::RecoveryError { execution_id, .. }) => {
//!         // Attempt to resume from checkpoint
//!         if let Ok(result) = executor.resume_execution(&execution_id, &dag).await {
//!             println!("Successfully resumed execution");
//!         }
//!     }
//!     result => {
//!         // Handle other results
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during task execution
///
/// This enum covers all possible error conditions in the native executor,
/// from task execution failures to system-level errors. Each variant includes
/// relevant context information for debugging and error handling.
///
/// # Error Categories
///
/// ## Execution Errors
/// - [`TaskExecutionFailed`](ExecutorError::TaskExecutionFailed): Individual task execution failure
/// - [`TaskTimeout`](ExecutorError::TaskTimeout): Task exceeded its timeout
/// - [`DependencyCycle`](ExecutorError::DependencyCycle): Circular dependencies in DAG
/// - [`ParallelExecutionError`](ExecutorError::ParallelExecutionError): Parallel execution coordination failure
///
/// ## Configuration Errors
/// - [`ConfigurationError`](ExecutorError::ConfigurationError): Invalid configuration values
/// - [`UnsupportedTaskType`](ExecutorError::UnsupportedTaskType): Task type not supported
/// - [`ValidationError`](ExecutorError::ValidationError): Configuration validation failure
///
/// ## Resource Errors
/// - [`ResourceLimitExceeded`](ExecutorError::ResourceLimitExceeded): System resource limits exceeded
/// - [`RetryLimitExceeded`](ExecutorError::RetryLimitExceeded): Task retry attempts exhausted
///
/// ## State Management Errors
/// - [`StatePersistenceError`](ExecutorError::StatePersistenceError): Database persistence failure
/// - [`RecoveryError`](ExecutorError::RecoveryError): Execution recovery failure
///
/// ## System Errors
/// - [`IoError`](ExecutorError::IoError): File system or network I/O error
/// - [`SerializationError`](ExecutorError::SerializationError): JSON serialization/deserialization error
/// - [`SemaphoreError`](ExecutorError::SemaphoreError): Concurrency control error
///
/// # Examples
///
/// ## Handling Specific Error Types
///
/// ```rust
/// use jorm::executor::ExecutorError;
///
/// fn handle_executor_error(error: ExecutorError) {
///     match error {
///         ExecutorError::TaskExecutionFailed { task_id, source } => {
///             eprintln!("Task '{}' failed: {}", task_id, source);
///             // Maybe retry the task or mark it as failed
///         }
///         ExecutorError::TaskTimeout { task_id, timeout } => {
///             eprintln!("Task '{}' timed out after {:?}", task_id, timeout);
///             // Maybe increase timeout or optimize the task
///         }
///         ExecutorError::ResourceLimitExceeded { resource } => {
///             eprintln!("Resource limit exceeded: {}", resource);
///             // Maybe reduce concurrency or increase limits
///         }
///         _ => eprintln!("Other error: {}", error),
///     }
/// }
/// ```
///
/// ## Error Recovery Patterns
///
/// ```rust
/// use jorm::executor::{ExecutorError, NativeExecutor};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let executor = NativeExecutor::new(Default::default());
///
/// match executor.execute_dag(&dag).await {
///     Err(ExecutorError::TaskTimeout { task_id, .. }) => {
///         // Retry with longer timeout
///         println!("Retrying task {} with extended timeout", task_id);
///         // Implementation would modify task timeout and retry
///     }
///     Err(ExecutorError::ResourceLimitExceeded { .. }) => {
///         // Reduce concurrency and retry
///         println!("Reducing concurrency due to resource limits");
///         // Implementation would create new executor with lower limits
///     }
///     result => {
///         // Handle success or other errors
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// Task execution failed
    ///
    /// Indicates that a specific task failed during execution. The `source` error
    /// contains the underlying cause of the failure.
    ///
    /// # Common Causes
    /// - Command returned non-zero exit code
    /// - Network request failed
    /// - File operation failed
    /// - Python script error
    /// - Invalid task configuration
    #[error("Task execution failed: {task_id}")]
    TaskExecutionFailed {
        /// ID of the task that failed
        task_id: String,
        /// Underlying error that caused the failure
        #[source]
        source: anyhow::Error,
    },

    /// Dependency cycle detected in DAG
    ///
    /// The DAG contains circular dependencies that prevent execution.
    /// This is detected during DAG validation before execution begins.
    ///
    /// # Resolution
    /// Review the DAG definition and remove circular dependencies.
    #[error("Dependency cycle detected in DAG")]
    DependencyCycle,

    /// Task timeout exceeded
    ///
    /// A task exceeded its configured timeout and was terminated.
    ///
    /// # Resolution
    /// - Increase the task timeout
    /// - Optimize the task to run faster
    /// - Check for hanging processes
    #[error("Task timeout: {task_id} exceeded {timeout:?}")]
    TaskTimeout {
        /// ID of the task that timed out
        task_id: String,
        /// Timeout duration that was exceeded
        timeout: Duration,
    },

    /// Resource limit exceeded
    ///
    /// System resource usage exceeded configured limits, causing task throttling
    /// or execution failure.
    ///
    /// # Resolution
    /// - Increase resource limits
    /// - Reduce concurrent task count
    /// - Optimize resource-intensive tasks
    #[error("Resource limit exceeded: {resource}")]
    ResourceLimitExceeded {
        /// Name of the resource that exceeded limits (e.g., "CPU", "Memory")
        resource: String,
    },

    /// State persistence error
    ///
    /// Failed to save or load execution state from the database.
    ///
    /// # Common Causes
    /// - Database connection failure
    /// - Disk space exhaustion
    /// - Database corruption
    /// - Permission issues
    #[error("State persistence error")]
    StatePersistenceError(#[from] sqlx::Error),

    /// Configuration error
    ///
    /// Invalid or missing configuration values.
    ///
    /// # Common Causes
    /// - Invalid configuration file format
    /// - Missing required configuration
    /// - Invalid configuration values
    /// - Configuration validation failure
    #[error("Configuration error: {message}")]
    ConfigurationError {
        /// Description of the configuration error
        message: String,
    },

    /// Unsupported task type
    ///
    /// The task type is not supported by any registered task executor.
    ///
    /// # Resolution
    /// - Check task type spelling
    /// - Register a custom task executor
    /// - Use a supported task type
    #[error("Unsupported task type: {task_type}")]
    UnsupportedTaskType {
        /// The unsupported task type
        task_type: String,
    },

    /// Task not found
    ///
    /// Referenced task ID does not exist in the DAG.
    ///
    /// # Common Causes
    /// - Typo in task dependency
    /// - Task was removed but dependencies not updated
    /// - Case sensitivity issues
    #[error("Task not found: {task_id}")]
    TaskNotFound {
        /// ID of the missing task
        task_id: String,
    },

    /// Execution context error
    ///
    /// Error in setting up or managing the execution context for a task.
    ///
    /// # Common Causes
    /// - Invalid working directory
    /// - Environment variable issues
    /// - Resource allocation failure
    #[error("Execution context error: {message}")]
    ExecutionContextError {
        /// Description of the context error
        message: String,
    },

    /// Retry limit exceeded
    ///
    /// Task failed and exhausted all retry attempts.
    ///
    /// # Resolution
    /// - Increase retry limit
    /// - Fix underlying task issue
    /// - Adjust retry strategy
    #[error("Retry limit exceeded for task: {task_id}")]
    RetryLimitExceeded {
        /// ID of the task that exhausted retries
        task_id: String,
        /// Number of attempts made
        attempts: u32,
    },

    /// Semaphore acquisition failed
    ///
    /// Failed to acquire concurrency control semaphore.
    /// This typically indicates a system-level issue.
    #[error("Semaphore acquisition failed")]
    SemaphoreError(#[from] tokio::sync::AcquireError),

    /// I/O error
    ///
    /// File system or network I/O operation failed.
    ///
    /// # Common Causes
    /// - File not found
    /// - Permission denied
    /// - Network connectivity issues
    /// - Disk space exhaustion
    #[error("IO error: {message}")]
    IoError {
        /// Description of the I/O error
        message: String,
        /// Underlying I/O error
        #[source]
        source: std::io::Error,
    },

    /// Serialization error
    ///
    /// Failed to serialize or deserialize data (typically JSON).
    ///
    /// # Common Causes
    /// - Invalid JSON format
    /// - Incompatible data types
    /// - Corrupted data
    #[error("Serialization error")]
    SerializationError(#[from] serde_json::Error),

    /// Task registry error
    ///
    /// Error in task executor registration or lookup.
    ///
    /// # Common Causes
    /// - Duplicate task executor registration
    /// - Task executor initialization failure
    /// - Registry corruption
    #[error("Task registry error: {message}")]
    TaskRegistryError {
        /// Description of the registry error
        message: String,
    },

    /// Parallel execution error
    ///
    /// Error in coordinating parallel task execution.
    ///
    /// # Common Causes
    /// - Task coordination failure
    /// - Resource contention
    /// - Multiple task failures
    #[error("Parallel execution error: {message}")]
    ParallelExecutionError {
        /// Description of the parallel execution error
        message: String,
        /// List of tasks that failed during parallel execution
        failed_tasks: Vec<String>,
    },

    /// Recovery error
    ///
    /// Failed to recover execution from a checkpoint.
    ///
    /// # Common Causes
    /// - Corrupted checkpoint data
    /// - Incompatible DAG changes
    /// - Missing execution state
    #[error("Recovery error: {message}")]
    RecoveryError {
        /// Description of the recovery error
        message: String,
        /// ID of the execution that failed to recover
        execution_id: String,
    },

    /// Validation error
    ///
    /// Configuration or data validation failed.
    ///
    /// # Common Causes
    /// - Invalid field values
    /// - Missing required fields
    /// - Data type mismatches
    #[error("Validation error: {message}")]
    ValidationError {
        /// Description of the validation error
        message: String,
        /// Name of the field that failed validation
        field: String,
        /// Value that failed validation
        value: String,
    },

    /// Parameter resolution error
    ///
    /// Failed to resolve a parameter value from its source.
    ///
    /// # Common Causes
    /// - Referenced task output not found
    /// - Invalid JSON path in task data
    /// - Missing environment variable
    /// - Secure parameter not found
    #[error("Parameter resolution error for '{parameter}': {message}")]
    ParameterResolutionError {
        /// Name of the parameter that failed to resolve
        parameter: String,
        /// Description of the resolution error
        message: String,
    },

    /// Parameter validation error
    ///
    /// Parameter value failed validation rules.
    ///
    /// # Common Causes
    /// - Value doesn't match required type
    /// - Value outside allowed range
    /// - Value doesn't match pattern
    /// - Value not in allowed list
    #[error("Parameter validation error for '{parameter}': {message}")]
    ParameterValidationError {
        /// Name of the parameter that failed validation
        parameter: String,
        /// Description of the validation error
        message: String,
    },
}

/// Error context providing additional information about where and when an error occurred
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// DAG ID where the error occurred
    pub dag_id: Option<String>,
    /// Task ID where the error occurred
    pub task_id: Option<String>,
    /// Execution ID
    pub execution_id: Option<String>,
    /// Timestamp when the error occurred
    pub timestamp: DateTime<Utc>,
    /// Additional metadata about the error
    pub metadata: HashMap<String, String>,
    /// Stack trace or call path
    pub call_stack: Vec<String>,
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            dag_id: None,
            task_id: None,
            execution_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            call_stack: Vec::new(),
        }
    }
}

impl ErrorContext {
    /// Create a new error context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create error context for a specific task
    pub fn for_task(dag_id: String, task_id: String, execution_id: Option<String>) -> Self {
        Self {
            dag_id: Some(dag_id),
            task_id: Some(task_id),
            execution_id,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            call_stack: Vec::new(),
        }
    }

    /// Create error context for a DAG
    pub fn for_dag(dag_id: String, execution_id: Option<String>) -> Self {
        Self {
            dag_id: Some(dag_id),
            task_id: None,
            execution_id,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            call_stack: Vec::new(),
        }
    }

    /// Add metadata to the error context
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add a call to the stack trace
    pub fn with_call(mut self, call: String) -> Self {
        self.call_stack.push(call);
        self
    }

    /// Add multiple calls to the stack trace
    pub fn with_calls(mut self, calls: Vec<String>) -> Self {
        self.call_stack.extend(calls);
        self
    }
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

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ExecutorError::TaskExecutionFailed { .. } => ErrorSeverity::High,
            ExecutorError::DependencyCycle => ErrorSeverity::Critical,
            ExecutorError::TaskTimeout { .. } => ErrorSeverity::Medium,
            ExecutorError::ResourceLimitExceeded { .. } => ErrorSeverity::High,
            ExecutorError::StatePersistenceError(_) => ErrorSeverity::Medium,
            ExecutorError::ConfigurationError { .. } => ErrorSeverity::Critical,
            ExecutorError::UnsupportedTaskType { .. } => ErrorSeverity::High,
            ExecutorError::TaskNotFound { .. } => ErrorSeverity::High,
            ExecutorError::ExecutionContextError { .. } => ErrorSeverity::Medium,
            ExecutorError::RetryLimitExceeded { .. } => ErrorSeverity::High,
            ExecutorError::SemaphoreError(_) => ErrorSeverity::Low,
            ExecutorError::IoError { .. } => ErrorSeverity::Medium,
            ExecutorError::SerializationError(_) => ErrorSeverity::Medium,
            ExecutorError::TaskRegistryError { .. } => ErrorSeverity::High,
            ExecutorError::ParallelExecutionError { .. } => ErrorSeverity::High,
            ExecutorError::RecoveryError { .. } => ErrorSeverity::High,
            ExecutorError::ValidationError { .. } => ErrorSeverity::Medium,
            ExecutorError::ParameterResolutionError { .. } => ErrorSeverity::High,
            ExecutorError::ParameterValidationError { .. } => ErrorSeverity::Medium,
        }
    }

    /// Create a detailed error report
    pub fn to_detailed_report(&self) -> ErrorReport {
        ErrorReport {
            error_type: format!("{:?}", self).split('(').next().unwrap_or("Unknown").to_string(),
            message: self.to_string(),
            severity: self.severity(),
            task_id: self.task_id().map(|s| s.to_string()),
            timestamp: Utc::now(),
            is_recoverable: self.is_recoverable(),
            should_retry: self.should_retry(),
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Detailed error report for logging and debugging
#[derive(Debug, Clone)]
pub struct ErrorReport {
    pub error_type: String,
    pub message: String,
    pub severity: ErrorSeverity,
    pub task_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub is_recoverable: bool,
    pub should_retry: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::for_task(
            "test_dag".to_string(),
            "test_task".to_string(),
            Some("exec_123".to_string()),
        );

        assert_eq!(context.dag_id, Some("test_dag".to_string()));
        assert_eq!(context.task_id, Some("test_task".to_string()));
        assert_eq!(context.execution_id, Some("exec_123".to_string()));
        assert!(context.metadata.is_empty());
        assert!(context.call_stack.is_empty());
    }

    #[test]
    fn test_error_context_with_metadata() {
        let context = ErrorContext::new()
            .with_metadata("key1".to_string(), "value1".to_string())
            .with_metadata("key2".to_string(), "value2".to_string())
            .with_call("function_a".to_string())
            .with_call("function_b".to_string());

        assert_eq!(context.metadata.len(), 2);
        assert_eq!(context.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(context.metadata.get("key2"), Some(&"value2".to_string()));
        assert_eq!(context.call_stack.len(), 2);
        assert_eq!(context.call_stack[0], "function_a");
        assert_eq!(context.call_stack[1], "function_b");
    }

    #[test]
    fn test_executor_error_task_id_extraction() {
        let error = ExecutorError::TaskTimeout {
            task_id: "test_task".to_string(),
            timeout: Duration::from_secs(30),
        };

        assert_eq!(error.task_id(), Some("test_task"));
    }

    #[test]
    fn test_executor_error_severity() {
        let error = ExecutorError::DependencyCycle;

        assert_eq!(error.severity(), ErrorSeverity::Critical);
        assert!(!error.is_recoverable());
        assert!(!error.should_retry());

        let timeout_error = ExecutorError::TaskTimeout {
            task_id: "test_task".to_string(),
            timeout: Duration::from_secs(30),
        };

        assert_eq!(timeout_error.severity(), ErrorSeverity::Medium);
        assert!(timeout_error.is_recoverable());
        assert!(timeout_error.should_retry());
    }

    #[test]
    fn test_error_report_generation() {
        let error = ExecutorError::TaskExecutionFailed {
            task_id: "test_task".to_string(),
            source: anyhow::anyhow!("Command failed"),
        };

        let report = error.to_detailed_report();

        assert!(report.error_type.contains("TaskExecutionFailed"));
        assert_eq!(report.severity, ErrorSeverity::High);
        assert_eq!(report.task_id, Some("test_task".to_string()));
        assert!(report.is_recoverable);
        assert!(report.should_retry);
    }

    #[test]
    fn test_resource_limit_error() {
        let error = ExecutorError::ResourceLimitExceeded {
            resource: "memory".to_string(),
        };

        assert_eq!(error.severity(), ErrorSeverity::High);
        assert!(error.is_recoverable());
        assert!(error.should_retry());
        assert!(error.to_string().contains("memory"));
    }
}
