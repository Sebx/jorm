//! Executor configuration and settings

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for the native executor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: usize,

    /// Default timeout for task execution
    pub default_timeout: Duration,

    /// Retry configuration
    pub retry_config: RetryConfig,

    /// Enable state persistence
    pub state_persistence: bool,

    /// Enable metrics collection
    pub metrics_enabled: bool,

    /// Working directory for task execution
    pub working_directory: Option<std::path::PathBuf>,

    /// Environment variables to inherit
    pub inherit_environment: bool,

    /// Custom environment variables
    pub environment_variables: std::collections::HashMap<String, String>,
}

/// Retry configuration for failed tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial delay between retries
    pub initial_delay: Duration,

    /// Maximum delay between retries
    pub max_delay: Duration,

    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,

    /// Conditions that trigger retries
    pub retry_on: Vec<RetryCondition>,
}

/// Conditions that can trigger a task retry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    /// Retry on specific exit code
    ExitCode(i32),

    /// Retry on timeout
    Timeout,

    /// Retry on network errors
    NetworkError,

    /// Retry on custom condition (regex pattern)
    Custom(String),
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: num_cpus::get(),
            default_timeout: Duration::from_secs(300), // 5 minutes
            retry_config: RetryConfig::default(),
            state_persistence: true,
            metrics_enabled: true,
            working_directory: None,
            inherit_environment: true,
            environment_variables: std::collections::HashMap::new(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            retry_on: vec![
                RetryCondition::ExitCode(1),
                RetryCondition::Timeout,
                RetryCondition::NetworkError,
            ],
        }
    }
}

impl ExecutorConfig {
    /// Create a new executor configuration with custom concurrency
    pub fn with_concurrency(max_concurrent_tasks: usize) -> Self {
        Self {
            max_concurrent_tasks,
            ..Default::default()
        }
    }

    /// Create a new executor configuration with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            default_timeout: timeout,
            ..Default::default()
        }
    }

    /// Enable or disable state persistence
    pub fn with_state_persistence(mut self, enabled: bool) -> Self {
        self.state_persistence = enabled;
        self
    }

    /// Enable or disable metrics collection
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.metrics_enabled = enabled;
        self
    }

    /// Set working directory
    pub fn with_working_directory(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Add environment variable
    pub fn with_env_var(mut self, key: String, value: String) -> Self {
        self.environment_variables.insert(key, value);
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_concurrent_tasks == 0 {
            return Err("max_concurrent_tasks must be greater than 0".to_string());
        }

        if self.default_timeout.is_zero() {
            return Err("default_timeout must be greater than 0".to_string());
        }

        if self.retry_config.max_attempts == 0 {
            return Err("retry max_attempts must be greater than 0".to_string());
        }

        if self.retry_config.backoff_multiplier <= 0.0 {
            return Err("retry backoff_multiplier must be greater than 0".to_string());
        }

        Ok(())
    }
}
