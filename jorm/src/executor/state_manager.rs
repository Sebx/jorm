//! SQLite-based state manager for execution persistence and recovery
//!
//! This module provides persistent state management for DAG executions,
//! allowing for recovery from failures and tracking execution history.

use crate::executor::{
    ExecutionResult, ExecutionStatus, TaskResult, TaskStatus,
    ExecutorError, ExecutionContext
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Row};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

/// SQLite-based state manager for execution persistence
pub struct StateManager {
    pool: SqlitePool,
}

/// Execution state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedExecution {
    pub execution_id: String,
    pub dag_id: String,
    pub status: ExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_duration_ms: Option<u64>,
    pub task_count: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub skipped_tasks: usize,
    pub peak_concurrent_tasks: usize,
    pub metadata: String, // JSON serialized metadata
}

/// Task execution state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedTask {
    pub task_id: String,
    pub execution_id: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub retry_count: u32,
    pub error_message: Option<String>,
    pub metadata: String, // JSON serialized metadata
}

/// Recovery checkpoint for resuming executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryCheckpoint {
    pub execution_id: String,
    pub dag_content: String, // Serialized DAG
    pub completed_tasks: Vec<String>,
    pub failed_tasks: Vec<String>,
    pub running_tasks: Vec<String>,
    pub checkpoint_time: DateTime<Utc>,
    pub execution_context: String, // Serialized ExecutionContext
}

/// Query filters for execution history
#[derive(Debug, Clone, Default)]
pub struct ExecutionQuery {
    pub dag_id: Option<String>,
    pub status: Option<ExecutionStatus>,
    pub start_time_after: Option<DateTime<Utc>>,
    pub start_time_before: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl StateManager {
    /// Create a new state manager with SQLite database
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let database_url = format!("sqlite:{}", db_path.as_ref().display());
        let pool = SqlitePool::connect(&database_url).await?;
        
        let state_manager = Self { pool };
        state_manager.initialize_schema().await?;
        
        Ok(state_manager)
    }

    /// Create an in-memory state manager for testing
    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        
        let state_manager = Self { pool };
        state_manager.initialize_schema().await?;
        
