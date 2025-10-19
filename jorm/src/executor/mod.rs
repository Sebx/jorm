//! Native Rust executor for JORM DAG execution
//!
//! This module provides high-performance, parallel task execution capabilities
//! that replace the Python execution engine while maintaining full compatibility.
//!
//! # Overview
//!
//! The native executor is the core component of JORM's Phase 3 implementation,
//! providing true parallel task execution with comprehensive error handling,
//! state management, and resource monitoring.
//!
//! # Key Features
//!
//! - **Parallel Execution**: True concurrent task execution respecting dependencies
//! - **Task Types**: Support for shell, Python, HTTP, and file operation tasks
//! - **State Management**: Persistent execution state with recovery capabilities
//! - **Resource Monitoring**: CPU and memory usage tracking with throttling
//! - **Retry Logic**: Configurable retry mechanisms with exponential backoff
//! - **Metrics Collection**: Comprehensive performance and execution metrics
//!
//! # Quick Start
//!
//! ```rust
//! use jorm::executor::{NativeExecutor, ExecutorConfig};
//! use jorm::parser::parse_dag_file;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create executor with default configuration
//!     let config = ExecutorConfig::default();
//!     let executor = NativeExecutor::new(config);
//!
//!     // Parse and execute a DAG
//!     let dag = parse_dag_file("my_workflow.txt").await?;
//!     let result = executor.execute_dag(&dag).await?;
//!
//!     println!("Execution completed with status: {:?}", result.status);
//!     Ok(())
//! }
//! ```
//!
//! # Configuration
//!
//! The executor can be configured with various options:
//!
//! ```rust
//! use jorm::executor::{ExecutorConfig, ResourceLimits};
//! use std::time::Duration;
//!
//! let config = ExecutorConfig {
//!     max_concurrent_tasks: 10,
//!     default_timeout: Duration::from_secs(300),
//!     enable_state_persistence: true,
//!     enable_resource_throttling: true,
//!     resource_limits: Some(ResourceLimits {
//!         max_cpu_percent: 80.0,
//!         max_memory_percent: 70.0,
//!         max_concurrent_tasks: 8,
//!     }),
//!     ..Default::default()
//! };
//! ```
//!
//! # Task Executors
//!
//! The executor supports multiple task types through pluggable executors:
//!
//! - [`ShellTaskExecutor`]: Execute shell commands and scripts
//! - [`PythonTaskExecutor`]: Run Python scripts and functions
//! - [`HttpTaskExecutor`]: Make HTTP requests
//! - [`FileTaskExecutor`]: Perform file operations
//!
//! # State Management
//!
//! Enable state persistence for execution recovery:
//!
//! ```rust
//! use jorm::executor::{NativeExecutor, ExecutorConfig, StateConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ExecutorConfig::default();
//! let state_config = StateConfig {
//!     database_url: "sqlite:jorm_state.db".to_string(),
//!     enable_checkpoints: true,
//!     checkpoint_interval: Duration::from_secs(30),
//! };
//!
//! let executor = NativeExecutor::with_state_management(config, state_config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Error Handling
//!
//! The executor provides comprehensive error handling through [`ExecutorError`]:
//!
//! ```rust
//! use jorm::executor::{ExecutorError, NativeExecutor};
//!
//! # async fn example() -> Result<(), ExecutorError> {
//! let executor = NativeExecutor::new(Default::default());
//! 
//! match executor.execute_dag(&dag).await {
//!     Ok(result) => println!("Success: {:?}", result),
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

pub mod config;
pub mod context;
pub mod dataflow;
pub mod environment;
pub mod error;
pub mod executors;
pub mod metrics;
pub mod output;
pub mod parallel;
pub mod recovery;
pub mod resource_monitor;
pub mod result;
pub mod retry;
pub mod scheduler;
pub mod state;
pub mod state_manager;
pub mod traits;

pub use config::{ExecutorConfig, ConfigManager, ConfigProfile, ConfigError};
pub use context::*;
pub use dataflow::{DataflowManager, ParameterValue, SecureParameter, ParameterValidation, ParameterType, ResolvedParameter, ParameterSource};
pub use environment::{EnvironmentConfig, EnvironmentManager, InterpolationContext, SecureValue};
pub use state::{ExecutionState, StateConfig, TaskState as StateTaskState};
pub use error::*;
pub use executors::*;
pub use metrics::{MetricsCollector, PerformanceBenchmark, ResourceMetrics, MetricsExport, MetricsSummary};
pub use output::{OutputFormatter, CompatibilityFormatter, ProgressBar, format_duration, format_bytes};
pub use parallel::{ParallelExecutor, ParallelExecutionStats, ResourceAwareParallelExecutor, ResourceAwareExecutionStats};
pub use recovery::*;
pub use resource_monitor::{ResourceMonitor, ResourceLimits, ResourceUsage, ResourceAvailability, QueuedTask, TaskPriority, EstimatedResources, QueueStatus};
pub use result::*;
pub use retry::{RetryConfig as TaskRetryConfig, BackoffStrategy, RetryableError, RetryExecutor, execute_with_retry};
pub use scheduler::{DependencyScheduler, ExecutionGraph, TaskState, ExecutionStats};
pub use state_manager::{StateManager as PersistentStateManager, RecoveryCheckpoint, PersistedExecution, PersistedTask, ExecutionQuery};
pub use traits::*;

use crate::parser::Dag;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use uuid::Uuid;

