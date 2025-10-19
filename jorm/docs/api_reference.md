# JORM Native Executor API Reference

This document provides a comprehensive API reference for the JORM native executor, generated from the Rust documentation.

## Table of Contents

- [Core Types](#core-types)
- [Executor Configuration](#executor-configuration)
- [Task Executors](#task-executors)
- [Error Handling](#error-handling)
- [State Management](#state-management)
- [Resource Monitoring](#resource-monitoring)
- [Environment Management](#environment-management)
- [Metrics and Performance](#metrics-and-performance)

## Core Types

### NativeExecutor

The main executor for DAG execution with parallel task processing.

```rust
pub struct NativeExecutor {
    config: ExecutorConfig,
    task_registry: TaskRegistry,
    execution_semaphore: Semaphore,
    state_manager: Option<StateManager>,
    resource_monitor: Option<Arc<ResourceMonitor>>,
}
```

#### Key Methods

- `new(config: ExecutorConfig) -> Self` - Create executor with configuration
- `with_state_management(config: ExecutorConfig, state_config: StateConfig) -> Result<Self>` - Create with state persistence
- `execute_dag(&self, dag: &Dag) -> Result<ExecutionResult>` - Execute a complete DAG
- `start_resource_monitoring(&self) -> Result<()>` - Start resource monitoring
- `get_resource_usage(&self) -> Option<ResourceUsage>` - Get current resource usage

### TaskRegistry

Registry for task executors by task type.

```rust
pub struct TaskRegistry {
    executors: HashMap<String, Box<dyn TaskExecutor>>,
}
```

#### Key Methods

- `new() -> Self` - Create empty registry
- `with_default_executors() -> Self` - Create with built-in executors
- `register(&mut self, task_type: String, executor: Box<dyn TaskExecutor>)` - Register executor
- `has_executor(&self, task_type: &str) -> bool` - Check if executor exists
- `validate_dag_tasks(&self, dag: &Dag) -> Result<()>` - Validate DAG tasks

### Task

Represents a single executable unit within a DAG.

```rust
pub struct Task {
    pub id: String,
    pub name: String,
    pub task_type: String,
    pub config: TaskConfig,
    pub timeout: Option<Duration>,
    pub retry_config: Option<TaskRetryConfig>,
    pub environment: HashMap<String, String>,
    pub depends_on: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

#### Key Methods

- `new(id: String, name: String, task_type: String, config: TaskConfig) -> Self`
- `add_dependency(self, dependency: String) -> Self`
- `with_timeout(self, timeout: Duration) -> Self`
- `with_env_var(self, key: String, value: String) -> Self`
- `effective_timeout(&self, default_timeout: Duration) -> Duration`

### TaskConfig

Configuration for different task types.

```rust
pub enum TaskConfig {
    Shell {
        command: String,
        working_dir: Option<String>,
        shell: Option<String>,
    },
    PythonScript {
        script: String,
        args: Vec<String>,
        python_path: Option<String>,
    },
    PythonFunction {
        module: String,
        function: String,
        args: Vec<serde_json::Value>,
        kwargs: HashMap<String, serde_json::Value>,
    },
    HttpRequest {
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Option<String>,
        auth: Option<AuthConfig>,
        timeout: Option<Duration>,
    },
    FileOperation {
        operation: String,
        source: Option<String>,
        destination: Option<String>,
        options: HashMap<String, serde_json::Value>,
    },
}
```

### ExecutionResult

Result of DAG execution.

```rust
pub struct ExecutionResult {
    pub dag_id: String,
    pub execution_id: String,
    pub status: ExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_duration: Option<Duration>,
    pub task_results: HashMap<String, TaskResult>,
    pub metrics: ExecutionMetrics,
}
```

### TaskResult

Result of individual task execution.

```rust
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration: Duration,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub retry_count: u32,
    pub output_data: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}
```

## Executor Configuration

### ExecutorConfig

Main configuration structure for the executor.

```rust
pub struct ExecutorConfig {
    pub max_concurrent_tasks: usize,
    pub default_timeout: Duration,
    pub retry_config: Option<RetryConfig>,
    pub environment_variables: HashMap<String, String>,
    pub inherit_environment: bool,
    pub working_directory: Option<PathBuf>,
    pub resource_limits: Option<ResourceLimits>,
    pub enable_resource_throttling: bool,
    pub profile: Option<String>,
}
```

#### Default Values

- `max_concurrent_tasks`: Number of CPU cores
- `default_timeout`: 300 seconds (5 minutes)
- `retry_config`: None (no retries)
- `inherit_environment`: true
- `enable_resource_throttling`: false

### RetryConfig

Configuration for task retry behavior.

```rust
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub use_exponential: bool,
}
```

### ConfigProfile

Configuration profile for different environments.

```rust
pub struct ConfigProfile {
    pub name: String,
    pub config: ExecutorConfig,
}
```

## Task Executors

### TaskExecutor Trait

Core trait that all task executors must implement.

```rust
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute(&self, task: &Task, context: &ExecutionContext) -> Result<TaskResult, ExecutorError>;
    fn task_type(&self) -> &'static str;
    fn supports_parallel(&self) -> bool { true }
    fn default_timeout(&self) -> Duration { Duration::from_secs(300) }
    fn validate_task(&self, task: &Task) -> Result<(), ExecutorError> { Ok(()) }
    fn clone_box(&self) -> Box<dyn TaskExecutor>;
}
```

### ShellTaskExecutor

Executor for shell commands and scripts.

```rust
pub struct ShellTaskExecutor {
    default_timeout: Duration,
    default_shell: String,
}
```

#### Key Methods

- `new() -> Self` - Create with platform defaults
- `with_timeout(timeout: Duration) -> Self` - Create with custom timeout
- `with_shell(shell: String) -> Self` - Create with custom shell

#### Supported Platforms

- **Windows**: cmd, PowerShell
- **Unix-like**: sh, bash, zsh, fish

### PythonTaskExecutor

Executor for Python scripts and functions.

```rust
pub struct PythonTaskExecutor {
    python_path: String,
    default_timeout: Duration,
}
```

#### Key Methods

- `new() -> Self` - Create with system Python
- `with_python_path(path: String) -> Self` - Create with custom Python
- `with_timeout(timeout: Duration) -> Self` - Create with custom timeout

### HttpTaskExecutor

Executor for HTTP requests.

```rust
pub struct HttpTaskExecutor {
    client: reqwest::Client,
    default_timeout: Duration,
}
```

#### Key Methods

- `new() -> Result<Self>` - Create with default client
- `with_timeout(timeout: Duration) -> Result<Self>` - Create with custom timeout
- `with_client_config<F>(config_fn: F) -> Result<Self>` - Create with custom client

#### Authentication Support

- Basic authentication
- Bearer token authentication
- API key authentication

### FileTaskExecutor

Executor for file system operations.

```rust
pub struct FileTaskExecutor;
```

#### Supported Operations

- `copy` - Copy files or directories
- `move` - Move/rename files or directories
- `delete` - Delete files or directories
- `create` - Create files with content

## Error Handling

### ExecutorError

Comprehensive error type for all executor operations.

```rust
pub enum ExecutorError {
    TaskExecutionFailed { task_id: String, source: anyhow::Error },
    DependencyCycle,
    TaskTimeout { task_id: String, timeout: Duration },
    ResourceLimitExceeded { resource: String },
    StatePersistenceError(sqlx::Error),
    ConfigurationError { message: String },
    UnsupportedTaskType { task_type: String },
    TaskNotFound { task_id: String },
    ExecutionContextError { message: String },
    RetryLimitExceeded { task_id: String, attempts: u32 },
    SemaphoreError(tokio::sync::AcquireError),
    IoError { message: String, source: std::io::Error },
    SerializationError(serde_json::Error),
    TaskRegistryError { message: String },
    ParallelExecutionError { message: String, failed_tasks: Vec<String> },
    RecoveryError { message: String, execution_id: String },
    ValidationError { message: String, field: String, value: String },
}
```

### ErrorContext

Additional context for debugging errors.

```rust
pub struct ErrorContext {
    pub dag_id: Option<String>,
    pub task_id: Option<String>,
    pub execution_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub call_stack: Vec<String>,
}
```

## State Management

### StateConfig

Configuration for execution state persistence.

```rust
pub struct StateConfig {
    pub database_url: String,
    pub enable_checkpoints: bool,
    pub checkpoint_interval: Duration,
    pub cleanup_completed_after: Option<Duration>,
    pub max_execution_history: Option<usize>,
}
```

### ExecutionState

Persistent execution state.

```rust
pub struct ExecutionState {
    pub dag_id: String,
    pub execution_id: String,
    pub status: ExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub task_states: HashMap<String, TaskState>,
}
```

### StateManager

Manager for execution state persistence.

```rust
pub struct StateManager {
    db_pool: SqlitePool,
    config: StateConfig,
}
```

#### Key Methods

- `new(config: StateConfig) -> Result<Self>` - Create state manager
- `save_execution_state(&self, state: &ExecutionState) -> Result<()>` - Save state
- `load_execution_state(&self, execution_id: &str) -> Result<Option<ExecutionState>>` - Load state
- `get_task_history(&self, dag_id: &str, task_id: &str) -> Result<Vec<TaskExecution>>` - Get history

## Resource Monitoring

### ResourceLimits

Configuration for resource limits and monitoring.

```rust
pub struct ResourceLimits {
    pub max_cpu_percent: f64,
    pub max_memory_percent: f64,
    pub max_concurrent_tasks: usize,
    pub monitoring_interval: Duration,
    pub throttle_threshold: f64,
}
```

### ResourceMonitor

Monitor for system resource usage.

```rust
pub struct ResourceMonitor {
    limits: ResourceLimits,
    // Internal fields...
}
```

#### Key Methods

- `new(limits: ResourceLimits) -> Self` - Create monitor
- `start_monitoring(&self) -> Result<()>` - Start monitoring
- `stop_monitoring(&self)` - Stop monitoring
- `get_current_usage(&self) -> ResourceUsage` - Get current usage
- `check_resource_availability(&self, estimated: &EstimatedResources) -> ResourceAvailability`

### ResourceUsage

Current system resource usage.

```rust
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub running_tasks: usize,
    pub queued_tasks: usize,
    pub timestamp: DateTime<Utc>,
}
```

## Environment Management

### EnvironmentConfig

Configuration for environment variable management.

```rust
pub struct EnvironmentConfig {
    pub variables: HashMap<String, String>,
    pub inherit_parent: bool,
    pub inherit_executor: bool,
    pub exclude: Vec<String>,
    pub enable_interpolation: bool,
    pub secure_prefixes: Vec<String>,
}
```

### EnvironmentManager

Manager for secure environment variable handling.

```rust
pub struct EnvironmentManager {
    executor_env: HashMap<String, String>,
    config: EnvironmentConfig,
}
```

#### Key Methods

- `new(executor_env: HashMap<String, String>, config: EnvironmentConfig) -> Self`
- `build_task_environment(&self, task_env: &HashMap<String, String>, context: Option<&InterpolationContext>) -> Result<HashMap<String, SecureValue>>`
- `validate_config(&self) -> Result<()>`

### SecureValue

Secure environment variable value that masks sensitive data.

```rust
pub struct SecureValue {
    value: String,
    is_secure: bool,
}
```

#### Key Methods

- `new(value: String, is_secure: bool) -> Self`
- `secure(value: String) -> Self` - Create secure value
- `plain(value: String) -> Self` - Create plain value
- `value(&self) -> &str` - Get actual value
- `is_secure(&self) -> bool` - Check if secure

### InterpolationContext

Context for environment variable interpolation.

```rust
pub struct InterpolationContext {
    pub variables: HashMap<String, String>,
    pub task_outputs: HashMap<String, String>,
    pub system_info: HashMap<String, String>,
}
```

## Metrics and Performance

### ExecutionMetrics

Comprehensive execution metrics.

```rust
pub struct ExecutionMetrics {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub skipped_tasks: usize,
    pub peak_concurrent_tasks: usize,
    pub total_execution_time: Duration,
    pub task_execution_times: HashMap<String, Duration>,
    pub resource_usage_samples: Vec<ResourceUsage>,
}
```

### MetricsCollector

Collector for execution metrics and performance data.

```rust
pub struct MetricsCollector {
    // Internal fields...
}
```

#### Key Methods

- `new() -> Self` - Create collector
- `start_dag_execution(&mut self, dag_id: String, task_count: usize)`
- `record_task_start(&mut self, task_id: String)`
- `record_task_completion(&mut self, task_id: String, result: &TaskResult)`
- `finish_dag_execution(&mut self) -> ExecutionMetrics`

### PerformanceBenchmark

Performance benchmarking utilities.

```rust
pub struct PerformanceBenchmark {
    // Internal fields...
}
```

#### Key Methods

- `new(name: String) -> Self` - Create benchmark
- `run_benchmark<F, Fut>(&self, operation: F) -> BenchmarkResult` - Run benchmark
- `compare_with_baseline(&self, baseline: &BenchmarkResult) -> ComparisonResult`

## Usage Examples

### Basic DAG Execution

```rust
use jorm::executor::{NativeExecutor, ExecutorConfig};
use jorm::parser::parse_dag_file;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExecutorConfig::default();
    let executor = NativeExecutor::new(config);
    
    let dag = parse_dag_file("workflow.txt").await?;
    let result = executor.execute_dag(&dag).await?;
    
    println!("Execution completed: {:?}", result.status);
    Ok(())
}
```

### Custom Configuration

```rust
use jorm::executor::{ExecutorConfig, ResourceLimits, RetryConfig};
use std::time::Duration;

let config = ExecutorConfig {
    max_concurrent_tasks: 8,
    default_timeout: Duration::from_secs(600),
    enable_resource_throttling: true,
    resource_limits: Some(ResourceLimits {
        max_cpu_percent: 80.0,
        max_memory_percent: 70.0,
        max_concurrent_tasks: 6,
        monitoring_interval: Duration::from_secs(10),
        throttle_threshold: 0.9,
    }),
    retry_config: Some(RetryConfig {
        max_attempts: 3,
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
        use_exponential: true,
    }),
    ..Default::default()
};
```

### State Management

```rust
use jorm::executor::{NativeExecutor, ExecutorConfig, StateConfig};
use std::time::Duration;

let config = ExecutorConfig::default();
let state_config = StateConfig {
    database_url: "sqlite:jorm_state.db".to_string(),
    enable_checkpoints: true,
    checkpoint_interval: Duration::from_secs(30),
    cleanup_completed_after: Some(Duration::from_secs(7 * 24 * 3600)),
    max_execution_history: Some(1000),
};

let executor = NativeExecutor::with_state_management(config, state_config).await?;
```

### Custom Task Executor

```rust
use jorm::executor::{TaskExecutor, Task, ExecutionContext, TaskResult, ExecutorError, TaskStatus};
use async_trait::async_trait;
use chrono::Utc;

pub struct CustomExecutor;

#[async_trait]
impl TaskExecutor for CustomExecutor {
    async fn execute(
        &self,
        task: &Task,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError> {
        let start_time = std::time::Instant::now();
        let started_at = Utc::now();
        
        // Custom execution logic here
        let result = "Custom task output".to_string();
        
        Ok(TaskResult {
            task_id: task.id.clone(),
            status: TaskStatus::Success,
            started_at,
            completed_at: Utc::now(),
            duration: start_time.elapsed(),
            stdout: result,
            stderr: String::new(),
            exit_code: Some(0),
            retry_count: 0,
            output_data: None,
            error_message: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn task_type(&self) -> &'static str {
        "custom"
    }

    fn clone_box(&self) -> Box<dyn TaskExecutor> {
        Box::new(CustomExecutor)
    }
}
```

This API reference provides comprehensive documentation for all public APIs in the JORM native executor. For more detailed examples and usage patterns, see the [API Examples](api_examples.md) document.