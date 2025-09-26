//! Native Rust executor for JORM DAG execution
//! 
//! This module provides high-performance, parallel task execution capabilities
//! that replace the Python execution engine while maintaining full compatibility.

pub mod config;
pub mod error;
pub mod context;
pub mod traits;
pub mod result;
pub mod executors;
pub mod dataflow;


pub use config::*;
pub use error::*;
pub use context::*;
pub use traits::*;
pub use result::*;
pub use executors::*;
// pub use retry::*;

use crate::parser::Dag;
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::Semaphore;
use uuid::Uuid;

/// Main native executor for DAG execution
pub struct NativeExecutor {
    config: ExecutorConfig,
    task_registry: TaskRegistry,
    execution_semaphore: Semaphore,
}

/// Registry for task executors by task type
pub struct TaskRegistry {
    executors: HashMap<String, Box<dyn TaskExecutor>>,
}

impl NativeExecutor {
    /// Create a new native executor with the given configuration
    pub fn new(config: ExecutorConfig) -> Self {
        let execution_semaphore = Semaphore::new(config.max_concurrent_tasks);
        
        Self {
            config,
            task_registry: TaskRegistry::with_default_executors(),
            execution_semaphore,
        }
    }
    
    /// Create a new native executor with custom task registry
    pub fn with_task_registry(config: ExecutorConfig, task_registry: TaskRegistry) -> Self {
        let execution_semaphore = Semaphore::new(config.max_concurrent_tasks);
        
        Self {
            config,
            task_registry,
            execution_semaphore,
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
    
    /// Execute a complete DAG
    pub async fn execute_dag(&self, dag: &Dag) -> Result<ExecutionResult> {
        let execution_id = Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();
        let started_at = chrono::Utc::now();
        
        println!("🚀 Starting native DAG execution: {} (ID: {})", dag.name, execution_id);
        
        // Validate that all tasks have registered executors
        if let Err(e) = self.task_registry.validate_dag_tasks(dag) {
            println!("❌ DAG validation failed: {}", e);
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
        
        // Resolve task dependencies and create execution order
        let execution_order = self.resolve_dependencies(dag)?;
        println!("📋 Execution order: {:?}", execution_order);
        
        let mut task_results = HashMap::new();
        let mut failed_tasks = Vec::new();
        let mut successful_tasks = 0;
        let total_tasks = dag.tasks.len();
        
        // Execute tasks in dependency order
        for task_name in execution_order {
            // Check if any prerequisite task has failed
            let should_skip = dag.dependencies.iter()
                .filter(|dep| dep.task == task_name)
                .any(|dep| failed_tasks.contains(&dep.depends_on));
            
            if should_skip {
                println!("⏭️  Skipping task '{}' due to failed prerequisite", task_name);
                continue;
            }
            
            if let Some(task) = dag.tasks.get(&task_name) {
                println!("🔄 Executing task: {}", task_name);
                
                // Create execution context for this task
                let context = ExecutionContext::new(
                    dag.name.clone(),
                    task_name.clone(),
                    self.config.clone(),
                );
                
                // Convert parser task to executor task
                let executor_task = self.convert_parser_task_to_executor_task(task, &task_name);
                
                // Execute the task
                match self.execute_task(&executor_task, &context).await {
                    Ok(result) => {
                        task_results.insert(task_name.clone(), result.clone());
                        
                        // Check if the task actually failed despite successful execution
                        if result.status == TaskStatus::Failed {
                            println!("❌ Task '{}' failed: {}", task_name, result.error_message.as_deref().unwrap_or("Unknown error"));
                            failed_tasks.push(task_name.clone());
                        } else {
                            println!("✅ Task '{}' completed successfully", task_name);
                            successful_tasks += 1;
                        }
                    }
                    Err(e) => {
                        println!("❌ Task '{}' failed: {}", task_name, e);
                        failed_tasks.push(task_name.clone());
                        
                        // Create a failed task result
                        let failed_result = TaskResult {
                            task_id: task_name.clone(),
                            status: TaskStatus::Failed,
                            started_at: chrono::Utc::now(),
                            completed_at: chrono::Utc::now(),
                            duration: std::time::Duration::from_secs(0),
                            stdout: String::new(),
                            stderr: format!("Task execution failed: {}", e),
                            exit_code: Some(1),
                            retry_count: 0,
                            output_data: None,
                            error_message: Some(e.to_string()),
                            metadata: std::collections::HashMap::new(),
                        };
                        task_results.insert(task_name.clone(), failed_result);
                    }
                }
            }
        }
        
        let completed_at = chrono::Utc::now();
        let total_duration = start_time.elapsed();
        
        // Determine overall execution status
        let status = if failed_tasks.is_empty() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Failed
        };
        
        // Create execution metrics
        let metrics = ExecutionMetrics {
            total_tasks,
            successful_tasks,
            failed_tasks: failed_tasks.len(),
            skipped_tasks: 0,
            peak_concurrent_tasks: 1, // For now, single-threaded execution
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
        
        println!("📊 Execution completed: {}/{} tasks successful", successful_tasks, total_tasks);
        if !failed_tasks.is_empty() {
            println!("❌ Failed tasks: {:?}", failed_tasks);
        }
        
        Ok(ExecutionResult {
            dag_id: dag.name.clone(),
            execution_id,
            status,
            started_at,
            completed_at: Some(completed_at),
            total_duration: Some(total_duration),
            task_results,
            metrics,
        })
    }
    
    /// Execute a single task
    pub async fn execute_task(&self, task: &Task, context: &ExecutionContext) -> Result<TaskResult> {
        // Acquire semaphore permit for concurrency control
        let _permit = self.execution_semaphore.acquire().await?;
        
        // Get appropriate executor for task type
        let executor = self.task_registry.get_executor(&task.task_type)?;
        
        // Execute the task
        executor.execute(task, context).await.map_err(|e| e.into())
    }
    
    /// Get current execution status
    pub fn get_execution_status(&self, _execution_id: &str) -> Option<ExecutionStatus> {
        // TODO: Implement status tracking
        None
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
    
    /// Convert parser task to executor task
    fn convert_parser_task_to_executor_task(&self, parser_task: &crate::parser::Task, task_name: &str) -> crate::executor::traits::Task {
        use crate::executor::traits::{Task, TaskConfig};
        use std::collections::HashMap;
        
        // Determine task type
        let task_type = parser_task.config.task_type.clone().unwrap_or_else(|| "shell".to_string());
        
        
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
                } else if let (Some(module), Some(function)) = (&parser_task.config.module, &parser_task.config.function) {
                    TaskConfig::PythonFunction {
                        module: module.clone(),
                        function: function.clone(),
                        args: parser_task.config.args.clone().unwrap_or_default(),
                        kwargs: parser_task.config.kwargs.clone().unwrap_or_default(),
                        python_path: None,
                    }
                } else {
                    // Default to shell if no valid Python config
                    TaskConfig::Shell {
                        command: "echo 'No valid Python configuration'".to_string(),
                        working_dir: None,
                        shell: None,
                    }
                }
            },
            "http" => TaskConfig::HttpRequest {
                method: parser_task.config.method.clone().unwrap_or_else(|| "GET".to_string()),
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
                let operation = if parser_task.config.source.is_some() && parser_task.config.destination.is_some() {
                    "copy".to_string()
                } else if parser_task.config.source.is_some() {
                    "delete".to_string()
                } else {
                    "create".to_string()
                };
                
                TaskConfig::FileOperation {
                    operation,
                    source: parser_task.config.source.clone(),
                    destination: parser_task.config.destination.clone(),
                    options,
                }
            },
            _ => TaskConfig::Shell {
                command: format!("echo 'Unsupported task type: {}'", task_type),
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
        registry.register_executor("http".to_string(), Box::new(HttpTaskExecutor::new().expect("Failed to create HTTP executor")));
        
        // Register file executor
        registry.register_executor("file".to_string(), Box::new(FileTaskExecutor::new()));
        
        registry
    }
    
    /// Register a task executor for a specific task type
    pub fn register_executor(&mut self, task_type: String, executor: Box<dyn TaskExecutor>) {
        println!("📝 Registering executor for task type: {}", task_type);
        self.executors.insert(task_type, executor);
    }
    
    /// Get executor for a task type
    pub fn get_executor(&self, task_type: &str) -> Result<&dyn TaskExecutor> {
        self.executors
            .get(task_type)
            .map(|e| e.as_ref())
            .ok_or_else(|| ExecutorError::UnsupportedTaskType {
                task_type: task_type.to_string(),
            }.into())
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
            }.into());
        }
        
        Ok(())
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}