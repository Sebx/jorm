//! Traits and interfaces for task execution
//!
//! This module defines the core traits and data structures used by the JORM executor
//! system. The main trait is [`TaskExecutor`], which defines the interface that all
//! task executors must implement.
//!
//! # Core Concepts
//!
//! ## TaskExecutor Trait
//!
//! The [`TaskExecutor`] trait is the foundation of JORM's pluggable execution system.
//! Each task type (shell, Python, HTTP, etc.) has a corresponding executor that
//! implements this trait.
//!
//! ## Task Definition
//!
//! The [`Task`] struct represents a single executable unit within a DAG, containing
//! all necessary information for execution including configuration, dependencies,
//! and metadata.
//!
//! ## Task Configuration
//!
//! The [`TaskConfig`] enum defines the specific configuration for different task types,
//! allowing type-safe configuration while maintaining flexibility.
//!
//! # Examples
//!
//! ## Implementing a Custom Task Executor
//!
//! ```rust
//! use jorm::executor::{TaskExecutor, Task, ExecutionContext, TaskResult, ExecutorError};
//! use async_trait::async_trait;
//! use std::time::Duration;
//!
//! pub struct CustomTaskExecutor;
//!
//! #[async_trait]
//! impl TaskExecutor for CustomTaskExecutor {
//!     async fn execute(
//!         &self,
//!         task: &Task,
//!         context: &ExecutionContext,
//!     ) -> Result<TaskResult, ExecutorError> {
//!         // Custom execution logic here
//!         todo!("Implement custom task execution")
//!     }
//!
//!     fn task_type(&self) -> &'static str {
//!         "custom"
//!     }
//!
//!     fn supports_parallel(&self) -> bool {
//!         true
//!     }
//!
//!     fn clone_box(&self) -> Box<dyn TaskExecutor> {
//!         Box::new(CustomTaskExecutor)
//!     }
//! }
//! ```
//!
//! ## Creating a Task
//!
//! ```rust
//! use jorm::executor::{Task, TaskConfig};
//!
//! let task = Task::new(
//!     "my_task".to_string(),
//!     "My Custom Task".to_string(),
//!     "shell".to_string(),
//!     TaskConfig::Shell {
//!         command: "echo 'Hello, World!'".to_string(),
//!         working_dir: None,
//!         shell: None,
//!     },
//! );
//! ```

