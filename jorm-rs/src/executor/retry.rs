//! Retry mechanisms and backoff strategies for task execution

use crate::executor::{TaskResult, ExecutorError, Task, ExecutionContext, TaskExecutor};
use std::time::Duration;
use tokio::time::sleep;
use async_trait::async_trait;

/// Retry configuration for task execution
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    
    /// Base delay between retries
    pub base_delay: Duration,
    
    /// Backoff strategy to use
    pub strategy: BackoffStrategy,
    
    /// Maximum delay between retries (caps exponential backoff)
    pub max_delay: Option<Duration>,
    
    /// Whether to retry on timeout
    pub retry_on_timeout: bool,
    
    /// Whether to retry on specific error types
    pub retry_on_errors: Vec<RetryableError>,
}

/// Backoff strategies for retry delays
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,
    
    /// Linear increase in delay
    Linear,
    
    /// Exponential increase in delay
    Exponential,
    
    /// Custom backoff function
    Custom(Box<dyn Fn(u32) -> Duration + Send + Sync>),
}

/// Types of errors that can be retried
#[derive(Debug, Clone, PartialEq)]
pub enum RetryableError {
    /// Network-related errors
    Network,
    
    /// Timeout errors
    Timeout,
    
    /// Temporary failures
    Temporary,
    
    /// All errors
    All,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            strategy: BackoffStrategy::Exponential,
            max_delay: Some(Duration::from_secs(60)),
            retry_on_timeout: true,
            retry_on_errors: vec![RetryableError::Network, RetryableError::Timeout],
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(max_retries: u32, base_delay: Duration, strategy: BackoffStrategy) -> Self {
        Self {
            max_retries,
            base_delay,
            strategy,
            max_delay: None,
            retry_on_timeout: true,
            retry_on_errors: vec![RetryableError::Network, RetryableError::Timeout],
        }
    }
    
    /// Create a retry configuration with no retries
    pub fn no_retries() -> Self {
        Self {
            max_retries: 0,
            base_delay: Duration::from_secs(0),
            strategy: BackoffStrategy::Fixed,
            max_delay: None,
            retry_on_timeout: false,
            retry_on_errors: vec![],
        }
    }
    
    /// Set maximum delay for backoff
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = Some(max_delay);
        self
    }
    
    /// Set which errors to retry on
    pub fn with_retry_on_errors(mut self, errors: Vec<RetryableError>) -> Self {
        self.retry_on_errors = errors;
        self
    }
    
    /// Calculate delay for the given attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = match &self.strategy {
            BackoffStrategy::Fixed => self.base_delay,
            BackoffStrategy::Linear => Duration::from_millis(
                self.base_delay.as_millis() as u64 * attempt as u64
            ),
            BackoffStrategy::Exponential => Duration::from_millis(
                self.base_delay.as_millis() as u64 * 2_u64.pow(attempt - 1)
            ),
            BackoffStrategy::Custom(func) => func(attempt),
        };
        
        // Apply maximum delay cap if set
        if let Some(max_delay) = self.max_delay {
            delay.min(max_delay)
        } else {
            delay
        }
    }
    
    /// Check if an error should be retried
    pub fn should_retry(&self, error: &ExecutorError, attempt: u32) -> bool {
        if attempt >= self.max_retries {
            return false;
        }
        
        // Check if error type is retryable
        let is_retryable = match error {
            ExecutorError::TaskTimeout { .. } => {
                self.retry_on_timeout && self.retry_on_errors.contains(&RetryableError::Timeout)
            }
            ExecutorError::TaskExecutionFailed { .. } => {
                self.retry_on_errors.contains(&RetryableError::Temporary) || 
                self.retry_on_errors.contains(&RetryableError::All)
            }
            ExecutorError::NetworkError { .. } => {
                self.retry_on_errors.contains(&RetryableError::Network) ||
                self.retry_on_errors.contains(&RetryableError::All)
            }
            _ => self.retry_on_errors.contains(&RetryableError::All),
        };
        
        is_retryable
    }
}

/// Retry wrapper for task executors
pub struct RetryExecutor<T: TaskExecutor> {
    inner: T,
    retry_config: RetryConfig,
}

impl<T: TaskExecutor> RetryExecutor<T> {
    /// Create a new retry executor
    pub fn new(inner: T, retry_config: RetryConfig) -> Self {
        Self { inner, retry_config }
    }
    
    /// Create a retry executor with default configuration
    pub fn with_default_retry(inner: T) -> Self {
        Self {
            inner,
            retry_config: RetryConfig::default(),
        }
    }
    
    /// Create a retry executor with no retries
    pub fn without_retry(inner: T) -> Self {
        Self {
            inner,
            retry_config: RetryConfig::no_retries(),
        }
    }
}

