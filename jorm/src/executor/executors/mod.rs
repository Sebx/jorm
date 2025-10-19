//! Task executor implementations
//!
//! This module contains concrete implementations of the [`TaskExecutor`] trait
//! for different task types. Each executor specializes in handling a specific
//! type of task while providing a consistent interface through the trait.
//!
//! # Available Executors
//!
//! ## Shell Task Executor
//!
//! The [`ShellTaskExecutor`] handles shell command execution across different platforms:
//!
//! ```rust
//! use jorm::executor::{ShellTaskExecutor, Task, TaskConfig, ExecutionContext};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = ShellTaskExecutor::new();
//! let task = Task::new(
//!     "hello".to_string(),
//!     "Hello World".to_string(),
//!     "shell".to_string(),
//!     TaskConfig::Shell {
//!         command: "echo 'Hello, World!'".to_string(),
//!         working_dir: None,
//!         shell: None,
//!     },
//! );
//!
//! let context = ExecutionContext::new("dag".to_string(), "hello".to_string(), Default::default());
//! let result = executor.execute(&task, &context).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Python Task Executor
//!
//! The [`PythonTaskExecutor`] handles Python script and function execution:
//!
//! ```rust
//! use jorm::executor::{PythonTaskExecutor, Task, TaskConfig};
//!
//! let executor = PythonTaskExecutor::new();
//! let task = Task::new(
//!     "python_task".to_string(),
//!     "Python Script".to_string(),
//!     "python_script".to_string(),
//!     TaskConfig::PythonScript {
//!         script: "print('Hello from Python!')".to_string(),
//!         args: vec![],
//!         python_path: None,
//!     },
//! );
//! ```
//!
//! ## HTTP Task Executor
//!
//! The [`HttpTaskExecutor`] handles HTTP requests with full authentication support:
//!
//! ```rust
//! use jorm::executor::{HttpTaskExecutor, Task, TaskConfig, AuthConfig};
//! use std::collections::HashMap;
//!
//! let executor = HttpTaskExecutor::new()?;
//! let task = Task::new(
//!     "api_call".to_string(),
//!     "API Call".to_string(),
//!     "http".to_string(),
//!     TaskConfig::HttpRequest {
//!         method: "GET".to_string(),
//!         url: "https://api.example.com/status".to_string(),
//!         headers: HashMap::new(),
//!         body: None,
//!         auth: Some(AuthConfig::Bearer {
//!             token: "your-token".to_string(),
//!         }),
//!         timeout: None,
//!     },
//! );
//! ```
//!
//! ## File Task Executor
//!
//! The [`FileTaskExecutor`] handles file system operations:
//!
//! ```rust
//! use jorm::executor::{FileTaskExecutor, Task, TaskConfig};
//! use std::collections::HashMap;
//!
//! let executor = FileTaskExecutor::new();
//! let task = Task::new(
//!     "copy_file".to_string(),
//!     "Copy File".to_string(),
//!     "file".to_string(),
//!     TaskConfig::FileOperation {
//!         operation: "copy".to_string(),
//!         source: Some("/path/to/source.txt".to_string()),
//!         destination: Some("/path/to/dest.txt".to_string()),
//!         options: HashMap::new(),
//!     },
//! );
//! ```
//!
//! # Task Registry Integration
//!
//! All executors are automatically registered with the default task registry:
//!
//! ```rust
//! use jorm::executor::TaskRegistry;
//!
//! let registry = TaskRegistry::with_default_executors();
//! assert!(registry.has_executor("shell"));
//! assert!(registry.has_executor("python_script"));
//! assert!(registry.has_executor("http"));
//! assert!(registry.has_executor("file"));
//! ```
//!
//! # Custom Executors
//!
//! You can implement custom executors by implementing the [`TaskExecutor`] trait:
//!
//! ```rust
//! use jorm::executor::{TaskExecutor, Task, ExecutionContext, TaskResult, ExecutorError};
//! use async_trait::async_trait;
//!
//! pub struct CustomExecutor;
//!
//! #[async_trait]
//! impl TaskExecutor for CustomExecutor {
//!     async fn execute(
//!         &self,
//!         task: &Task,
//!         context: &ExecutionContext,
//!     ) -> Result<TaskResult, ExecutorError> {
//!         // Custom execution logic
//!         todo!("Implement custom task execution")
//!     }
//!
//!     fn task_type(&self) -> &'static str {
//!         "custom"
//!     }
//!
//!     fn clone_box(&self) -> Box<dyn TaskExecutor> {
//!         Box::new(CustomExecutor)
//!     }
//! }
//! ```
//!
//! # Platform Support
//!
//! All executors are designed to work cross-platform:
//!
//! - **Shell Executor**: Automatically detects platform (cmd on Windows, sh on Unix)
//! - **Python Executor**: Works with any Python installation
//! - **HTTP Executor**: Platform-independent HTTP client
//! - **File Executor**: Cross-platform file operations
//!
//! # Error Handling
//!
//! All executors provide comprehensive error handling and reporting:
//!
//! - Detailed error messages with context
//! - Proper exit code handling
//! - Timeout support
//! - Resource cleanup on failure
//!
//! # Performance Considerations
//!
//! - **Shell Executor**: Optimized process spawning and I/O handling
//! - **Python Executor**: Efficient subprocess management
//! - **HTTP Executor**: Connection pooling and async I/O
//! - **File Executor**: Async file operations with proper error handling

pub mod file;
pub mod http;
pub mod python;
pub mod shell;

pub use file::*;
pub use http::*;
pub use python::*;
pub use shell::*;
