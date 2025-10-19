//! Execution recovery mechanisms for resuming failed or interrupted DAG executions
//!
//! This module provides functionality to recover from execution failures,
//! resume interrupted executions, and handle graceful shutdowns.

use crate::executor::{
    ExecutionResult, ExecutionStatus, TaskResult, TaskStatus,
    ExecutorError, NativeExecutor, ExecutionGraph, TaskState
};
use crate::executor::state_manager::{StateManager as PersistentStateManager, RecoveryCheckpoint};
use crate::parser::Dag;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Recovery manager for handling execution recovery
pub struct RecoveryManager {
    state_manager: Arc<PersistentStateManager>,
    executor: Arc<NativeExecutor>,
    recovery_config: RecoveryConfig,
}

/// Configuration for recovery behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// Maximum number of recovery attempts
    pub max_recovery_attempts: u32,
    /// Whether to retry failed tasks during recovery
    pub retry_failed_tasks: bool,
    /// Whether to skip failed tasks and continue with dependents
    pub skip_failed_tasks: bool,
    /// Checkpoint interval in seconds
    pub checkpoint_interval_seconds: u64,
    /// Whether to enable automatic checkpointing
    pub enable_auto_checkpointing: bool,
    /// Maximum age of checkpoints to consider for recovery (in hours)
    pub max_checkpoint_age_hours: u64,
}

/// Recovery strategy for handling different failure scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Resume from the last checkpoint
    ResumeFromCheckpoint,
    /// Restart failed tasks only
    RestartFailedTasks,
    /// Restart entire execution
    RestartExecution,
    /// Skip failed tasks and continue
    SkipFailedTasks,
}

/// Recovery context containing execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryContext {
    pub execution_id: String,
    pub dag_content: String, // Serialized DAG
    pub completed_tasks: HashSet<String>,
    pub failed_tasks: HashSet<String>,
    pub running_tasks: HashSet<String>,
    pub task_results: HashMap<String, String>, // Serialized TaskResult
    pub recovery_attempt: u32,
    pub original_start_time: DateTime<Utc>,
    pub recovery_start_time: DateTime<Utc>,
    pub execution_context: String, // Serialized ExecutionContext
}

/// Recovery operation result
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    pub success: bool,
    pub execution_result: Option<ExecutionResult>,
    pub recovery_strategy_used: RecoveryStrategy,
    pub tasks_recovered: usize,
    pub tasks_skipped: usize,
    pub recovery_duration: std::time::Duration,
    pub error_message: Option<String>,
}