use crate::executor::{ExecutionContext, ExecutorError, TaskResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Trait for task executors
///
/// The `TaskExecutor` trait defines the interface that all task executors must implement.
/// This trait enables JORM's pluggable architecture where different task types can be
/// handled by specialized executors.
///
/// # Required Methods
///
/// - [`execute`](TaskExecutor::execute): The main execution method
/// - [`task_type`](TaskExecutor::task_type): Returns the task type this executor handles
/// - [`clone_box`](TaskExecutor::clone_box): Creates a boxed clone of the executor
///
/// # Optional Methods
///
/// - [`supports_parallel`](TaskExecutor::supports_parallel): Whether the executor supports parallel execution
/// - [`default_timeout`](TaskExecutor::default_timeout): Default timeout for tasks
/// - [`validate_task`](TaskExecutor::validate_task): Pre-execution task validation
///
/// # Examples
///
/// ## Basic Implementation
///
/// ```rust
/// use jorm::executor::{TaskExecutor, Task, ExecutionContext, TaskResult, ExecutorError, TaskStatus};
/// use async_trait::async_trait;
/// use std::time::Duration;
/// use chrono::Utc;
///
/// pub struct EchoTaskExecutor;
///
/// #[async_trait]
/// impl TaskExecutor for EchoTaskExecutor {
///     async fn execute(
///         &self,
///         task: &Task,
///         context: &ExecutionContext,
///     ) -> Result<TaskResult, ExecutorError> {
///         let start_time = std::time::Instant::now();
///         let started_at = Utc::now();
///         
///         // Simple echo implementation
///         let message = format!("Echo: {}", task.name);
///         
///         Ok(TaskResult {
///             task_id: task.id.clone(),
///             status: TaskStatus::Success,
///             started_at,
///             completed_at: Utc::now(),
///             duration: start_time.elapsed(),
///             stdout: message,
///             stderr: String::new(),
///             exit_code: Some(0),
///             retry_count: 0,
///             output_data: None,
///             error_message: None,
///             metadata: std::collections::HashMap::new(),
///         })
///     }
///
///     fn task_type(&self) -> &'static str {
///         "echo"
///     }
///
///     fn clone_box(&self) -> Box<dyn TaskExecutor> {
///         Box::new(EchoTaskExecutor)
///     }
/// }
/// ```
///
/// ## With Validation
///
/// ```rust
/// use jorm::executor::{TaskExecutor, Task, ExecutorError};
/// # use async_trait::async_trait;
/// # pub struct ValidatingExecutor;
///
/// #[async_trait]
/// impl TaskExecutor for ValidatingExecutor {
///     # async fn execute(&self, task: &Task, context: &jorm::executor::ExecutionContext) -> Result<jorm::executor::TaskResult, ExecutorError> { todo!() }
///     # fn task_type(&self) -> &'static str { "validating" }
///     # fn clone_box(&self) -> Box<dyn TaskExecutor> { Box::new(ValidatingExecutor) }
///
///     fn validate_task(&self, task: &Task) -> Result<(), ExecutorError> {
///         if task.name.is_empty() {
///             return Err(ExecutorError::ConfigurationError {
///                 message: "Task name cannot be empty".to_string(),
///             });
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task with the given context
    ///
    /// This is the main execution method that all task executors must implement.
    /// It receives a task definition and execution context, and returns the result
    /// of the task execution.
    ///
    /// # Arguments
    ///
    /// * `task` - The task to execute, containing configuration and metadata
    /// * `context` - The execution context with environment and timing information
    ///
    /// # Returns
    ///
    /// Returns a [`TaskResult`] containing:
    /// - Execution status (Success/Failed)
    /// - Standard output and error streams
    /// - Exit code (if applicable)
    /// - Execution timing information
    /// - Any output data or error messages
    ///
    /// # Errors
    ///
    /// Should return an [`ExecutorError`] if:
    /// - Task configuration is invalid
    /// - Execution fails due to system errors
    /// - Timeout is exceeded
    /// - Resource limits are exceeded
    async fn execute(
        &self,
        task: &Task,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError>;

    /// Get the task type this executor handles
    ///
    /// Returns a string identifier for the task type this executor can handle.
    /// This is used by the task registry to route tasks to the appropriate executor.
    ///
    /// # Examples
    ///
    /// Common task types:
    /// - `"shell"` - Shell command execution
    /// - `"python_script"` - Python script execution
    /// - `"python_function"` - Python function calls
    /// - `"http"` - HTTP requests
    /// - `"file"` - File operations
    fn task_type(&self) -> &'static str;

    /// Check if this executor supports parallel execution
    fn supports_parallel(&self) -> bool {
        true
    }

    /// Get the default timeout for this executor
    fn default_timeout(&self) -> Duration {
        Duration::from_secs(300) // 5 minutes
    }

    /// Validate task configuration before execution
    fn validate_task(&self, _task: &Task) -> Result<(), ExecutorError> {
        // Default implementation - no validation
        Ok(())
    }

    /// Clone this executor into a boxed trait object
    fn clone_box(&self) -> Box<dyn TaskExecutor>;
}

/// Task definition
///
/// Represents a single executable unit within a DAG. A task contains all the information
/// necessary for execution, including its configuration, dependencies, environment settings,
/// and metadata.
///
/// # Fields
///
/// - `id`: Unique identifier for the task within the DAG
/// - `name`: Human-readable name for the task
/// - `task_type`: Type identifier used to select the appropriate executor
/// - `config`: Task-specific configuration (command, URL, etc.)
/// - `timeout`: Optional timeout override for this task
/// - `retry_config`: Optional retry configuration for this task
/// - `environment`: Task-specific environment variables
/// - `depends_on`: List of task IDs this task depends on
/// - `metadata`: Additional metadata for the task
///
/// # Examples
///
/// ## Shell Task
///
/// ```rust
/// use jorm::executor::{Task, TaskConfig};
/// use std::time::Duration;
///
/// let task = Task::new(
///     "build".to_string(),
///     "Build Application".to_string(),
///     "shell".to_string(),
///     TaskConfig::Shell {
///         command: "cargo build --release".to_string(),
///         working_dir: Some("./app".to_string()),
///         shell: None,
///     },
/// )
/// .with_timeout(Duration::from_secs(600))
/// .add_dependency("setup".to_string());
/// ```
///
/// ## HTTP Task
///
/// ```rust
/// use jorm::executor::{Task, TaskConfig};
/// use std::collections::HashMap;
///
/// let mut headers = HashMap::new();
/// headers.insert("Content-Type".to_string(), "application/json".to_string());
///
/// let task = Task::new(
///     "api_call".to_string(),
///     "Call API".to_string(),
///     "http".to_string(),
///     TaskConfig::HttpRequest {
///         method: "POST".to_string(),
///         url: "https://api.example.com/data".to_string(),
///         headers,
///         body: Some(r#"{"key": "value"}"#.to_string()),
///         auth: None,
///         timeout: None,
///     },
/// );
/// ```
///
/// ## Python Task
///
/// ```rust
/// use jorm::executor::{Task, TaskConfig};
///
/// let task = Task::new(
///     "data_processing".to_string(),
///     "Process Data".to_string(),
///     "python_script".to_string(),
///     TaskConfig::PythonScript {
///         script: "process_data.py".to_string(),
///         args: vec!["--input".to_string(), "data.csv".to_string()],
///         python_path: None,
///     },
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: String,

    /// Human-readable task name
    pub name: String,

    /// Task type (determines which executor to use)
    pub task_type: String,

    /// Task-specific configuration
    pub config: TaskConfig,

    /// Task timeout override
    pub timeout: Option<Duration>,

    /// Retry configuration override
    pub retry_config: Option<crate::executor::TaskRetryConfig>,

    /// Environment variables specific to this task
    pub environment: HashMap<String, String>,

    /// Environment configuration for this task
    pub environment_config: Option<crate::executor::EnvironmentConfig>,

    /// Tasks this task depends on
    pub depends_on: Vec<String>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Task-specific configuration
///
/// Defines the configuration for different types of tasks. Each variant contains
/// the specific parameters needed for that task type.
///
/// # Variants
///
/// ## Shell Tasks
///
/// Execute shell commands or scripts:
///
/// ```rust
/// use jorm::executor::TaskConfig;
///
/// let config = TaskConfig::Shell {
///     command: "ls -la".to_string(),
///     working_dir: Some("/tmp".to_string()),
///     shell: Some("bash".to_string()),
/// };
/// ```
///
/// ## Python Script Tasks
///
/// Execute Python scripts with arguments:
///
/// ```rust
/// use jorm::executor::TaskConfig;
///
/// let config = TaskConfig::PythonScript {
///     script: "analyze.py".to_string(),
///     args: vec!["--verbose".to_string(), "data.json".to_string()],
///     python_path: Some("/usr/bin/python3".to_string()),
/// };
/// ```
///
/// ## Python Function Tasks
///
/// Call specific Python functions with arguments:
///
/// ```rust
/// use jorm::executor::TaskConfig;
/// use std::collections::HashMap;
///
/// let mut kwargs = HashMap::new();
/// kwargs.insert("verbose".to_string(), serde_json::Value::Bool(true));
///
/// let config = TaskConfig::PythonFunction {
///     module: "data_processor".to_string(),
///     function: "process_file".to_string(),
///     args: vec![serde_json::Value::String("input.csv".to_string())],
///     kwargs,
/// };
/// ```
///
/// ## HTTP Request Tasks
///
/// Make HTTP requests with full configuration:
///
/// ```rust
/// use jorm::executor::{TaskConfig, AuthConfig};
/// use std::collections::HashMap;
/// use std::time::Duration;
///
/// let mut headers = HashMap::new();
/// headers.insert("User-Agent".to_string(), "JORM/1.0".to_string());
///
/// let config = TaskConfig::HttpRequest {
///     method: "GET".to_string(),
///     url: "https://api.example.com/status".to_string(),
///     headers,
///     body: None,
///     auth: Some(AuthConfig::Bearer {
///         token: "your-token-here".to_string(),
///     }),
///     timeout: Some(Duration::from_secs(30)),
/// };
/// ```
///
/// ## File Operation Tasks
///
/// Perform file system operations:
///
/// ```rust
/// use jorm::executor::TaskConfig;
/// use std::collections::HashMap;
///
/// let mut options = HashMap::new();
/// options.insert("create_dirs".to_string(), serde_json::Value::Bool(true));
///
/// let config = TaskConfig::FileOperation {
///     operation: "copy".to_string(),
///     source: Some("/path/to/source".to_string()),
///     destination: Some("/path/to/dest".to_string()),
///     options,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskConfig {
    /// Shell command execution
    Shell {
        command: String,
        working_dir: Option<String>,
        shell: Option<String>, // e.g., "bash", "cmd", "powershell"
    },

    /// Python script execution
    PythonScript {
        script: String,
        args: Vec<String>,
        python_path: Option<String>,
    },

    /// Python function execution
    PythonFunction {
        module: String,
        function: String,
        args: Vec<serde_json::Value>,
        kwargs: HashMap<String, serde_json::Value>,
    },

    /// File operation
    File {
        operation: String,
        source: Option<String>,
        destination: Option<String>,
        options: HashMap<String, serde_json::Value>,
    },

    /// HTTP request
    HttpRequest {
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Option<String>,
        auth: Option<AuthConfig>,
        timeout: Option<Duration>,
    },

    /// File operation
    FileOperation {
        operation: String, // "copy", "move", "delete", "create"
        source: Option<String>,
        destination: Option<String>,
        options: HashMap<String, serde_json::Value>,
    },
}

