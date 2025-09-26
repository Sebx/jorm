//! Traits and interfaces for task execution

use crate::executor::{ExecutionContext, TaskResult, ExecutorError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Trait for task executors
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task with the given context
    async fn execute(&self, task: &Task, context: &ExecutionContext) -> Result<TaskResult, ExecutorError>;
    
    /// Get the task type this executor handles
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
}

/// Task definition
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
    pub retry_config: Option<crate::executor::RetryConfig>,
    
    /// Environment variables specific to this task
    pub environment: HashMap<String, String>,
    
    /// Tasks this task depends on
    pub depends_on: Vec<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Task-specific configuration
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
        python_path: Option<String>,
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
    Basic {
        username: String,
        password: String,
    },
    
    /// Bearer token authentication
    Bearer {
        token: String,
    },
    
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