/// Graceful shutdown manager
pub struct GracefulShutdownManager {
    state_manager: Arc<PersistentStateManager>,
    active_executions: Arc<RwLock<HashMap<String, RecoveryContext>>>,
    shutdown_requested: Arc<RwLock<bool>>,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_recovery_attempts: 3,
            retry_failed_tasks: true,
            skip_failed_tasks: false,
            checkpoint_interval_seconds: 30,
            enable_auto_checkpointing: true,
            max_checkpoint_age_hours: 24,
        }
    }
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(
        state_manager: Arc<PersistentStateManager>,
        executor: Arc<NativeExecutor>,
        config: RecoveryConfig,
    ) -> Self {
        Self {
            state_manager,
            executor,
            recovery_config: config,
        }
    }

    /// Attempt to recover an execution from checkpoint
    pub async fn recover_execution(
        &self,
        execution_id: &str,
        strategy: RecoveryStrategy,
    ) -> Result<RecoveryResult> {
        let start_time = std::time::Instant::now();
        
        // Get checkpoint data
        let checkpoint = self.state_manager
            .get_checkpoint(execution_id)
            .await?
            .ok_or_else(|| ExecutorError::RecoveryError {
                message: format!("No checkpoint found for execution {}", execution_id),
                execution_id: execution_id.to_string(),
            })?;

        // Check if checkpoint is too old
        let checkpoint_age = Utc::now().signed_duration_since(checkpoint.checkpoint_time);
        if checkpoint_age.num_hours() > self.recovery_config.max_checkpoint_age_hours as i64 {
            return Ok(RecoveryResult {
                success: false,
                execution_result: None,
                recovery_strategy_used: strategy,
                tasks_recovered: 0,
                tasks_skipped: 0,
                recovery_duration: start_time.elapsed(),
                error_message: Some("Checkpoint too old for recovery".to_string()),
            });
        }

        // Deserialize recovery context
        let mut recovery_context = self.deserialize_recovery_context(&checkpoint)?;
        recovery_context.recovery_attempt += 1;
        recovery_context.recovery_start_time = Utc::now();

        // Check recovery attempt limit
        if recovery_context.recovery_attempt > self.recovery_config.max_recovery_attempts {
            return Ok(RecoveryResult {
                success: false,
                execution_result: None,
                recovery_strategy_used: strategy,
                tasks_recovered: 0,
                tasks_skipped: 0,
                recovery_duration: start_time.elapsed(),
                error_message: Some("Maximum recovery attempts exceeded".to_string()),
            });
        }

        // Execute recovery based on strategy
        let result = match strategy {
            RecoveryStrategy::ResumeFromCheckpoint => {
                self.resume_from_checkpoint(recovery_context).await
            }
            RecoveryStrategy::RestartFailedTasks => {
                self.restart_failed_tasks(recovery_context).await
            }
            RecoveryStrategy::RestartExecution => {
                self.restart_execution(recovery_context).await
            }
            RecoveryStrategy::SkipFailedTasks => {
                self.skip_failed_tasks(recovery_context).await
            }
        };

        match result {
            Ok(execution_result) => Ok(RecoveryResult {
                success: true,
                execution_result: Some(execution_result.clone()),
                recovery_strategy_used: strategy,
                tasks_recovered: execution_result.metrics.successful_tasks,
                tasks_skipped: execution_result.metrics.skipped_tasks,
                recovery_duration: start_time.elapsed(),
                error_message: None,
            }),
            Err(e) => Ok(RecoveryResult {
                success: false,
                execution_result: None,
                recovery_strategy_used: strategy,
                tasks_recovered: 0,
                tasks_skipped: 0,
                recovery_duration: start_time.elapsed(),
                error_message: Some(e.to_string()),
            }),
        }
    }

    /// Resume execution from checkpoint
    async fn resume_from_checkpoint(&self, context: RecoveryContext) -> Result<ExecutionResult> {
        // Deserialize the DAG
        let dag: Dag = serde_json::from_str(&context.dag_content)?;
        
        // For now, just restart the entire execution
        // In a full implementation, this would create a filtered DAG
        // with only the remaining tasks
        let result = self.executor.execute_dag(&dag).await?;
        
        // Update execution in state manager
        self.state_manager.update_execution(&result).await?;
        
        Ok(result)
    }

    /// Restart only failed tasks
    async fn restart_failed_tasks(&self, context: RecoveryContext) -> Result<ExecutionResult> {
        // Deserialize the DAG
        let dag: Dag = serde_json::from_str(&context.dag_content)?;
        
        // For now, just restart the entire execution
        // In a full implementation, this would create a filtered DAG
        // with only the failed tasks and their dependencies
        let result = self.executor.execute_dag(&dag).await?;
        
        // Update execution in state manager
        self.state_manager.update_execution(&result).await?;
        
        Ok(result)
    }

    /// Restart entire execution
    async fn restart_execution(&self, context: RecoveryContext) -> Result<ExecutionResult> {
        // Deserialize the DAG
        let dag: Dag = serde_json::from_str(&context.dag_content)?;
        
        // Simply re-execute the entire DAG
        let result = self.executor.execute_dag(&dag).await?;
        
        // Update execution in state manager
        self.state_manager.update_execution(&result).await?;
        
        Ok(result)
    }

    /// Skip failed tasks and continue with remaining
    async fn skip_failed_tasks(&self, context: RecoveryContext) -> Result<ExecutionResult> {
        // Deserialize the DAG
        let dag: Dag = serde_json::from_str(&context.dag_content)?;
        
        // For now, just restart the entire execution
        // In a full implementation, this would mark failed tasks as skipped
        let result = self.executor.execute_dag(&dag).await?;
        
        // Update execution in state manager
        self.state_manager.update_execution(&result).await?;
        
        Ok(result)
    }

    /// Create a checkpoint for the current execution state
    pub async fn create_checkpoint(
        &self,
        execution_id: &str,
        dag: &Dag,
        completed_tasks: Vec<String>,
        failed_tasks: Vec<String>,
        running_tasks: Vec<String>,
    ) -> Result<()> {
        let checkpoint = RecoveryCheckpoint {
            execution_id: execution_id.to_string(),
            dag_content: serde_json::to_string(dag)?,
            completed_tasks,
            failed_tasks,
            running_tasks,
            checkpoint_time: Utc::now(),
            execution_context: "{}".to_string(), // Simplified for now
        };

        self.state_manager.create_checkpoint(&checkpoint).await?;
        Ok(())
    }

    /// List available recovery points
    pub async fn list_recovery_points(&self, execution_id: &str) -> Result<Vec<RecoveryCheckpoint>> {
        if let Some(checkpoint) = self.state_manager.get_checkpoint(execution_id).await? {
            Ok(vec![checkpoint])
        } else {
            Ok(vec![])
        }
    }

    /// Validate if recovery is possible for an execution
    pub async fn can_recover(&self, execution_id: &str) -> Result<bool> {
        if let Some(checkpoint) = self.state_manager.get_checkpoint(execution_id).await? {
            let checkpoint_age = Utc::now().signed_duration_since(checkpoint.checkpoint_time);
            Ok(checkpoint_age.num_hours() <= self.recovery_config.max_checkpoint_age_hours as i64)
        } else {
            Ok(false)
        }
    }

    /// Helper method to deserialize recovery context from checkpoint
    fn deserialize_recovery_context(&self, checkpoint: &RecoveryCheckpoint) -> Result<RecoveryContext> {
        Ok(RecoveryContext {
            execution_id: checkpoint.execution_id.clone(),
            dag_content: checkpoint.dag_content.clone(),
            completed_tasks: checkpoint.completed_tasks.iter().cloned().collect(),
            failed_tasks: checkpoint.failed_tasks.iter().cloned().collect(),
            running_tasks: checkpoint.running_tasks.iter().cloned().collect(),
            task_results: HashMap::new(), // Simplified for now
            recovery_attempt: 0,
            original_start_time: checkpoint.checkpoint_time,
            recovery_start_time: Utc::now(),
            execution_context: checkpoint.execution_context.clone(),
        })
    }
}