/// Main native executor for DAG execution
///
/// The `NativeExecutor` is the primary interface for executing DAGs with the native Rust engine.
/// It provides high-performance, parallel task execution while maintaining full compatibility
/// with existing DAG definitions.
///
/// # Features
///
/// - **Parallel Execution**: Executes independent tasks concurrently while respecting dependencies
/// - **Resource Management**: Optional CPU and memory monitoring with throttling
/// - **State Persistence**: Optional execution state persistence for recovery
/// - **Comprehensive Metrics**: Detailed execution metrics and performance data
/// - **Flexible Configuration**: Extensive configuration options for different use cases
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use jorm::executor::{NativeExecutor, ExecutorConfig};
/// use jorm::parser::parse_dag_file;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let executor = NativeExecutor::new(ExecutorConfig::default());
/// let dag = parse_dag_file("workflow.txt").await?;
/// let result = executor.execute_dag(&dag).await?;
/// 
/// println!("Executed {} tasks", result.task_results.len());
/// # Ok(())
/// # }
/// ```
///
/// ## With State Management
///
/// ```rust
/// use jorm::executor::{NativeExecutor, ExecutorConfig, StateConfig};
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ExecutorConfig::default();
/// let state_config = StateConfig {
///     database_url: "sqlite:jorm_state.db".to_string(),
///     enable_checkpoints: true,
///     checkpoint_interval: Duration::from_secs(30),
/// };
///
/// let executor = NativeExecutor::with_state_management(config, state_config).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## With Resource Monitoring
///
/// ```rust
/// use jorm::executor::{ExecutorConfig, ResourceLimits, NativeExecutor};
///
/// let config = ExecutorConfig {
///     enable_resource_throttling: true,
///     resource_limits: Some(ResourceLimits {
///         max_cpu_percent: 80.0,
///         max_memory_percent: 70.0,
///         max_concurrent_tasks: 8,
///     }),
///     ..Default::default()
/// };
///
/// let executor = NativeExecutor::new(config);
/// ```
pub struct NativeExecutor {
    config: ExecutorConfig,
    task_registry: TaskRegistry,
    execution_semaphore: Semaphore,
    state_manager: Option<state::StateManager>,
    resource_monitor: Option<Arc<ResourceMonitor>>,
    dataflow_manager: DataflowManager,
}

/// Registry for task executors by task type
///
/// The `TaskRegistry` manages the mapping between task types and their corresponding
/// executor implementations. It provides a pluggable architecture where custom
/// task executors can be registered for specific task types.
///
/// # Default Executors
///
/// The registry comes with built-in executors for common task types:
/// - `shell`: [`ShellTaskExecutor`] for shell commands
/// - `python_script`: [`PythonTaskExecutor`] for Python scripts
/// - `python_function`: [`PythonTaskExecutor`] for Python function calls
/// - `http`: [`HttpTaskExecutor`] for HTTP requests
/// - `file`: [`FileTaskExecutor`] for file operations
///
/// # Examples
///
/// ## Using Default Registry
///
/// ```rust
/// use jorm::executor::TaskRegistry;
///
/// let registry = TaskRegistry::with_default_executors();
/// assert!(registry.has_executor("shell"));
/// assert!(registry.has_executor("http"));
/// ```
///
/// ## Custom Registry
///
/// ```rust
/// use jorm::executor::{TaskRegistry, ShellTaskExecutor};
///
/// let mut registry = TaskRegistry::new();
/// registry.register("custom_shell", Box::new(ShellTaskExecutor::new()));
/// ```
pub struct TaskRegistry {
    executors: HashMap<String, Box<dyn TaskExecutor>>,
}



impl NativeExecutor {
    /// Create a new native executor with the given configuration
    ///
    /// Creates a new `NativeExecutor` instance with the specified configuration.
    /// The executor will use default task executors for all supported task types.
    ///
    /// # Arguments
    ///
    /// * `config` - The executor configuration specifying timeouts, concurrency limits, etc.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jorm::executor::{NativeExecutor, ExecutorConfig};
    /// use std::time::Duration;
    ///
    /// let config = ExecutorConfig {
    ///     max_concurrent_tasks: 5,
    ///     default_timeout: Duration::from_secs(600),
    ///     ..Default::default()
    /// };
    ///
    /// let executor = NativeExecutor::new(config);
    /// ```
    pub fn new(config: ExecutorConfig) -> Self {
        let execution_semaphore = Semaphore::new(config.max_concurrent_tasks);
        
        // Create resource monitor if enabled
        let resource_monitor = if config.enable_resource_throttling {
            config.resource_limits.as_ref().map(|limits| {
                Arc::new(ResourceMonitor::new(limits.clone()))
            })
        } else {
            None
        };

        Self {
            config,
            task_registry: TaskRegistry::with_default_executors(),
            execution_semaphore,
            state_manager: None,
            resource_monitor,
            dataflow_manager: DataflowManager::new(),
        }
    }

    /// Create a new native executor with state management enabled
    ///
    /// Creates a new `NativeExecutor` with persistent state management capabilities.
    /// This enables execution recovery, checkpointing, and execution history tracking.
    ///
    /// # Arguments
    ///
    /// * `config` - The executor configuration
    /// * `state_config` - Configuration for state persistence (database URL, checkpoint settings)
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the configured executor or an error if state
    /// management initialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jorm::executor::{NativeExecutor, ExecutorConfig, StateConfig};
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ExecutorConfig::default();
    /// let state_config = StateConfig {
    ///     database_url: "sqlite:jorm_state.db".to_string(),
    ///     enable_checkpoints: true,
    ///     checkpoint_interval: Duration::from_secs(30),
    /// };
    ///
    /// let executor = NativeExecutor::with_state_management(config, state_config).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The database connection cannot be established
    /// - Database schema initialization fails
    /// - Invalid state configuration is provided
    pub async fn with_state_management(config: ExecutorConfig, state_config: StateConfig) -> Result<Self> {
        let execution_semaphore = Semaphore::new(config.max_concurrent_tasks);
        let state_manager = state::StateManager::new(state_config).await?;
        
        // Create resource monitor if enabled
        let resource_monitor = if config.enable_resource_throttling {
            config.resource_limits.as_ref().map(|limits| {
                Arc::new(ResourceMonitor::new(limits.clone()))
            })
        } else {
            None
        };

        Ok(Self {
            config,
            task_registry: TaskRegistry::with_default_executors(),
            execution_semaphore,
            state_manager: Some(state_manager),
            resource_monitor,
            dataflow_manager: DataflowManager::new(),
        })
    }

    /// Create a new native executor with custom task registry
    pub fn with_task_registry(config: ExecutorConfig, task_registry: TaskRegistry) -> Self {
        let execution_semaphore = Semaphore::new(config.max_concurrent_tasks);
        
        // Create resource monitor if enabled
        let resource_monitor = if config.enable_resource_throttling {
            config.resource_limits.as_ref().map(|limits| {
                Arc::new(ResourceMonitor::new(limits.clone()))
            })
        } else {
            None
        };

        Self {
            config,
            task_registry,
            execution_semaphore,
            state_manager: None,
            resource_monitor,
            dataflow_manager: DataflowManager::new(),
        }
    }