/// Authentication configuration for HTTP requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthConfig {
    /// Basic authentication
    Basic { username: String, password: String },

    /// Bearer token authentication
    Bearer { token: String },

    /// API key authentication
    ApiKey {
        key: String,
        header: String, // Header name to use
    },
}

impl Task {
    /// Create a new task
    pub fn new(id: String, name: String, task_type: String, config: TaskConfig) -> Self {
        Self {
            id,
            name,
            task_type,
            config,
            timeout: None,
            retry_config: None,
            environment: HashMap::new(),
            environment_config: None,
            depends_on: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a dependency to this task
    pub fn add_dependency(mut self, dependency: String) -> Self {
        self.depends_on.push(dependency);
        self
    }

    /// Set timeout for this task
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add environment variable to this task
    pub fn with_env_var(mut self, key: String, value: String) -> Self {
        self.environment.insert(key, value);
        self
    }

    /// Add metadata to this task
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get the effective timeout for this task
    pub fn effective_timeout(&self, default_timeout: Duration) -> Duration {
        self.timeout.unwrap_or(default_timeout)
    }

    /// Check if this task has dependencies
    pub fn has_dependencies(&self) -> bool {
        !self.depends_on.is_empty()
    }

    /// Get task dependencies
    pub fn dependencies(&self) -> &[String] {
        &self.depends_on
    }
}
