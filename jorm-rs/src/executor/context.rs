//! Execution context for task execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;
use crate::executor::Task;

/// Context information for task execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Unique execution ID
    pub execution_id: String,
    
    /// DAG name being executed
    pub dag_name: String,
    
    /// Task being executed
    pub task_id: String,
    
    /// Start time of execution
    pub start_time: Instant,
    
    /// Timestamp when execution started
    pub started_at: DateTime<Utc>,
    
    /// Environment variables for this execution
    pub environment: HashMap<String, String>,
    
    /// Working directory for this execution
    pub working_directory: Option<std::path::PathBuf>,
    
    /// Metadata for this execution
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Results from previously completed tasks
    pub task_outputs: HashMap<String, TaskOutput>,
    
    /// Execution configuration
    pub config: crate::executor::ExecutorConfig,
}

/// Output from a completed task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    /// Task ID that produced this output
    pub task_id: String,
    
    /// Standard output
    pub stdout: String,
    
    /// Standard error
    pub stderr: String,
    
    /// Exit code (if applicable)
    pub exit_code: Option<i32>,
    
    /// Structured data output (if any)
    pub data: Option<serde_json::Value>,
    
    /// Execution duration
    pub duration: std::time::Duration,
    
    /// When the task completed
    pub completed_at: DateTime<Utc>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(
        dag_name: String,
        task_id: String,
        config: crate::executor::ExecutorConfig,
    ) -> Self {
        let execution_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let mut environment = HashMap::new();
        
        // Add config environment variables
        for (key, value) in &config.environment_variables {
            environment.insert(key.clone(), value.clone());
        }
        
        // Inherit system environment if configured
        if config.inherit_environment {
            for (key, value) in std::env::vars() {
                environment.entry(key).or_insert(value);
            }
        }
        
        Self {
            execution_id,
            dag_name,
            task_id,
            start_time: Instant::now(),
            started_at: now,
            environment,
            working_directory: config.working_directory.clone(),
            metadata: HashMap::new(),
            task_outputs: HashMap::new(),
            config,
        }
    }
    
    /// Create a new execution context for a DAG execution
    pub fn for_dag_execution(
        dag_name: String,
        config: crate::executor::ExecutorConfig,
    ) -> Self {
        Self::new(dag_name, "dag_execution".to_string(), config)
    }
    
    /// Create a new execution context for a specific task
    pub fn for_task_execution(
        dag_name: String,
        task_id: String,
        config: crate::executor::ExecutorConfig,
        task: &Task,
    ) -> Self {
        let mut context = Self::new(dag_name, task_id, config);
        
        // Add task-specific environment variables
        for (key, value) in &task.environment {
            context.set_env_var(key.clone(), value.clone());
        }
        
        // Add task metadata
        for (key, value) in &task.metadata {
            context.add_metadata(key.clone(), value.clone());
        }
        
        context
    }
    
    /// Add metadata to the execution context
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }
    
    /// Get metadata from the execution context
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }
    
    /// Add task output to the context
    pub fn add_task_output(&mut self, output: TaskOutput) {
        self.task_outputs.insert(output.task_id.clone(), output);
    }
    
    /// Get output from a previously completed task
    pub fn get_task_output(&self, task_id: &str) -> Option<&TaskOutput> {
        self.task_outputs.get(task_id)
    }
    
    /// Set environment variable
    pub fn set_env_var(&mut self, key: String, value: String) {
        self.environment.insert(key, value);
    }
    
    /// Get environment variable
    pub fn get_env_var(&self, key: &str) -> Option<&String> {
        self.environment.get(key)
    }
    
    /// Get all environment variables
    pub fn get_all_env_vars(&self) -> &HashMap<String, String> {
        &self.environment
    }
    
    /// Get elapsed time since execution started
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
    
    /// Create a child context for a subtask
    pub fn create_child_context(&self, task_id: String) -> Self {
        let mut child = self.clone();
        child.task_id = task_id;
        child.start_time = Instant::now();
        child.started_at = Utc::now();
        child
    }
    
    /// Check if a task output exists
    pub fn has_task_output(&self, task_id: &str) -> bool {
        self.task_outputs.contains_key(task_id)
    }
    
    /// Get all task output IDs
    pub fn task_output_ids(&self) -> Vec<&String> {
        self.task_outputs.keys().collect()
    }
    
    /// Clear all task outputs
    pub fn clear_task_outputs(&mut self) {
        self.task_outputs.clear();
    }
    
    /// Get the effective working directory (task-specific or global)
    pub fn effective_working_directory(&self) -> Option<&std::path::PathBuf> {
        self.working_directory.as_ref()
    }
    
    /// Update the working directory
    pub fn set_working_directory(&mut self, dir: std::path::PathBuf) {
        self.working_directory = Some(dir);
    }
    
    /// Get environment variables as a Vec for process execution
    pub fn env_vars_for_process(&self) -> Vec<(String, String)> {
        self.environment.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
    
    /// Merge environment variables from another source
    pub fn merge_env_vars(&mut self, env_vars: HashMap<String, String>) {
        for (key, value) in env_vars {
            self.environment.insert(key, value);
        }
    }
    
    /// Get execution summary
    pub fn execution_summary(&self) -> ExecutionSummary {
        ExecutionSummary {
            execution_id: self.execution_id.clone(),
            dag_name: self.dag_name.clone(),
            current_task: self.task_id.clone(),
            started_at: self.started_at,
            elapsed: self.elapsed(),
            completed_tasks: self.task_outputs.len(),
            environment_vars: self.environment.len(),
            metadata_entries: self.metadata.len(),
        }
    }
}

/// Summary of execution context state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub execution_id: String,
    pub dag_name: String,
    pub current_task: String,
    pub started_at: DateTime<Utc>,
    pub elapsed: Duration,
    pub completed_tasks: usize,
    pub environment_vars: usize,
    pub metadata_entries: usize,
}