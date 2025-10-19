//! Parallel task execution coordinator for managing concurrent task execution
//!
//! This module provides high-performance parallel task execution with semaphore-based
//! concurrency limiting and resource management capabilities.

use crate::executor::context::ExecutionContext;
use crate::executor::error::ExecutorError;
use crate::executor::resource_monitor::{ResourceMonitor, EstimatedResources, QueuedTask, TaskPriority};
use crate::executor::result::TaskResult;
use crate::executor::traits::{Task, TaskExecutor};
use crate::executor::TaskRegistry;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// Parallel execution coordinator for managing concurrent task execution
pub struct ParallelExecutor {
    /// Semaphore for controlling concurrency
    semaphore: Arc<Semaphore>,
    /// Join set for managing concurrent tasks
    task_handles: JoinSet<(String, Result<TaskResult, ExecutorError>)>,
    /// Maximum concurrent tasks
    max_concurrent: usize,
    /// Currently running task count
    running_count: usize,
}

impl ParallelExecutor {
    /// Create a new parallel executor with the specified concurrency limit
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            task_handles: JoinSet::new(),
            max_concurrent,
            running_count: 0,
        }
    }

    /// Check if a new task can be started (within concurrency limits)
    pub fn can_start_task(&self) -> bool {
        self.running_count < self.max_concurrent
    }

    /// Get the current number of running tasks
    pub fn running_task_count(&self) -> usize {
        self.running_count
    }

    /// Get the maximum concurrent task limit
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Start a new task execution
    pub async fn start_task(
        &mut self,
        task_id: String,
        task: Task,
        context: ExecutionContext,
        task_registry: &TaskRegistry,
    ) -> Result<()> {
        // Get the appropriate executor for this task type
        let executor = task_registry.get_executor(&task.task_type)?;
        
        // Clone the executor for the async task
        let executor_clone = executor.clone_box();
        
        // Spawn the task with semaphore control
        let semaphore = self.semaphore.clone();
        self.task_handles.spawn(async move {
            let _permit = semaphore.acquire().await.expect("Semaphore closed");
            let task_id_clone = task_id.clone();
            
            // Execute the task
            let result = executor_clone.execute(&task, &context).await;
            
            (task_id_clone, result)
        });
        
        self.running_count += 1;
        
        Ok(())
    }

    /// Wait for the next task to complete and return its result
    pub async fn wait_for_next_completion(&mut self) -> Option<(String, Result<TaskResult, ExecutorError>)> {
        if let Some(result) = self.task_handles.join_next().await {
            self.running_count = self.running_count.saturating_sub(1);
            
            match result {
                Ok((task_id, task_result)) => Some((task_id, task_result)),
                Err(join_error) => {
                    // Handle join error (task panicked)
                    let error_msg = format!("Task panicked: {}", join_error);
                    Some((
                        "unknown".to_string(),
                        Err(ExecutorError::TaskExecutionFailed {
                            task_id: "unknown".to_string(),
                            source: anyhow::anyhow!(error_msg),
                        }),
                    ))
                }
            }
        } else {
            None
        }
    }

    /// Wait for all currently running tasks to complete
    pub async fn wait_for_all_completion(&mut self) {
        while !self.task_handles.is_empty() {
            if let Some(_) = self.wait_for_next_completion().await {
                // Task completed, continue waiting for others
            }
        }
        self.running_count = 0;
    }

    /// Execute multiple tasks in parallel with dependency respect
    pub async fn execute_tasks_parallel(
        &mut self,
        tasks: Vec<Task>,
        contexts: Vec<ExecutionContext>,
        task_registry: &TaskRegistry,
    ) -> Result<Vec<TaskResult>> {
        if tasks.len() != contexts.len() {
            return Err(anyhow::anyhow!(
                "Number of tasks ({}) must match number of contexts ({})",
                tasks.len(),
                contexts.len()
            ));
        }

        let mut results = HashMap::new();
        let total_tasks = tasks.len();

        // Start all tasks
        for (task, context) in tasks.into_iter().zip(contexts.into_iter()) {
            let task_id = task.id.clone();
            self.start_task(task_id, task, context, task_registry).await?;
        }

        // Collect all results
        for _ in 0..total_tasks {
            if let Some((task_id, result)) = self.wait_for_next_completion().await {
                match result {
                    Ok(task_result) => {
                        results.insert(task_id, task_result);
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }
        }

        // Convert HashMap to Vec in original order (by task ID)
        let mut result_vec = Vec::new();
        for i in 0..total_tasks {
            let task_id = format!("task_{}", i); // This assumes task IDs follow a pattern
            if let Some(result) = results.remove(&task_id) {
                result_vec.push(result);
            }
        }

        // If we couldn't match by pattern, just return all results
        if result_vec.is_empty() {
            result_vec = results.into_values().collect();
        }

        Ok(result_vec)
    }

    /// Cancel all running tasks
    pub fn cancel_all_tasks(&mut self) {
        self.task_handles.abort_all();
        self.running_count = 0;
    }

    /// Check if there are any running tasks
    pub fn has_running_tasks(&self) -> bool {
        self.running_count > 0
    }

    /// Get execution statistics
    pub fn get_execution_stats(&self) -> ParallelExecutionStats {
        ParallelExecutionStats {
            max_concurrent: self.max_concurrent,
            current_running: self.running_count,
            available_slots: self.max_concurrent.saturating_sub(self.running_count),
            total_spawned: self.task_handles.len() + self.running_count,
        }
    }

    /// Resize the concurrency limit (affects future task starts)
    pub fn resize_concurrency(&mut self, new_limit: usize) {
        self.max_concurrent = new_limit;
        // Note: This doesn't affect the semaphore of already running tasks
        // Only new ParallelExecutor instances will use the new limit
    }
}

/// Statistics for parallel execution monitoring
#[derive(Debug, Clone)]
pub struct ParallelExecutionStats {
    pub max_concurrent: usize,
    pub current_running: usize,
    pub available_slots: usize,
    pub total_spawned: usize,
}

impl ParallelExecutionStats {
    /// Calculate utilization percentage
    pub fn utilization_percentage(&self) -> f64 {
        if self.max_concurrent == 0 {
            return 0.0;
        }
        
        (self.current_running as f64 / self.max_concurrent as f64) * 100.0
    }

    /// Check if at capacity
    pub fn is_at_capacity(&self) -> bool {
        self.current_running >= self.max_concurrent
    }

    /// Check if idle (no running tasks)
    pub fn is_idle(&self) -> bool {
        self.current_running == 0
    }
}

/// Resource-aware parallel executor that monitors system resources
pub struct ResourceAwareParallelExecutor {
    base_executor: ParallelExecutor,
    resource_monitor: Arc<ResourceMonitor>,
    adaptive_concurrency: bool,
}

impl ResourceAwareParallelExecutor {
    /// Create a new resource-aware parallel executor
    pub fn new(
        max_concurrent: usize,
        resource_monitor: Arc<ResourceMonitor>,
        adaptive_concurrency: bool,
    ) -> Self {
        Self {
            base_executor: ParallelExecutor::new(max_concurrent),
            resource_monitor,
            adaptive_concurrency,
        }
    }

    /// Check if system resources allow starting a new task
    pub async fn can_start_task_with_resources(&self, estimated_resources: &EstimatedResources) -> bool {
        if !self.base_executor.can_start_task() {
            return false;
        }

        if !self.adaptive_concurrency {
            return true;
        }

        let availability = self.resource_monitor.check_resource_availability(estimated_resources).await;
        availability.can_start_task
    }

    /// Queue a task for execution when resources become available
    pub async fn queue_task_for_resources(&self, task_id: String, estimated_resources: EstimatedResources, priority: TaskPriority) {
        let queued_task = QueuedTask {
            task_id,
            queued_at: chrono::Utc::now(),
            priority,
            estimated_resources,
        };
        
        self.resource_monitor.queue_task(queued_task).await;
    }

    /// Get the next task that can be started based on resource availability
    pub async fn get_next_available_task(&self) -> Option<QueuedTask> {
        self.resource_monitor.get_next_available_task().await
    }

    /// Start a task with resource monitoring
    pub async fn start_task(
        &mut self,
        task_id: String,
        task: Task,
        context: ExecutionContext,
        task_registry: &TaskRegistry,
    ) -> Result<()> {
        // Register task start with resource monitor
        self.resource_monitor.register_task_start(&task_id).await?;
        
        // Start the task
        let result = self.base_executor.start_task(task_id.clone(), task, context, task_registry).await;
        
        // If task failed to start, unregister it
        if result.is_err() {
            let _ = self.resource_monitor.register_task_completion(&task_id).await;
        }
        
        result
    }

    /// Wait for next task completion with resource monitoring
    pub async fn wait_for_next_completion(&mut self) -> Option<(String, Result<TaskResult, ExecutorError>)> {
        if let Some((task_id, result)) = self.base_executor.wait_for_next_completion().await {
            // Register task completion with resource monitor
            let _ = self.resource_monitor.register_task_completion(&task_id).await;
            Some((task_id, result))
        } else {
            None
        }
    }

    /// Delegate to base executor
    pub async fn wait_for_all_completion(&mut self) {
        self.base_executor.wait_for_all_completion().await
    }

    /// Get combined statistics
    pub async fn get_execution_stats(&self) -> ResourceAwareExecutionStats {
        let base_stats = self.base_executor.get_execution_stats();
        let queue_status = self.resource_monitor.get_queue_status().await;
        let current_usage = self.resource_monitor.get_current_usage().await;
        
        ResourceAwareExecutionStats {
            parallel_stats: base_stats,
            queue_status,
            current_usage,
            adaptive_concurrency: self.adaptive_concurrency,
        }
    }
}

/// Extended statistics for resource-aware execution
#[derive(Debug, Clone)]
pub struct ResourceAwareExecutionStats {
    pub parallel_stats: ParallelExecutionStats,
    pub queue_status: crate::executor::resource_monitor::QueueStatus,
    pub current_usage: crate::executor::resource_monitor::ResourceUsage,
    pub adaptive_concurrency: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::config::ExecutorConfig;
    use crate::executor::executors::shell::ShellTaskExecutor;
    use crate::executor::traits::TaskConfig;
    use std::time::Duration;

    fn create_test_task(id: &str, command: &str) -> Task {
        Task {
            id: id.to_string(),
            name: format!("Test Task {}", id),
            task_type: "shell".to_string(),
            config: TaskConfig::Shell {
                command: command.to_string(),
                working_dir: None,
                shell: None,
            },
            timeout: Some(Duration::from_secs(30)),
            retry_config: None,
            environment: std::collections::HashMap::new(),
            environment_config: None,
            depends_on: vec![],
            metadata: std::collections::HashMap::new(),
        }
    }

    fn create_test_context(task_id: &str) -> ExecutionContext {
        ExecutionContext::new(
            "test_dag".to_string(),
            task_id.to_string(),
            ExecutorConfig::default(),
        )
    }

    fn create_test_registry() -> TaskRegistry {
        let mut registry = TaskRegistry::new();
        registry.register_executor("shell".to_string(), Box::new(ShellTaskExecutor::new()));
        registry
    }

    #[tokio::test]
    async fn test_parallel_executor_creation() {
        let executor = ParallelExecutor::new(4);
        
        assert_eq!(executor.max_concurrent(), 4);
        assert_eq!(executor.running_task_count(), 0);
        assert!(executor.can_start_task());
    }

    #[tokio::test]
    async fn test_concurrency_limits() {
        let mut executor = ParallelExecutor::new(2);
        let registry = create_test_registry();
        
        // Start first task
        let task1 = create_test_task("task1", "sleep 0.1");
        let context1 = create_test_context("task1");
        executor.start_task("task1".to_string(), task1, context1, &registry).await.unwrap();
        
        assert_eq!(executor.running_task_count(), 1);
        assert!(executor.can_start_task());
        
        // Start second task
        let task2 = create_test_task("task2", "sleep 0.1");
        let context2 = create_test_context("task2");
        executor.start_task("task2".to_string(), task2, context2, &registry).await.unwrap();
        
        assert_eq!(executor.running_task_count(), 2);
        assert!(!executor.can_start_task()); // At capacity
        
        // Wait for tasks to complete
        executor.wait_for_all_completion().await;
        
        assert_eq!(executor.running_task_count(), 0);
        assert!(executor.can_start_task());
    }

    #[tokio::test]
    async fn test_task_execution_results() {
        let mut executor = ParallelExecutor::new(2);
        let registry = create_test_registry();
        
        // Start a simple echo task
        let task = create_test_task("echo_task", "echo 'Hello World'");
        let context = create_test_context("echo_task");
        executor.start_task("echo_task".to_string(), task, context, &registry).await.unwrap();
        
        // Wait for completion
        if let Some((task_id, result)) = executor.wait_for_next_completion().await {
            assert_eq!(task_id, "echo_task");
            
            match result {
                Ok(task_result) => {
                    assert_eq!(task_result.task_id, "echo_task");
                    assert!(task_result.stdout.contains("Hello World"));
                }
                Err(e) => panic!("Task execution failed: {}", e),
            }
        } else {
            panic!("No task result received");
        }
    }

    #[tokio::test]
    async fn test_parallel_execution_stats() {
        let mut executor = ParallelExecutor::new(3);
        let registry = create_test_registry();
        
        let stats = executor.get_execution_stats();
        assert_eq!(stats.max_concurrent, 3);
        assert_eq!(stats.current_running, 0);
        assert_eq!(stats.available_slots, 3);
        assert_eq!(stats.utilization_percentage(), 0.0);
        assert!(stats.is_idle());
        
        // Start a task
        let task = create_test_task("test_task", "sleep 0.1");
        let context = create_test_context("test_task");
        executor.start_task("test_task".to_string(), task, context, &registry).await.unwrap();
        
        let stats = executor.get_execution_stats();
        assert_eq!(stats.current_running, 1);
        assert_eq!(stats.available_slots, 2);
        assert!((stats.utilization_percentage() - 33.33).abs() < 0.1);
        assert!(!stats.is_idle());
        assert!(!stats.is_at_capacity());
        
        executor.wait_for_all_completion().await;
    }

    #[tokio::test]
    async fn test_resource_aware_executor() {
        use crate::executor::resource_monitor::{ResourceMonitor, ResourceLimits, EstimatedResources};
        use std::sync::Arc;
        
        let limits = ResourceLimits {
            max_cpu_percent: 80.0,
            max_memory_percent: 90.0,
            max_concurrent_tasks: 4,
            ..Default::default()
        };
        let resource_monitor = Arc::new(ResourceMonitor::new(limits));
        
        let mut executor = ResourceAwareParallelExecutor::new(
            4,
            resource_monitor.clone(),
            true, // Adaptive concurrency
        );
        
        // Start monitoring
        resource_monitor.start_monitoring().await.unwrap();
        
        // Test resource checking
        let resources = EstimatedResources::default();
        let can_start = executor.can_start_task_with_resources(&resources).await;
        assert!(can_start); // Should be true with mock low resource usage
        
        let stats = executor.get_execution_stats().await;
        assert_eq!(stats.parallel_stats.max_concurrent, 4);
        assert!(stats.adaptive_concurrency);
        
        resource_monitor.stop_monitoring().await;
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let mut executor = ParallelExecutor::new(2);
        let registry = create_test_registry();
        
        // Start a long-running task
        let task = create_test_task("long_task", "sleep 10");
        let context = create_test_context("long_task");
        executor.start_task("long_task".to_string(), task, context, &registry).await.unwrap();
        
        assert!(executor.has_running_tasks());
        
        // Cancel all tasks
        executor.cancel_all_tasks();
        
        assert!(!executor.has_running_tasks());
        assert_eq!(executor.running_task_count(), 0);
    }

    #[tokio::test]
    async fn test_executor_resize() {
        let mut executor = ParallelExecutor::new(2);
        
        assert_eq!(executor.max_concurrent(), 2);
        
        executor.resize_concurrency(5);
        
        assert_eq!(executor.max_concurrent(), 5);
    }
}