        Ok(state_manager)
    }

    /// Initialize database schema
    async fn initialize_schema(&self) -> Result<()> {
        // Create executions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS executions (
                execution_id TEXT PRIMARY KEY,
                dag_id TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                total_duration_ms INTEGER,
                task_count INTEGER NOT NULL,
                successful_tasks INTEGER NOT NULL DEFAULT 0,
                failed_tasks INTEGER NOT NULL DEFAULT 0,
                skipped_tasks INTEGER NOT NULL DEFAULT 0,
                peak_concurrent_tasks INTEGER NOT NULL DEFAULT 1,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create tasks table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id TEXT NOT NULL,
                execution_id TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                stdout TEXT NOT NULL DEFAULT '',
                stderr TEXT NOT NULL DEFAULT '',
                exit_code INTEGER,
                retry_count INTEGER NOT NULL DEFAULT 0,
                error_message TEXT,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (execution_id) REFERENCES executions (execution_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create recovery checkpoints table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS recovery_checkpoints (
                execution_id TEXT PRIMARY KEY,
                dag_content TEXT NOT NULL,
                completed_tasks TEXT NOT NULL, -- JSON array
                failed_tasks TEXT NOT NULL,    -- JSON array
                running_tasks TEXT NOT NULL,   -- JSON array
                checkpoint_time TEXT NOT NULL,
                execution_context TEXT NOT NULL, -- JSON serialized
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (execution_id) REFERENCES executions (execution_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for better query performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_executions_dag_id ON executions (dag_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_executions_status ON executions (status)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_executions_started_at ON executions (started_at)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_execution_id ON tasks (execution_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks (status)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Start tracking a new execution
    pub async fn start_execution(
        &self,
        execution_id: &str,
        dag_id: &str,
        task_count: usize,
    ) -> Result<()> {
        let started_at = Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            INSERT INTO executions (
                execution_id, dag_id, status, started_at, task_count
            ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(execution_id)
        .bind(dag_id)
        .bind("Running")
        .bind(started_at)
        .bind(task_count as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update execution status and metrics
    pub async fn update_execution(&self, result: &ExecutionResult) -> Result<()> {
        let completed_at = result.completed_at.map(|dt| dt.to_rfc3339());
        let total_duration_ms = result.total_duration.map(|d| d.as_millis() as u64);
        let status_str = match result.status {
            ExecutionStatus::Success => "Success",
            ExecutionStatus::Failed => "Failed",
            ExecutionStatus::Running => "Running",
            ExecutionStatus::Pending => "Pending",
            ExecutionStatus::Cancelled => "Cancelled",
            ExecutionStatus::Timeout => "Timeout",
            ExecutionStatus::PartialSuccess => "PartialSuccess",
        };

        sqlx::query(
            r#"
            UPDATE executions SET
                status = ?,
                completed_at = ?,
                total_duration_ms = ?,
                successful_tasks = ?,
                failed_tasks = ?,
                skipped_tasks = ?,
                peak_concurrent_tasks = ?
            WHERE execution_id = ?
            "#,
        )
        .bind(status_str)
        .bind(completed_at)
        .bind(total_duration_ms.map(|ms| ms as i64))
        .bind(result.metrics.successful_tasks as i64)
        .bind(result.metrics.failed_tasks as i64)
        .bind(result.metrics.skipped_tasks as i64)
        .bind(result.metrics.peak_concurrent_tasks as i64)
        .bind(&result.execution_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record task execution result
    pub async fn record_task_result(
        &self,
        execution_id: &str,
        task_result: &TaskResult,
    ) -> Result<()> {
        let status_str = match task_result.status {
            TaskStatus::Success => "Success",
            TaskStatus::Failed => "Failed",
            TaskStatus::Running => "Running",
            TaskStatus::Pending => "Pending",
            TaskStatus::Cancelled => "Cancelled",
            TaskStatus::Timeout => "Timeout",
            TaskStatus::Skipped => "Skipped",
            TaskStatus::Retrying => "Retrying",
        };

        let metadata = serde_json::to_string(&task_result.metadata)
            .unwrap_or_else(|_| "{}".to_string());

        sqlx::query(
            r#"
            INSERT INTO tasks (
                task_id, execution_id, status, started_at, completed_at,
                duration_ms, stdout, stderr, exit_code, retry_count,
                error_message, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task_result.task_id)
        .bind(execution_id)
        .bind(status_str)
        .bind(task_result.started_at.to_rfc3339())
        .bind(task_result.completed_at.to_rfc3339())
        .bind(task_result.duration.as_millis() as i64)
        .bind(&task_result.stdout)
        .bind(&task_result.stderr)
        .bind(task_result.exit_code)
        .bind(task_result.retry_count as i64)
        .bind(&task_result.error_message)
        .bind(metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create a recovery checkpoint
    pub async fn create_checkpoint(
        &self,
        checkpoint: &RecoveryCheckpoint,
    ) -> Result<()> {
        let completed_tasks_json = serde_json::to_string(&checkpoint.completed_tasks)?;
        let failed_tasks_json = serde_json::to_string(&checkpoint.failed_tasks)?;
        let running_tasks_json = serde_json::to_string(&checkpoint.running_tasks)?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO recovery_checkpoints (
                execution_id, dag_content, completed_tasks, failed_tasks,
                running_tasks, checkpoint_time, execution_context
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&checkpoint.execution_id)
        .bind(&checkpoint.dag_content)
        .bind(completed_tasks_json)
        .bind(failed_tasks_json)
        .bind(running_tasks_json)
        .bind(checkpoint.checkpoint_time.to_rfc3339())
        .bind(&checkpoint.execution_context)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get recovery checkpoint for an execution
    pub async fn get_checkpoint(
        &self,
        execution_id: &str,
    ) -> Result<Option<RecoveryCheckpoint>> {
        let row = sqlx::query(
            r#"
            SELECT execution_id, dag_content, completed_tasks, failed_tasks,
                   running_tasks, checkpoint_time, execution_context
            FROM recovery_checkpoints
            WHERE execution_id = ?
            "#,
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let completed_tasks: Vec<String> = serde_json::from_str(row.get("completed_tasks"))?;
            let failed_tasks: Vec<String> = serde_json::from_str(row.get("failed_tasks"))?;
            let running_tasks: Vec<String> = serde_json::from_str(row.get("running_tasks"))?;
            let checkpoint_time: String = row.get("checkpoint_time");

            Ok(Some(RecoveryCheckpoint {
                execution_id: row.get("execution_id"),
                dag_content: row.get("dag_content"),
                completed_tasks,
                failed_tasks,
                running_tasks,
                checkpoint_time: DateTime::parse_from_rfc3339(&checkpoint_time)?.with_timezone(&Utc),
                execution_context: row.get("execution_context"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get execution by ID
    pub async fn get_execution(&self, execution_id: &str) -> Result<Option<PersistedExecution>> {
        let row = sqlx::query(
            r#"
            SELECT execution_id, dag_id, status, started_at, completed_at,
                   total_duration_ms, task_count, successful_tasks, failed_tasks,
                   skipped_tasks, peak_concurrent_tasks, metadata
            FROM executions
            WHERE execution_id = ?
            "#,
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let started_at: String = row.get("started_at");
            let completed_at: Option<String> = row.get("completed_at");
            let status_str: String = row.get("status");

            let status = match status_str.as_str() {
                "Success" => ExecutionStatus::Success,
                "Failed" => ExecutionStatus::Failed,
                "Running" => ExecutionStatus::Running,
                _ => ExecutionStatus::Failed,
            };

            Ok(Some(PersistedExecution {
                execution_id: row.get("execution_id"),
                dag_id: row.get("dag_id"),
                status,
                started_at: DateTime::parse_from_rfc3339(&started_at)?.with_timezone(&Utc),
                completed_at: completed_at
                    .map(|dt| DateTime::parse_from_rfc3339(&dt))
                    .transpose()?
                    .map(|dt| dt.with_timezone(&Utc)),
                total_duration_ms: row.get::<Option<i64>, _>("total_duration_ms").map(|ms| ms as u64),
                task_count: row.get::<i64, _>("task_count") as usize,
                successful_tasks: row.get::<i64, _>("successful_tasks") as usize,
                failed_tasks: row.get::<i64, _>("failed_tasks") as usize,
                skipped_tasks: row.get::<i64, _>("skipped_tasks") as usize,
                peak_concurrent_tasks: row.get::<i64, _>("peak_concurrent_tasks") as usize,
                metadata: row.get("metadata"),
            }))
        } else {
            Ok(None)
        }
    }
}