    /// Get a reference to the task registry
    pub fn task_registry(&self) -> &TaskRegistry {
        &self.task_registry
    }

    /// Get a mutable reference to the task registry
    pub fn task_registry_mut(&mut self) -> &mut TaskRegistry {
        &mut self.task_registry
    }

    /// Get the executor configuration
    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }

    /// Get the resource monitor if enabled
    pub fn resource_monitor(&self) -> Option<&Arc<ResourceMonitor>> {
        self.resource_monitor.as_ref()
    }

    /// Get a reference to the dataflow manager
    pub fn dataflow_manager(&self) -> &DataflowManager {
        &self.dataflow_manager
    }

    /// Get a mutable reference to the dataflow manager
    pub fn dataflow_manager_mut(&mut self) -> &mut DataflowManager {
        &mut self.dataflow_manager
    }

    /// Start resource monitoring if enabled
    pub async fn start_resource_monitoring(&self) -> Result<()> {
        if let Some(monitor) = &self.resource_monitor {
            monitor.start_monitoring().await?;
            println!("📊 Resource monitoring started");
        }
        Ok(())
    }

    /// Stop resource monitoring if enabled
    pub async fn stop_resource_monitoring(&self) {
        if let Some(monitor) = &self.resource_monitor {
            monitor.stop_monitoring().await;
            println!("📊 Resource monitoring stopped");
        }
    }

    /// Get current resource usage if monitoring is enabled
    pub async fn get_resource_usage(&self) -> Option<ResourceUsage> {
        if let Some(monitor) = &self.resource_monitor {
            Some(monitor.get_current_usage().await)
        } else {
            None
        }
    }

    /// Execute a complete DAG with parallel execution
    ///
    /// Executes all tasks in the provided DAG, respecting dependencies and running
    /// independent tasks in parallel. This is the primary method for DAG execution.
    ///
    /// # Arguments
    ///
    /// * `dag` - The DAG to execute, containing tasks and their dependencies
    ///
    /// # Returns
    ///
    /// Returns an [`ExecutionResult`] containing:
    /// - Overall execution status (Success/Failed)
    /// - Individual task results with stdout/stderr
    /// - Execution metrics and timing information
    /// - Error details for failed tasks
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jorm::executor::{NativeExecutor, ExecutorConfig, ExecutionStatus};
    /// use jorm::parser::parse_dag_file;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = NativeExecutor::new(ExecutorConfig::default());
    /// let dag = parse_dag_file("workflow.txt").await?;
    /// 
    /// let result = executor.execute_dag(&dag).await?;
    /// 
    /// match result.status {
    ///     ExecutionStatus::Success => {
    ///         println!("All {} tasks completed successfully", result.task_results.len());
    ///     }
    ///     ExecutionStatus::Failed => {
    ///         let failed_tasks: Vec<_> = result.task_results.iter()
    ///             .filter(|(_, r)| r.status == TaskStatus::Failed)
    ///             .collect();
    ///         println!("Execution failed. {} tasks failed", failed_tasks.len());
    ///     }
    ///     _ => println!("Execution status: {:?}", result.status),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Behavior
    ///
    /// - Tasks with no dependencies start immediately
    /// - Tasks wait for their dependencies to complete successfully
    /// - Failed tasks cause dependent tasks to be skipped
    /// - Resource limits are respected if monitoring is enabled
    /// - Execution state is checkpointed if state management is enabled
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - DAG validation fails (cycles, missing executors)
    /// - Critical system errors occur during execution
    /// - State persistence fails (if enabled)
    pub async fn execute_dag(&self, dag: &Dag) -> Result<ExecutionResult> {
        self.execute_dag_internal(dag, None).await
    }

    /// Internal DAG execution with optional execution ID for recovery
    async fn execute_dag_internal(&self, dag: &Dag, execution_id: Option<String>) -> Result<ExecutionResult> {
        let execution_id = execution_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let start_time = std::time::Instant::now();
        let started_at = chrono::Utc::now();

        println!(
            "🚀 Starting parallel DAG execution: {} (ID: {})",
            dag.name, execution_id
        );

        // Validate that all tasks have registered executors
        if let Err(e) = self.task_registry.validate_dag_tasks(dag) {
            println!("❌ DAG validation failed: {e}");
            return Ok(ExecutionResult {
                dag_id: dag.name.clone(),
                execution_id,
                status: ExecutionStatus::Failed,
                started_at,
                completed_at: Some(chrono::Utc::now()),
                total_duration: Some(start_time.elapsed()),
                task_results: HashMap::new(),
                metrics: ExecutionMetrics::default(),
            });
        }

        // Create execution graph for dependency tracking
        let mut execution_graph = ExecutionGraph::new(dag)?;
        println!("📋 Execution graph created with {} tasks", dag.tasks.len());

        // Create parallel executor (resource-aware if monitoring is enabled)
        let mut parallel_executor = if let Some(resource_monitor) = &self.resource_monitor {
            // Start resource monitoring for this execution
            resource_monitor.start_monitoring().await?;
            println!("📊 Resource monitoring enabled for DAG execution");
            
            // Use resource-aware executor
            let resource_aware = ResourceAwareParallelExecutor::new(
                self.config.max_concurrent_tasks,
                resource_monitor.clone(),
                true, // Enable adaptive concurrency
            );
            
            // For now, we'll use the base ParallelExecutor but could extend this
            // to use ResourceAwareParallelExecutor throughout the execution
            ParallelExecutor::new(self.config.max_concurrent_tasks)
        } else {
            ParallelExecutor::new(self.config.max_concurrent_tasks)
        };

        let mut task_results = HashMap::new();
        let mut successful_tasks = 0;
        let total_tasks = dag.tasks.len();
        let mut peak_concurrent_tasks = 0;

        // Capture the current process working directory at the start of this DAG
        // execution and use it as the effective working directory for all
        // ExecutionContext instances created for tasks in this DAG. This
        // avoids races when other tests or tasks change the global process
        // cwd concurrently.
        let base_cwd = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => std::path::PathBuf::from("."),
        };

        // Main execution loop - continue until all tasks are processed
        while execution_graph.has_pending_tasks() {
            // Start ready tasks
            while let Some(task_name) = execution_graph.get_next_ready_task() {
                if parallel_executor.can_start_task() {
                    if let Some(task) = dag.tasks.get(&task_name) {
                        println!("🔄 Starting task: {task_name}");

                        // Mark task as running
                        execution_graph.mark_task_running(&task_name);

                        // Create execution context for this task
                        let mut context = ExecutionContext::new(
                            dag.name.clone(),
                            task_name.clone(),
                            self.config.clone(),
                        );
                        context.set_working_directory(base_cwd.clone());

                        // Convert parser task to executor task
                        let executor_task = self.convert_parser_task_to_executor_task(task, &task_name);

                        // Check resource availability if monitoring is enabled
                        if let Some(resource_monitor) = &self.resource_monitor {
                            let estimated_resources = EstimatedResources::default(); // Could be improved with task-specific estimates
                            let availability = resource_monitor.check_resource_availability(&estimated_resources).await;
                            
                            if !availability.can_start_task {
                                println!("⏸️ Task '{}' queued due to resource limits: {}", 
                                    task_name, 
                                    availability.reason.unwrap_or_else(|| "Resource limit reached".to_string())
                                );
                                
                                // Queue the task for later execution
                                let queued_task = QueuedTask {
                                    task_id: task_name.clone(),
                                    queued_at: chrono::Utc::now(),
                                    priority: TaskPriority::Normal,
                                    estimated_resources,
                                };
                                resource_monitor.queue_task(queued_task).await;
                                continue;
                            }
                            
                            // Register task start with resource monitor
                            resource_monitor.register_task_start(&task_name).await?;
                        }

                        // Start task execution
                        let start_result = parallel_executor.start_task(
                            task_name.clone(),
                            executor_task,
                            context,
                            &self.task_registry,
                        ).await;
                        
                        // If task failed to start and resource monitoring is enabled, unregister it
                        if start_result.is_err() {
                            if let Some(resource_monitor) = &self.resource_monitor {
                                let _ = resource_monitor.register_task_completion(&task_name).await;
                            }
                        }
                        
                        start_result?;

                        // Update peak concurrent tasks
                        peak_concurrent_tasks = peak_concurrent_tasks.max(parallel_executor.running_task_count());
                    }
                } else {
                    // No more capacity, break and wait for tasks to complete
                    break;
                }
            }

            // Wait for at least one task to complete
            if let Some((task_name, result)) = parallel_executor.wait_for_next_completion().await {
                // Register task completion with resource monitor
                if let Some(resource_monitor) = &self.resource_monitor {
                    let _ = resource_monitor.register_task_completion(&task_name).await;
                }
                
                match result {
                    Ok(task_result) => {
                        // Check if the task actually failed despite successful execution
                        if task_result.status == TaskStatus::Failed {
                            println!(
                                "❌ Task '{}' failed: {}",
                                task_name,
                                task_result.error_message.as_deref().unwrap_or("Unknown error")
                            );
                            execution_graph.mark_task_failed(&task_name);
                        } else {
                            println!("✅ Task '{task_name}' completed successfully");
                            execution_graph.mark_task_completed(&task_name);
                            successful_tasks += 1;
                        }
                        task_results.insert(task_name.clone(), task_result);
                    }
                    Err(e) => {
                        println!("❌ Task '{task_name}' failed: {e}");
                        execution_graph.mark_task_failed(&task_name);

                        // Create a failed task result
                        let failed_result = TaskResult {
                            task_id: task_name.clone(),
                            status: TaskStatus::Failed,
                            started_at: chrono::Utc::now(),
                            completed_at: chrono::Utc::now(),
                            duration: std::time::Duration::from_secs(0),
                            stdout: String::new(),
                            stderr: format!("Task execution failed: {e}"),
                            exit_code: Some(1),
                            retry_count: 0,
                            output_data: None,
                            error_message: Some(e.to_string()),
                            metadata: std::collections::HashMap::new(),
                        };
                        task_results.insert(task_name.clone(), failed_result);
                    }
                }

                // Save intermediate state checkpoint if state management is enabled
                if let Some(state_manager) = &self.state_manager {
                    let intermediate_state = self.create_intermediate_execution_state(
                        &execution_id,
                        &dag.name,
                        &started_at,
                        &task_results,
                        ExecutionStatus::Running,
                    );
                    
                    if let Err(e) = state_manager.save_execution_state(&intermediate_state).await {
                        println!("⚠️ Failed to save intermediate state: {e}");
                    }
                }

                // Update ready tasks based on completed/failed task
                execution_graph.update_ready_tasks();
                
                // Process queued tasks if resource monitoring is enabled
                if let Some(resource_monitor) = &self.resource_monitor {
                    while let Some(queued_task) = resource_monitor.get_next_available_task().await {
                        if parallel_executor.can_start_task() {
                            // Try to start the queued task
                            if let Some(task) = dag.tasks.get(&queued_task.task_id) {
                                println!("🔄 Starting queued task: {}", queued_task.task_id);
                                
                                // Mark task as running
                                execution_graph.mark_task_running(&queued_task.task_id);
                                
                                // Create execution context for this task
                                let mut context = ExecutionContext::new(
                                    dag.name.clone(),
                                    queued_task.task_id.clone(),
                                    self.config.clone(),
                                );
                                context.set_working_directory(base_cwd.clone());
                                
                                // Convert parser task to executor task
                                let executor_task = self.convert_parser_task_to_executor_task(task, &queued_task.task_id);
                                
                                // Register task start with resource monitor
                                resource_monitor.register_task_start(&queued_task.task_id).await?;
                                
                                // Start task execution
                                let start_result = parallel_executor.start_task(
                                    queued_task.task_id.clone(),
                                    executor_task,
                                    context,
                                    &self.task_registry,
                                ).await;
                                
                                // If task failed to start, unregister it
                                if start_result.is_err() {
                                    let _ = resource_monitor.register_task_completion(&queued_task.task_id).await;
                                }
                                
                                start_result?;
                                
                                // Update peak concurrent tasks
                                peak_concurrent_tasks = peak_concurrent_tasks.max(parallel_executor.running_task_count());
                            }
                        } else {
                            // No capacity for more tasks, break
                            break;
                        }
                    }
                }
            }
        }

        // Wait for any remaining tasks to complete
        parallel_executor.wait_for_all_completion().await;

        let completed_at = chrono::Utc::now();
        let total_duration = start_time.elapsed();

        // Determine overall execution status
        let failed_tasks = execution_graph.failed_task_count();
        let skipped_tasks = execution_graph.skipped_task_count();
        let status = if failed_tasks == 0 {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Failed
        };

        // Create execution metrics
        let metrics = ExecutionMetrics {
            total_tasks,
            successful_tasks,
            failed_tasks,
            skipped_tasks,
            peak_concurrent_tasks,
            average_task_duration: if total_tasks > 0 {
                Some(total_duration / total_tasks as u32)
            } else {
                Some(std::time::Duration::from_secs(0))
            },
            total_cpu_time: None,
            peak_memory_usage: None,
            total_retries: 0,
            task_timeline: vec![],
        };

        println!(
            "📊 Parallel execution completed: {successful_tasks}/{total_tasks} tasks successful, peak concurrency: {peak_concurrent_tasks}"
        );
        if failed_tasks > 0 {
            println!("❌ Failed tasks: {failed_tasks}");
        }
        if skipped_tasks > 0 {
            println!("⏭️ Skipped tasks: {skipped_tasks}");
        }

        let result = ExecutionResult {
            dag_id: dag.name.clone(),
            execution_id: execution_id.clone(),
            status,
            started_at,
            completed_at: Some(completed_at),
            total_duration: Some(total_duration),
            task_results,
            metrics,
        };

        // Save final execution state if state management is enabled
        if let Some(state_manager) = &self.state_manager {
            let execution_state = ExecutionState::from(&result);
            if let Err(e) = state_manager.save_execution_state(&execution_state).await {
                println!("⚠️ Failed to save final execution state: {e}");
            } else {
                println!("💾 Final execution state saved");
            }
        }

        // Stop resource monitoring if it was enabled
        if let Some(resource_monitor) = &self.resource_monitor {
            resource_monitor.stop_monitoring().await;
            
            // Print resource usage summary
            let final_usage = resource_monitor.get_current_usage().await;
            println!("📊 Final resource usage: CPU: {:.1}%, Memory: {:.1}%", 
                final_usage.cpu_percent, final_usage.memory_percent);
            
            let queue_status = resource_monitor.get_queue_status().await;
            if queue_status.queued_tasks > 0 {
                println!("⚠️ {} tasks remained queued due to resource limits", queue_status.queued_tasks);
            }
        }

        Ok(result)
    }

    /// Execute a single task
    pub async fn execute_task(
        &self,
        task: &Task,
        context: &ExecutionContext,
    ) -> Result<TaskResult> {
        // Acquire semaphore permit for concurrency control
        let _permit = self.execution_semaphore.acquire().await?;

        // Get appropriate executor for task type
        let executor = self.task_registry.get_executor(&task.task_type)?;

        // Execute the task
        executor.execute(task, context).await.map_err(|e| e.into())
    }

    /// Get current execution status
    pub async fn get_execution_status(&self, execution_id: &str) -> Option<ExecutionStatus> {
        if let Some(state_manager) = &self.state_manager {
            if let Ok(Some(state)) = state_manager.load_execution_state(execution_id).await {
                return Some(state.status);
            }
        }
        None
    }

    /// Resume execution from a previous interrupted state
    pub async fn resume_execution(&self, execution_id: &str, dag: &Dag) -> Result<ExecutionResult> {
        let state_manager = self.state_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("State management not enabled"))?;

        // Load previous execution state
        let previous_state = state_manager.load_execution_state(execution_id).await?
            .ok_or_else(|| anyhow::anyhow!("Execution state not found: {}", execution_id))?;

        println!("🔄 Resuming execution: {} (DAG: {})", execution_id, previous_state.dag_id);

        // Determine which tasks are already completed
        let completed_tasks: HashSet<String> = previous_state.task_states.iter()
            .filter(|(_, state)| matches!(state.status, TaskStatus::Success))
            .map(|(task_id, _)| task_id.clone())
            .collect();

        let failed_tasks: HashSet<String> = previous_state.task_states.iter()
            .filter(|(_, state)| matches!(state.status, TaskStatus::Failed))
            .map(|(task_id, _)| task_id.clone())
            .collect();

        println!("✅ Already completed tasks: {:?}", completed_tasks);
        println!("❌ Previously failed tasks: {:?}", failed_tasks);

        // Create a modified DAG with only remaining tasks
        let mut remaining_dag = dag.clone();
        
        // Remove completed tasks from the DAG
        remaining_dag.tasks.retain(|task_id, _| !completed_tasks.contains(task_id));
        
        // Remove dependencies that involve completed tasks
        remaining_dag.dependencies.retain(|dep| {
            !completed_tasks.contains(&dep.task) && !completed_tasks.contains(&dep.depends_on)
        });

        if remaining_dag.tasks.is_empty() {
            println!("✅ All tasks already completed, nothing to resume");
            
            // Reconstruct the final result from saved state
            let mut task_results = HashMap::new();
            for (task_id, task_state) in previous_state.task_states {
                task_results.insert(task_id.clone(), TaskResult {
                    task_id: task_state.task_id,
                    status: task_state.status,
                    started_at: task_state.started_at,
                    completed_at: task_state.completed_at.unwrap_or(task_state.started_at),
                    duration: task_state.duration.unwrap_or(Duration::from_secs(0)),
                    stdout: task_state.stdout,
                    stderr: task_state.stderr,
                    exit_code: task_state.exit_code,
                    retry_count: task_state.retry_count,
                    output_data: task_state.output_data,
                    error_message: task_state.error_message,
                    metadata: task_state.metadata,
                });
            }

            return Ok(ExecutionResult {
                dag_id: previous_state.dag_id,
                execution_id: previous_state.execution_id,
                status: previous_state.status,
                started_at: previous_state.started_at,
                completed_at: previous_state.completed_at,
                total_duration: previous_state.total_duration,
                task_results,
                metrics: ExecutionMetrics::default(), // Could be reconstructed from state
            });
        }

        println!("🔄 Resuming execution with {} remaining tasks", remaining_dag.tasks.len());

        // Execute remaining tasks
        let mut result = self.execute_dag_internal(&remaining_dag, Some(execution_id.to_string())).await?;

        // Merge results with previous completed tasks
        for (task_id, task_state) in previous_state.task_states {
            if completed_tasks.contains(&task_id) {
                result.task_results.insert(task_id.clone(), TaskResult {
                    task_id: task_state.task_id,
                    status: task_state.status,
                    started_at: task_state.started_at,
                    completed_at: task_state.completed_at.unwrap_or(task_state.started_at),
                    duration: task_state.duration.unwrap_or(Duration::from_secs(0)),
                    stdout: task_state.stdout,
                    stderr: task_state.stderr,
                    exit_code: task_state.exit_code,
                    retry_count: task_state.retry_count,
                    output_data: task_state.output_data,
                    error_message: task_state.error_message,
                    metadata: task_state.metadata,
                });
            }
        }

        // Update metrics to include all tasks
        result.metrics.total_tasks = dag.tasks.len();
        result.metrics.successful_tasks = result.task_results.values()
            .filter(|r| r.status == TaskStatus::Success).count();
        result.metrics.failed_tasks = result.task_results.values()
            .filter(|r| r.status == TaskStatus::Failed).count();

        // Use original execution start time
        result.started_at = previous_state.started_at;
        if let Some(completed_at) = result.completed_at {
            result.total_duration = Some(completed_at.signed_duration_since(previous_state.started_at).to_std()?);
        }

        Ok(result)
    }

    /// List executions that can be resumed (interrupted or failed executions)
    pub async fn list_recoverable_executions(&self) -> Result<Vec<ExecutionState>> {
        let state_manager = self.state_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("State management not enabled"))?;

        let all_executions = state_manager.list_executions(None, 100).await?;
        
        // Filter for executions that can be resumed
        let recoverable: Vec<ExecutionState> = all_executions.into_iter()
            .filter(|exec| matches!(exec.status, ExecutionStatus::Running | ExecutionStatus::Failed))
            .collect();

        Ok(recoverable)
    }

    /// Check if an execution can be resumed
    pub async fn can_resume_execution(&self, execution_id: &str) -> Result<bool> {
        let state_manager = self.state_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("State management not enabled"))?;

        if let Some(state) = state_manager.load_execution_state(execution_id).await? {
            // Can resume if execution was running or failed (but not if it was successful)
            Ok(matches!(state.status, ExecutionStatus::Running | ExecutionStatus::Failed))
        } else {
            Ok(false)
        }
    }

    /// Resolve task dependencies and return execution order
    fn resolve_dependencies(&self, dag: &Dag) -> Result<Vec<String>> {
        let mut execution_order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        // Topological sort using DFS
        for task_name in dag.tasks.keys() {
            if !visited.contains(task_name) {
                self.dfs_resolve_dependencies(
                    dag,
                    task_name,
                    &mut visited,
                    &mut visiting,
                    &mut execution_order,
                )?;
            }
        }

        // Reverse the order to get correct execution sequence
        execution_order.reverse();

        Ok(execution_order)
    }

    /// DFS helper for dependency resolution
    fn dfs_resolve_dependencies(
        &self,
        dag: &Dag,
        task_name: &str,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
        execution_order: &mut Vec<String>,
    ) -> Result<()> {
        if visiting.contains(task_name) {
            return Err(ExecutorError::DependencyCycle.into());
        }

        if visited.contains(task_name) {
            return Ok(());
        }

        visiting.insert(task_name.to_string());

        // Process dependencies first - check DAG dependencies
        for dependency in &dag.dependencies {
            if dependency.task == task_name {
                self.dfs_resolve_dependencies(
                    dag,
                    &dependency.depends_on,
                    visited,
                    visiting,
                    execution_order,
                )?;
            }
        }

        visiting.remove(task_name);
        visited.insert(task_name.to_string());

        // Add current task to the beginning of the order (dependencies first)
        execution_order.insert(0, task_name.to_string());

        Ok(())
    }

    /// Create intermediate execution state for checkpointing
    fn create_intermediate_execution_state(
        &self,
        execution_id: &str,
        dag_id: &str,
        started_at: &DateTime<Utc>,
        task_results: &HashMap<String, TaskResult>,
        status: ExecutionStatus,
    ) -> ExecutionState {
        let mut task_states = HashMap::new();
        
        for (task_id, task_result) in task_results {
            task_states.insert(task_id.clone(), StateTaskState {
                task_id: task_result.task_id.clone(),
                status: task_result.status.clone(),
                started_at: task_result.started_at,
                completed_at: Some(task_result.completed_at),
                duration: Some(task_result.duration),
                stdout: task_result.stdout.clone(),
                stderr: task_result.stderr.clone(),
                exit_code: task_result.exit_code,
                retry_count: task_result.retry_count,
                error_message: task_result.error_message.clone(),
                output_data: task_result.output_data.clone(),
                metadata: task_result.metadata.clone(),
            });
        }

        ExecutionState {
            execution_id: execution_id.to_string(),
            dag_id: dag_id.to_string(),
            status,
            started_at: *started_at,
            completed_at: None, // Still running
            total_duration: None, // Still running
            task_states,
            metadata: HashMap::new(),
        }
    }

    /// Convert parser task to executor task
    fn convert_parser_task_to_executor_task(
        &self,
        parser_task: &crate::parser::Task,
        task_name: &str,
    ) -> crate::executor::traits::Task {
        use crate::executor::traits::{Task, TaskConfig};
        use std::collections::HashMap;

        // Determine task type
        let task_type = parser_task
            .config
            .task_type
            .clone()
            .unwrap_or_else(|| "shell".to_string());

        // Convert task config based on type
        let config = match task_type.as_str() {
            "shell" => TaskConfig::Shell {
                command: parser_task.config.command.clone().unwrap_or_default(),
                working_dir: None,
                shell: None,
            },
            "python" => {
                if let Some(script) = &parser_task.config.script {
                    TaskConfig::PythonScript {
                        script: script.clone(),
                        args: vec![],
                        python_path: None,
                    }
                } else if let (Some(module), Some(function)) =
                    (&parser_task.config.module, &parser_task.config.function)
                {
                    TaskConfig::PythonFunction {
                        module: module.clone(),
                        function: function.clone(),
                        args: parser_task.config.args.clone().unwrap_or_default(),
                        kwargs: parser_task.config.kwargs.clone().unwrap_or_default(),
                    }
                } else {
                    // Default to shell if no valid Python config
                    TaskConfig::Shell {
                        command: "echo 'No valid Python configuration'".to_string(),
                        working_dir: None,
                        shell: None,
                    }
                }
            }
            "http" => TaskConfig::HttpRequest {
                method: parser_task
                    .config
                    .method
                    .clone()
                    .unwrap_or_else(|| "GET".to_string()),
                url: parser_task.config.url.clone().unwrap_or_default(),
                headers: parser_task.config.headers.clone().unwrap_or_default(),
                body: parser_task.config.data.as_ref().map(|d| d.to_string()),
                auth: None,
                timeout: None,
            },
            "file" => {
                let mut options = HashMap::new();

                // Convert script to content for file operations
                if let Some(script) = &parser_task.config.script {
                    options.insert("content".to_string(), serde_json::json!(script));
                }

                // Determine operation type based on available fields
                let operation = if parser_task.config.source.is_some()
                    && parser_task.config.destination.is_some()
                {
                    "copy".to_string()
                } else if parser_task.config.source.is_some() {
                    "delete".to_string()
                } else {
                    "create".to_string()
                };

                TaskConfig::File {
                    operation,
                    source: parser_task.config.source.clone(),
                    destination: parser_task.config.destination.clone(),
                    options,
                }
            }
            _ => TaskConfig::Shell {
                command: format!("echo 'Unsupported task type: {task_type}'"),
                working_dir: None,
                shell: None,
            },
        };

        Task {
            id: task_name.to_string(),
            name: parser_task.name.clone(),
            task_type,
            config,
            timeout: None,
            retry_config: None,
            environment: HashMap::new(),
            environment_config: None,
            depends_on: vec![], // Will be handled by dependency resolution
            metadata: HashMap::new(),
        }
    }
}

