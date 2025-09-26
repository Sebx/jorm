//! Result types for task and DAG execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Result of executing a complete DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// DAG identifier
    pub dag_id: String,
    
    /// Unique execution identifier
    pub execution_id: String,
    
    /// Overall execution status
    pub status: ExecutionStatus,
    
    /// When execution started
    pub started_at: DateTime<Utc>,
    
    /// When execution completed (if finished)
    pub completed_at: Option<DateTime<Utc>>,
    
    /// Total execution duration
    pub total_duration: Option<Duration>,
    
    /// Results from individual tasks
    pub task_results: HashMap<String, TaskResult>,
    
    /// Execution metrics
    pub metrics: ExecutionMetrics,
}

/// Result of executing a single task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task identifier
    pub task_id: String,
    
    /// Task execution status
    pub status: TaskStatus,
    
    /// Standard output
    pub stdout: String,
    
    /// Standard error
    pub stderr: String,
    
    /// Exit code (if applicable)
    pub exit_code: Option<i32>,
    
    /// Execution duration
    pub duration: Duration,
    
    /// Number of retry attempts made
    pub retry_count: u32,
    
    /// When task started
    pub started_at: DateTime<Utc>,
    
    /// When task completed
    pub completed_at: DateTime<Utc>,
    
    /// Structured output data (if any)
    pub output_data: Option<serde_json::Value>,
    
    /// Error message (if failed)
    pub error_message: Option<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Execution status for DAGs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Execution is pending
    Pending,
    
    /// Execution is currently running
    Running,
    
    /// Execution completed successfully
    Success,
    
    /// Execution failed
    Failed,
    
    /// Execution was cancelled
    Cancelled,
    
    /// Execution timed out
    Timeout,
    
    /// Execution was partially successful (some tasks failed)
    PartialSuccess,
}

/// Execution status for individual tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,
    
    /// Task is currently running
    Running,
    
    /// Task completed successfully
    Success,
    
    /// Task failed
    Failed,
    
    /// Task was cancelled
    Cancelled,
    
    /// Task timed out
    Timeout,
    
    /// Task was skipped due to dependency failure
    Skipped,
    
    /// Task is being retried
    Retrying,
}

/// Metrics collected during execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Total number of tasks
    pub total_tasks: usize,
    
    /// Number of successful tasks
    pub successful_tasks: usize,
    
    /// Number of failed tasks
    pub failed_tasks: usize,
    
    /// Number of skipped tasks
    pub skipped_tasks: usize,
    
    /// Peak concurrent tasks
    pub peak_concurrent_tasks: usize,
    
    /// Average task duration
    pub average_task_duration: Option<Duration>,
    
    /// Total CPU time used
    pub total_cpu_time: Option<Duration>,
    
    /// Peak memory usage (bytes)
    pub peak_memory_usage: Option<u64>,
    
    /// Number of retries performed
    pub total_retries: u32,
    
    /// Task execution timeline
    pub task_timeline: Vec<TaskTimelineEntry>,
}

/// Entry in the task execution timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTimelineEntry {
    /// Task identifier
    pub task_id: String,
    
    /// Event type
    pub event: TaskEvent,
    
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    
    /// Duration since execution start
    pub elapsed: Duration,
}

/// Task execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskEvent {
    /// Task was queued for execution
    Queued,
    
    /// Task execution started
    Started,
    
    /// Task execution completed
    Completed { status: TaskStatus },
    
    /// Task retry attempt
    RetryAttempt { attempt: u32 },
}

impl ExecutionResult {
    /// Create a new execution result
    pub fn new(dag_id: String, execution_id: String) -> Self {
        Self {
            dag_id,
            execution_id,
            status: ExecutionStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
            total_duration: None,
            task_results: HashMap::new(),
            metrics: ExecutionMetrics::default(),
        }
    }
    
    /// Mark execution as completed
    pub fn complete(&mut self, status: ExecutionStatus) {
        self.status = status;
        self.completed_at = Some(Utc::now());
        
        if let Some(completed_at) = self.completed_at {
            self.total_duration = Some(
                completed_at.signed_duration_since(self.started_at)
                    .to_std()
                    .unwrap_or_default()
            );
        }
    }
    
    /// Add a task result
    pub fn add_task_result(&mut self, result: TaskResult) {
        self.task_results.insert(result.task_id.clone(), result);
        self.update_metrics();
    }
    
    /// Update metrics based on current task results
    fn update_metrics(&mut self) {
        self.metrics.total_tasks = self.task_results.len();
        self.metrics.successful_tasks = self.task_results.values()
            .filter(|r| r.status == TaskStatus::Success)
            .count();
        self.metrics.failed_tasks = self.task_results.values()
            .filter(|r| matches!(r.status, TaskStatus::Failed | TaskStatus::Timeout))
            .count();
        self.metrics.skipped_tasks = self.task_results.values()
            .filter(|r| r.status == TaskStatus::Skipped)
            .count();
        
        // Calculate average task duration
        let durations: Vec<Duration> = self.task_results.values()
            .map(|r| r.duration)
            .collect();
        
        if !durations.is_empty() {
            let total_duration: Duration = durations.iter().sum();
            self.metrics.average_task_duration = Some(total_duration / durations.len() as u32);
        }
        
        // Sum total retries
        self.metrics.total_retries = self.task_results.values()
            .map(|r| r.retry_count)
            .sum();
    }
    
    /// Check if execution was successful
    pub fn is_successful(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success | ExecutionStatus::PartialSuccess)
    }
    
    /// Get success rate (percentage of successful tasks)
    pub fn success_rate(&self) -> f64 {
        if self.metrics.total_tasks == 0 {
            return 0.0;
        }
        
        (self.metrics.successful_tasks as f64 / self.metrics.total_tasks as f64) * 100.0
    }
}

impl TaskResult {
    /// Create a new task result
    pub fn new(task_id: String) -> Self {
        let now = Utc::now();
        
        Self {
            task_id,
            status: TaskStatus::Pending,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            duration: Duration::default(),
            retry_count: 0,
            started_at: now,
            completed_at: now,
            output_data: None,
            error_message: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Mark task as completed with the given status
    pub fn complete(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self.completed_at = Utc::now();
        self.duration = self.completed_at.signed_duration_since(self.started_at)
            .to_std()
            .unwrap_or_default();
        self
    }
    
    /// Set stdout
    pub fn with_stdout(mut self, stdout: String) -> Self {
        self.stdout = stdout;
        self
    }
    
    /// Set stderr
    pub fn with_stderr(mut self, stderr: String) -> Self {
        self.stderr = stderr;
        self
    }
    
    /// Set exit code
    pub fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = Some(exit_code);
        self
    }
    
    /// Set error message
    pub fn with_error(mut self, error: String) -> Self {
        self.error_message = Some(error);
        self
    }
    
    /// Check if task was successful
    pub fn is_successful(&self) -> bool {
        self.status == TaskStatus::Success
    }
}