#[async_trait]
impl<T: TaskExecutor> TaskExecutor for RetryExecutor<T> {
    async fn execute(&self, task: &Task, context: &ExecutionContext) -> Result<TaskResult, ExecutorError> {
        let mut last_error = None;
        
        for attempt in 0..=self.retry_config.max_retries {
            match self.inner.execute(task, context).await {
                Ok(result) => {
                    // Success - return result with retry information
                    return Ok(TaskResult {
                        retry_count: attempt,
                        ..result
                    });
                }
                Err(error) => {
                    last_error = Some(error.clone());
                    
                    // Check if we should retry this error
                    if !self.retry_config.should_retry(&error, attempt) {
                        break;
                    }
                    
                    // If this is not the last attempt, wait before retrying
                    if attempt < self.retry_config.max_retries {
                        let delay = self.retry_config.calculate_delay(attempt + 1);
                        println!("🔄 Retrying task '{}' in {:?} (attempt {}/{})", 
                                task.name, delay, attempt + 1, self.retry_config.max_retries);
                        sleep(delay).await;
                    }
                }
            }
        }
        
        // All retries exhausted - return the last error
        Err(last_error.unwrap_or_else(|| ExecutorError::TaskExecutionFailed {
            task_id: context.task_id.clone(),
            source: anyhow::anyhow!("Unknown error during retry execution"),
        }))
    }
    
    fn task_type(&self) -> &'static str {
        self.inner.task_type()
    }
    
    fn supports_parallel(&self) -> bool {
        self.inner.supports_parallel()
    }
    
    fn default_timeout(&self) -> Duration {
        self.inner.default_timeout()
    }
    
    fn validate_task(&self, task: &Task) -> Result<(), ExecutorError> {
        self.inner.validate_task(task)
    }
}

/// Execute a task with retry logic
pub async fn execute_with_retry<T: TaskExecutor>(
    executor: &T,
    task: &Task,
    context: &ExecutionContext,
    retry_config: &RetryConfig,
) -> Result<TaskResult, ExecutorError> {
    let retry_executor = RetryExecutor::new(executor.clone(), retry_config.clone());
    retry_executor.execute(task, context).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{TaskStatus, TaskResult, ExecutionContext, ExecutorConfig};
    use std::time::Instant;
    
    #[tokio::test]
    async fn test_retry_config_creation() {
        let config = RetryConfig::new(3, Duration::from_secs(1), BackoffStrategy::Exponential);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay, Duration::from_secs(1));
    }
    
    #[tokio::test]
    async fn test_retry_config_no_retries() {
        let config = RetryConfig::no_retries();
        assert_eq!(config.max_retries, 0);
        assert_eq!(config.base_delay, Duration::from_secs(0));
    }
    
    #[tokio::test]
    async fn test_delay_calculation() {
        let config = RetryConfig::new(3, Duration::from_secs(1), BackoffStrategy::Exponential);
        
        // First attempt (attempt 1)
        let delay1 = config.calculate_delay(1);
        assert_eq!(delay1, Duration::from_secs(1));
        
        // Second attempt (attempt 2)
        let delay2 = config.calculate_delay(2);
        assert_eq!(delay2, Duration::from_secs(2));
        
        // Third attempt (attempt 3)
        let delay3 = config.calculate_delay(3);
        assert_eq!(delay3, Duration::from_secs(4));
    }
    
    #[tokio::test]
    async fn test_delay_calculation_with_max_delay() {
        let config = RetryConfig::new(3, Duration::from_secs(1), BackoffStrategy::Exponential)
            .with_max_delay(Duration::from_secs(3));
        
        // Should be capped at max delay
        let delay4 = config.calculate_delay(4);
        assert_eq!(delay4, Duration::from_secs(3));
    }
    
    #[tokio::test]
    async fn test_linear_backoff() {
        let config = RetryConfig::new(3, Duration::from_secs(1), BackoffStrategy::Linear);
        
        let delay1 = config.calculate_delay(1);
        assert_eq!(delay1, Duration::from_secs(1));
        
        let delay2 = config.calculate_delay(2);
        assert_eq!(delay2, Duration::from_secs(2));
        
        let delay3 = config.calculate_delay(3);
        assert_eq!(delay3, Duration::from_secs(3));
    }
    
    #[tokio::test]
    async fn test_should_retry_logic() {
        let config = RetryConfig::new(3, Duration::from_secs(1), BackoffStrategy::Fixed)
            .with_retry_on_errors(vec![RetryableError::Timeout]);
        
        let timeout_error = ExecutorError::TaskTimeout {
            task_id: "test".to_string(),
            timeout: Duration::from_secs(5),
        };
        
        assert!(config.should_retry(&timeout_error, 0));
        assert!(config.should_retry(&timeout_error, 1));
        assert!(config.should_retry(&timeout_error, 2));
        assert!(!config.should_retry(&timeout_error, 3)); // Max retries exceeded
    }
}