impl GracefulShutdownManager {
    /// Create a new graceful shutdown manager
    pub fn new(state_manager: Arc<PersistentStateManager>) -> Self {
        Self {
            state_manager,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            shutdown_requested: Arc::new(RwLock::new(false)),
        }
    }

    /// Register an active execution
    pub async fn register_execution(&self, context: RecoveryContext) {
        let mut executions = self.active_executions.write().await;
        executions.insert(context.execution_id.clone(), context);
    }

    /// Unregister a completed execution
    pub async fn unregister_execution(&self, execution_id: &str) {
        let mut executions = self.active_executions.write().await;
        executions.remove(execution_id);
    }

    /// Request graceful shutdown
    pub async fn request_shutdown(&self) -> Result<()> {
        let mut shutdown_requested = self.shutdown_requested.write().await;
        *shutdown_requested = true;

        // Create checkpoints for all active executions
        let executions = self.active_executions.read().await;
        for (execution_id, context) in executions.iter() {
            let checkpoint = RecoveryCheckpoint {
                execution_id: execution_id.clone(),
                dag_content: context.dag_content.clone(),
                completed_tasks: context.completed_tasks.iter().cloned().collect(),
                failed_tasks: context.failed_tasks.iter().cloned().collect(),
                running_tasks: context.running_tasks.iter().cloned().collect(),
                checkpoint_time: Utc::now(),
                execution_context: context.execution_context.clone(),
            };

            self.state_manager.create_checkpoint(&checkpoint).await?;
        }

        Ok(())
    }

    /// Check if shutdown has been requested
    pub async fn is_shutdown_requested(&self) -> bool {
        *self.shutdown_requested.read().await
    }

    /// Get count of active executions
    pub async fn active_execution_count(&self) -> usize {
        self.active_executions.read().await.len()
    }

    /// Wait for all executions to complete or timeout
    pub async fn wait_for_completion(&self, timeout_seconds: u64) -> Result<bool> {
        let start_time = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_secs(timeout_seconds);

        while start_time.elapsed() < timeout_duration {
            if self.active_execution_count().await == 0 {
                return Ok(true);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Ok(false) // Timeout reached
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::state_manager::StateManager;

    #[tokio::test]
    async fn test_recovery_manager_creation() {
        let state_manager = Arc::new(PersistentStateManager::new_in_memory().await.unwrap());
        let executor = Arc::new(NativeExecutor::new(crate::executor::ExecutorConfig::default()));
        let config = RecoveryConfig::default();

        let recovery_manager = RecoveryManager::new(state_manager, executor, config);
        assert_eq!(recovery_manager.recovery_config.max_recovery_attempts, 3);
    }

    #[tokio::test]
    async fn test_graceful_shutdown_manager() {
        let state_manager = Arc::new(PersistentStateManager::new_in_memory().await.unwrap());
        let shutdown_manager = GracefulShutdownManager::new(state_manager);

        assert_eq!(shutdown_manager.active_execution_count().await, 0);
        assert!(!shutdown_manager.is_shutdown_requested().await);

        // Test shutdown request
        shutdown_manager.request_shutdown().await.unwrap();
        assert!(shutdown_manager.is_shutdown_requested().await);
    }

    #[tokio::test]
    async fn test_recovery_context_serialization() {
        let recovery_context = RecoveryContext {
            execution_id: "test-exec".to_string(),
            dag_content: "{}".to_string(),
            completed_tasks: HashSet::from(["task1".to_string()]),
            failed_tasks: HashSet::from(["task2".to_string()]),
            running_tasks: HashSet::new(),
            task_results: HashMap::new(),
            recovery_attempt: 1,
            original_start_time: Utc::now(),
            recovery_start_time: Utc::now(),
            execution_context: "{}".to_string(),
        };

        let serialized = serde_json::to_string(&recovery_context).unwrap();
        let deserialized: RecoveryContext = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.execution_id, "test-exec");
        assert_eq!(deserialized.recovery_attempt, 1);
    }
}