impl TaskRegistry {
    /// Create a new task registry
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
        }
    }

    /// Create a new task registry with default executors
    pub fn with_default_executors() -> Self {
        let mut registry = Self::new();

        // Register shell executor
        registry.register_executor("shell".to_string(), Box::new(ShellTaskExecutor::new()));

        // Register Python executor
        registry.register_executor("python".to_string(), Box::new(PythonTaskExecutor::new()));

        // Register HTTP executor
        registry.register_executor(
            "http".to_string(),
            Box::new(HttpTaskExecutor::new().expect("Failed to create HTTP executor")),
        );

        // Register file executor
        registry.register_executor("file".to_string(), Box::new(FileTaskExecutor::new()));

        registry
    }

    /// Register a task executor for a specific task type
    pub fn register_executor(&mut self, task_type: String, executor: Box<dyn TaskExecutor>) {
        println!("📝 Registering executor for task type: {task_type}");
        self.executors.insert(task_type, executor);
    }

    /// Get executor for a task type
    pub fn get_executor(&self, task_type: &str) -> Result<&dyn TaskExecutor> {
        self.executors
            .get(task_type)
            .map(|e| e.as_ref())
            .ok_or_else(|| {
                ExecutorError::UnsupportedTaskType {
                    task_type: task_type.to_string(),
                }
                .into()
            })
    }

    /// Check if an executor is registered for a task type
    pub fn has_executor(&self, task_type: &str) -> bool {
        self.executors.contains_key(task_type)
    }

    /// Get all registered task types
    pub fn registered_task_types(&self) -> Vec<&String> {
        self.executors.keys().collect()
    }

    /// Get the number of registered executors
    pub fn executor_count(&self) -> usize {
        self.executors.len()
    }

    /// Validate that all tasks in a DAG have registered executors
    pub fn validate_dag_tasks(&self, dag: &Dag) -> Result<()> {
        let mut missing_executors = Vec::new();

        for task in dag.tasks.values() {
            // Extract task type from the parser's task structure
            if let Some(task_type) = &task.config.task_type {
                if !self.has_executor(task_type) {
                    missing_executors.push(task_type.clone());
                }
            } else {
                // If no task type is specified, we can't validate it
                missing_executors.push(format!("unknown_type_for_{}", task.name));
            }
        }

        if !missing_executors.is_empty() {
            return Err(ExecutorError::TaskRegistryError {
                message: format!(
                    "No executors registered for task types: {}",
                    missing_executors.join(", ")
                ),
            }
            .into());
        }

        Ok(())
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::state::TaskState as StateTaskState;
    use crate::parser::{Dag, Task as ParserTask, TaskConfig as ParserTaskConfig, Dependency};
    use chrono::Utc;
    use std::collections::HashMap;
    use std::time::Duration;

    #[tokio::test]
    async fn test_parallel_execution_basic() {
        // Create a simple DAG with parallel tasks
        let mut dag = Dag {
            name: "test_parallel".to_string(),
            schedule: None,
            tasks: HashMap::new(),
            dependencies: Vec::new(),
        };

        // Add two independent tasks that can run in parallel
        let task1 = ParserTask {
            name: "task1".to_string(),
            description: Some("First task".to_string()),
            config: ParserTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Task 1'".to_string()),
                ..Default::default()
            },
        };

        let task2 = ParserTask {
            name: "task2".to_string(),
            description: Some("Second task".to_string()),
            config: ParserTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Task 2'".to_string()),
                ..Default::default()
            },
        };

        dag.tasks.insert("task1".to_string(), task1);
        dag.tasks.insert("task2".to_string(), task2);

        // Create executor with concurrency of 2
        let config = ExecutorConfig {
            max_concurrent_tasks: 2,
            ..Default::default()
        };
        let executor = NativeExecutor::new(config);

        // Execute the DAG
        let result = executor.execute_dag(&dag).await.unwrap();

        // Verify execution was successful
        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.task_results.len(), 2);
        assert!(result.task_results.contains_key("task1"));
        assert!(result.task_results.contains_key("task2"));

        // Both tasks should have succeeded
        assert_eq!(result.task_results["task1"].status, TaskStatus::Success);
        assert_eq!(result.task_results["task2"].status, TaskStatus::Success);

        // Peak concurrent tasks should be 2 (both ran in parallel)
        assert_eq!(result.metrics.peak_concurrent_tasks, 2);
    }

    #[tokio::test]
    async fn test_dependency_ordering() {
        // Create a DAG with dependencies: task1 -> task2 -> task3
        let mut dag = Dag {
            name: "test_dependencies".to_string(),
            schedule: None,
            tasks: HashMap::new(),
            dependencies: Vec::new(),
        };

        // Add tasks
        let task1 = ParserTask {
            name: "task1".to_string(),
            description: Some("First task".to_string()),
            config: ParserTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Task 1'".to_string()),
                ..Default::default()
            },
        };

        let task2 = ParserTask {
            name: "task2".to_string(),
            description: Some("Second task".to_string()),
            config: ParserTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Task 2'".to_string()),
                ..Default::default()
            },
        };

        let task3 = ParserTask {
            name: "task3".to_string(),
            description: Some("Third task".to_string()),
            config: ParserTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Task 3'".to_string()),
                ..Default::default()
            },
        };

        dag.tasks.insert("task1".to_string(), task1);
        dag.tasks.insert("task2".to_string(), task2);
        dag.tasks.insert("task3".to_string(), task3);

        // Add dependencies
        dag.dependencies.push(Dependency {
            task: "task2".to_string(),
            depends_on: "task1".to_string(),
        });
        dag.dependencies.push(Dependency {
            task: "task3".to_string(),
            depends_on: "task2".to_string(),
        });

        // Create executor
        let config = ExecutorConfig {
            max_concurrent_tasks: 3,
            ..Default::default()
        };
        let executor = NativeExecutor::new(config);

        // Execute the DAG
        let result = executor.execute_dag(&dag).await.unwrap();

        // Verify execution was successful
        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.task_results.len(), 3);

        // All tasks should have succeeded
        assert_eq!(result.task_results["task1"].status, TaskStatus::Success);
        assert_eq!(result.task_results["task2"].status, TaskStatus::Success);
        assert_eq!(result.task_results["task3"].status, TaskStatus::Success);

        // Verify execution order by checking timestamps
        let task1_time = result.task_results["task1"].started_at;
        let task2_time = result.task_results["task2"].started_at;
        let task3_time = result.task_results["task3"].started_at;

        assert!(task2_time >= task1_time); // task2 should start after task1
        assert!(task3_time >= task2_time); // task3 should start after task2
    }

    #[tokio::test]
    async fn test_execution_with_state_management() {
        // Create a simple DAG
        let mut dag = Dag {
            name: "test_state_management".to_string(),
            schedule: None,
            tasks: HashMap::new(),
            dependencies: Vec::new(),
        };

        let task1 = ParserTask {
            name: "task1".to_string(),
            description: Some("First task".to_string()),
            config: ParserTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Task 1'".to_string()),
                ..Default::default()
            },
        };

        dag.tasks.insert("task1".to_string(), task1);

        // Create executor with state management
        let executor_config = ExecutorConfig {
            max_concurrent_tasks: 2,
            ..Default::default()
        };
        let state_config = StateConfig {
            database_path: ":memory:".to_string(),
            ..Default::default()
        };

        let executor = NativeExecutor::with_state_management(executor_config, state_config).await.unwrap();

        // Execute the DAG
        let result = executor.execute_dag(&dag).await.unwrap();

        // Verify execution was successful
        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.task_results.len(), 1);

        // Verify state was saved
        let status = executor.get_execution_status(&result.execution_id).await;
        assert_eq!(status, Some(ExecutionStatus::Success));
    }

    #[tokio::test]
    async fn test_execution_recovery() {
        // Create a DAG with multiple tasks
        let mut dag = Dag {
            name: "test_recovery".to_string(),
            schedule: None,
            tasks: HashMap::new(),
            dependencies: Vec::new(),
        };

        let task1 = ParserTask {
            name: "task1".to_string(),
            description: Some("First task".to_string()),
            config: ParserTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Task 1'".to_string()),
                ..Default::default()
            },
        };

        let task2 = ParserTask {
            name: "task2".to_string(),
            description: Some("Second task".to_string()),
            config: ParserTaskConfig {
                task_type: Some("shell".to_string()),
                command: Some("echo 'Task 2'".to_string()),
                ..Default::default()
            },
        };

        dag.tasks.insert("task1".to_string(), task1);
        dag.tasks.insert("task2".to_string(), task2);

        // Add dependency: task2 depends on task1
        dag.dependencies.push(Dependency {
            task: "task2".to_string(),
            depends_on: "task1".to_string(),
        });

        // Create executor with state management
        let executor_config = ExecutorConfig {
            max_concurrent_tasks: 2,
            ..Default::default()
        };
        let state_config = StateConfig {
            database_path: ":memory:".to_string(),
            ..Default::default()
        };

        let executor = NativeExecutor::with_state_management(executor_config, state_config).await.unwrap();

        // Simulate a partial execution by manually creating state
        let execution_id = Uuid::new_v4().to_string();
        let mut task_states = HashMap::new();
        
        // Task1 completed successfully
        task_states.insert("task1".to_string(), StateTaskState {
            task_id: "task1".to_string(),
            status: TaskStatus::Success,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            duration: Some(Duration::from_secs(1)),
            stdout: "Task 1".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            retry_count: 0,
            error_message: None,
            output_data: None,
            metadata: HashMap::new(),
        });

        let partial_state = ExecutionState {
            execution_id: execution_id.clone(),
            dag_id: dag.name.clone(),
            status: ExecutionStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            total_duration: None,
            task_states,
            metadata: HashMap::new(),
        };

        // Save the partial state
        if let Some(state_manager) = &executor.state_manager {
            state_manager.save_execution_state(&partial_state).await.unwrap();
        }

        // Resume execution
        let result = executor.resume_execution(&execution_id, &dag).await.unwrap();

        // Verify both tasks completed
        assert_eq!(result.status, ExecutionStatus::Success);
        assert_eq!(result.task_results.len(), 2);
        assert_eq!(result.task_results["task1"].status, TaskStatus::Success);
        assert_eq!(result.task_results["task2"].status, TaskStatus::Success);
    }
}




