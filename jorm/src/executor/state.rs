//! State management and persistence for DAG execution
//!
//! This module provides SQLite-based state persistence for tracking execution
//! state, task history, and recovery capabilities.

use crate::executor::{ExecutionResult, ExecutionStatus, TaskResult, TaskStatus};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

/// Configuration for state management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    /// Database file path
    pub database_path: String,
    /// Maximum number of execution records to keep
    pub max_executions: usize,
    /// How long to keep completed executions
    pub retention_period: Duration,
    /// Enable automatic cleanup
    pub auto_cleanup: bool,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            database_path: "jorm_state.db".to_string(),
            max_executions: 1000,
            retention_period: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            auto_cleanup: true,
        }
    }
}

/// Execution state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub execution_id: String,
    pub dag_id: String,
    pub status: ExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_duration: Option<Duration>,
    pub task_states: HashMap<String, TaskState>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Task state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub task_id: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration: Option<Duration>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub retry_count: u32,
    pub error_message: Option<String>,
    pub output_data: Option<serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Task execution history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: i64,
    pub execution_id: String,
    pub dag_id: String,
    pub task_id: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub retry_count: u32,
    pub error_message: Option<String>,
    pub output_data: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
}

/// State manager for execution persistence
pub struct StateManager {
    pool: SqlitePool,
    config: StateConfig,
}

impl StateManager {
    /// Create a new state manager with the given configuration
    pub async fn new(config: StateConfig) -> Result<Self> {
        // Ensure database directory exists
        if let Some(parent) = Path::new(&config.database_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Create connection pool
        let database_url = format!("sqlite:{}", config.database_path);
        let pool = SqlitePool::connect(&database_url).await?;

        let manager = Self { pool, config };

        // Initialize database schema
        manager.initialize_schema().await?;

        // Perform cleanup if enabled
        if manager.config.auto_cleanup {
            manager.cleanup_old_executions().await?;
        }

        Ok(manager)
    }

    /// Initialize the database schema
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
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create task_executions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS task_executions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                execution_id TEXT NOT NULL,
                dag_id TEXT NOT NULL,
                task_id TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                duration_ms INTEGER,
                stdout TEXT NOT NULL DEFAULT '',
                stderr TEXT NOT NULL DEFAULT '',
                exit_code INTEGER,
                retry_count INTEGER NOT NULL DEFAULT 0,
                error_message TEXT,
                output_data TEXT,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (execution_id) REFERENCES executions (execution_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for better performance
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_executions_dag_id ON executions (dag_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_executions_status ON executions (status)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_task_executions_execution_id ON task_executions (execution_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_task_executions_task_id ON task_executions (task_id)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Save execution state to database
    pub async fn save_execution_state(&self, state: &ExecutionState) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Insert or update execution record
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO executions 
            (execution_id, dag_id, status, started_at, completed_at, total_duration_ms, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&state.execution_id)
        .bind(&state.dag_id)
        .bind(serde_json::to_string(&state.status)?)
        .bind(state.started_at.to_rfc3339())
        .bind(state.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(state.total_duration.map(|d| d.as_millis() as i64))
        .bind(serde_json::to_string(&state.metadata)?)
        .execute(&mut *tx)
        .await?;

        // Insert or update task states
        for (task_id, task_state) in &state.task_states {
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO task_executions 
                (execution_id, dag_id, task_id, status, started_at, completed_at, duration_ms, 
                 stdout, stderr, exit_code, retry_count, error_message, output_data, metadata)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&state.execution_id)
            .bind(&state.dag_id)
            .bind(task_id)
            .bind(serde_json::to_string(&task_state.status)?)
            .bind(task_state.started_at.to_rfc3339())
            .bind(task_state.completed_at.map(|dt| dt.to_rfc3339()))
            .bind(task_state.duration.map(|d| d.as_millis() as i64))
            .bind(&task_state.stdout)
            .bind(&task_state.stderr)
            .bind(task_state.exit_code)
            .bind(task_state.retry_count as i32)
            .bind(&task_state.error_message)
            .bind(task_state.output_data.as_ref().map(|d| serde_json::to_string(d)).transpose()?)
            .bind(serde_json::to_string(&task_state.metadata)?)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Load execution state from database
    pub async fn load_execution_state(&self, execution_id: &str) -> Result<Option<ExecutionState>> {
        // Load execution record
        let execution_row = sqlx::query(
            "SELECT execution_id, dag_id, status, started_at, completed_at, total_duration_ms, metadata FROM executions WHERE execution_id = ?"
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await?;

        let execution_row = match execution_row {
            Some(row) => row,
            None => return Ok(None),
        };

        // Parse execution data
        let status: ExecutionStatus = serde_json::from_str(&execution_row.get::<String, _>("status"))?;
        let started_at = DateTime::parse_from_rfc3339(&execution_row.get::<String, _>("started_at"))?
            .with_timezone(&Utc);
        let completed_at = execution_row
            .get::<Option<String>, _>("completed_at")
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()?;
        let total_duration = execution_row
            .get::<Option<i64>, _>("total_duration_ms")
            .map(|ms| Duration::from_millis(ms as u64));
        let metadata: HashMap<String, serde_json::Value> = 
            serde_json::from_str(&execution_row.get::<String, _>("metadata"))?;

        // Load task states
        let task_rows = sqlx::query(
            r#"
            SELECT task_id, status, started_at, completed_at, duration_ms, stdout, stderr, 
                   exit_code, retry_count, error_message, output_data, metadata
            FROM task_executions 
            WHERE execution_id = ?
            "#
        )
        .bind(execution_id)
        .fetch_all(&self.pool)
        .await?;

        let mut task_states = HashMap::new();
        for row in task_rows {
            let task_id = row.get::<String, _>("task_id");
            let status: TaskStatus = serde_json::from_str(&row.get::<String, _>("status"))?;
            let started_at = DateTime::parse_from_rfc3339(&row.get::<String, _>("started_at"))?
                .with_timezone(&Utc);
            let completed_at = row
                .get::<Option<String>, _>("completed_at")
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                .transpose()?;
            let duration = row
                .get::<Option<i64>, _>("duration_ms")
                .map(|ms| Duration::from_millis(ms as u64));
            let output_data = row
                .get::<Option<String>, _>("output_data")
                .map(|s| serde_json::from_str(&s))
                .transpose()?;
            let task_metadata: HashMap<String, serde_json::Value> = 
                serde_json::from_str(&row.get::<String, _>("metadata"))?;

            let task_state = TaskState {
                task_id: task_id.clone(),
                status,
                started_at,
                completed_at,
                duration,
                stdout: row.get("stdout"),
                stderr: row.get("stderr"),
                exit_code: row.get("exit_code"),
                retry_count: row.get::<i32, _>("retry_count") as u32,
                error_message: row.get("error_message"),
                output_data,
                metadata: task_metadata,
            };

            task_states.insert(task_id, task_state);
        }

        Ok(Some(ExecutionState {
            execution_id: execution_row.get("execution_id"),
            dag_id: execution_row.get("dag_id"),
            status,
            started_at,
            completed_at,
            total_duration,
            task_states,
            metadata,
        }))
    }

    /// Get task execution history for a specific DAG and task
    pub async fn get_task_history(&self, dag_id: &str, task_id: &str) -> Result<Vec<TaskExecution>> {
        let rows = sqlx::query(
            r#"
            SELECT id, execution_id, dag_id, task_id, status, started_at, completed_at, 
                   duration_ms, stdout, stderr, exit_code, retry_count, error_message, 
                   output_data, metadata
            FROM task_executions 
            WHERE dag_id = ? AND task_id = ?
            ORDER BY started_at DESC
            LIMIT 100
            "#
        )
        .bind(dag_id)
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;

        let mut history = Vec::new();
        for row in rows {
            let status: TaskStatus = serde_json::from_str(&row.get::<String, _>("status"))?;
            let started_at = DateTime::parse_from_rfc3339(&row.get::<String, _>("started_at"))?
                .with_timezone(&Utc);
            let completed_at = row
                .get::<Option<String>, _>("completed_at")
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                .transpose()?;
            let output_data = row
                .get::<Option<String>, _>("output_data")
                .map(|s| serde_json::from_str(&s))
                .transpose()?;
            let metadata: serde_json::Value = 
                serde_json::from_str(&row.get::<String, _>("metadata"))?;

            history.push(TaskExecution {
                id: row.get("id"),
                execution_id: row.get("execution_id"),
                dag_id: row.get("dag_id"),
                task_id: row.get("task_id"),
                status,
                started_at,
                completed_at,
                duration_ms: row.get("duration_ms"),
                stdout: row.get("stdout"),
                stderr: row.get("stderr"),
                exit_code: row.get("exit_code"),
                retry_count: row.get::<i32, _>("retry_count") as u32,
                error_message: row.get("error_message"),
                output_data,
                metadata,
            });
        }

        Ok(history)
    }

    /// List recent executions for a DAG
    pub async fn list_executions(&self, dag_id: Option<&str>, limit: usize) -> Result<Vec<ExecutionState>> {
        let query = if let Some(dag_id) = dag_id {
            sqlx::query(
                r#"
                SELECT execution_id, dag_id, status, started_at, completed_at, total_duration_ms, metadata
                FROM executions 
                WHERE dag_id = ?
                ORDER BY started_at DESC
                LIMIT ?
                "#
            )
            .bind(dag_id)
            .bind(limit as i32)
        } else {
            sqlx::query(
                r#"
                SELECT execution_id, dag_id, status, started_at, completed_at, total_duration_ms, metadata
                FROM executions 
                ORDER BY started_at DESC
                LIMIT ?
                "#
            )
            .bind(limit as i32)
        };

        let rows = query.fetch_all(&self.pool).await?;

        let mut executions = Vec::new();
        for row in rows {
            let execution_id: String = row.get("execution_id");
            
            // Load full execution state (this could be optimized for listing)
            if let Some(state) = self.load_execution_state(&execution_id).await? {
                executions.push(state);
            }
        }

        Ok(executions)
    }

    /// Delete execution state and history
    pub async fn delete_execution(&self, execution_id: &str) -> Result<bool> {
        let mut tx = self.pool.begin().await?;

        // Delete task executions first (foreign key constraint)
        sqlx::query("DELETE FROM task_executions WHERE execution_id = ?")
            .bind(execution_id)
            .execute(&mut *tx)
            .await?;

        // Delete execution record
        let result = sqlx::query("DELETE FROM executions WHERE execution_id = ?")
            .bind(execution_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(result.rows_affected() > 0)
    }

    /// Clean up old executions based on retention policy
    pub async fn cleanup_old_executions(&self) -> Result<usize> {
        let cutoff_date = Utc::now() - chrono::Duration::from_std(self.config.retention_period)?;
        
        let mut tx = self.pool.begin().await?;

        // Get execution IDs to delete
        let old_executions: Vec<String> = sqlx::query_scalar(
            "SELECT execution_id FROM executions WHERE started_at < ? AND status IN ('Success', 'Failed')"
        )
        .bind(cutoff_date.to_rfc3339())
        .fetch_all(&mut *tx)
        .await?;

        let mut deleted_count = 0;

        // Delete old executions
        for execution_id in &old_executions {
            sqlx::query("DELETE FROM task_executions WHERE execution_id = ?")
                .bind(execution_id)
                .execute(&mut *tx)
                .await?;

            let result = sqlx::query("DELETE FROM executions WHERE execution_id = ?")
                .bind(execution_id)
                .execute(&mut *tx)
                .await?;

            if result.rows_affected() > 0 {
                deleted_count += 1;
            }
        }

        // Also enforce max_executions limit
        let excess_executions: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT execution_id FROM executions 
            ORDER BY started_at DESC 
            LIMIT -1 OFFSET ?
            "#
        )
        .bind(self.config.max_executions as i32)
        .fetch_all(&mut *tx)
        .await?;

        for execution_id in &excess_executions {
            sqlx::query("DELETE FROM task_executions WHERE execution_id = ?")
                .bind(execution_id)
                .execute(&mut *tx)
                .await?;

            let result = sqlx::query("DELETE FROM executions WHERE execution_id = ?")
                .bind(execution_id)
                .execute(&mut *tx)
                .await?;

            if result.rows_affected() > 0 {
                deleted_count += 1;
            }
        }

        tx.commit().await?;

        Ok(deleted_count)
    }

    /// Get execution statistics
    pub async fn get_execution_stats(&self) -> Result<ExecutionStats> {
        let total_executions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM executions")
            .fetch_one(&self.pool)
            .await?;

        let successful_executions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM executions WHERE status = ?"
        )
        .bind(serde_json::to_string(&ExecutionStatus::Success)?)
        .fetch_one(&self.pool)
        .await?;

        let failed_executions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM executions WHERE status = ?"
        )
        .bind(serde_json::to_string(&ExecutionStatus::Failed)?)
        .fetch_one(&self.pool)
        .await?;

        let running_executions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM executions WHERE status = ?"
        )
        .bind(serde_json::to_string(&ExecutionStatus::Running)?)
        .fetch_one(&self.pool)
        .await?;

        Ok(ExecutionStats {
            total_executions: total_executions as usize,
            successful_executions: successful_executions as usize,
            failed_executions: failed_executions as usize,
            running_executions: running_executions as usize,
        })
    }

    /// Close the state manager and connection pool
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub running_executions: usize,
}

/// Convert ExecutionResult to ExecutionState for persistence
impl From<&ExecutionResult> for ExecutionState {
    fn from(result: &ExecutionResult) -> Self {
        let mut task_states = HashMap::new();
        
        for (task_id, task_result) in &result.task_results {
            task_states.insert(task_id.clone(), TaskState {
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

        Self {
            execution_id: result.execution_id.clone(),
            dag_id: result.dag_id.clone(),
            status: result.status.clone(),
            started_at: result.started_at,
            completed_at: result.completed_at,
            total_duration: result.total_duration,
            task_states,
            metadata: HashMap::new(), // Could be extended to include execution metadata
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_state_manager_creation() {
        let config = StateConfig {
            database_path: ":memory:".to_string(),
            ..Default::default()
        };

        let state_manager = StateManager::new(config).await.unwrap();
        
        // Verify database was created and schema initialized
        let stats = state_manager.get_execution_stats().await.unwrap();
        assert_eq!(stats.total_executions, 0);
        
        state_manager.close().await;
    }

    #[tokio::test]
    async fn test_save_and_load_execution_state() {
        let config = StateConfig {
            database_path: ":memory:".to_string(),
            ..Default::default()
        };

        let state_manager = StateManager::new(config).await.unwrap();

        // Create test execution state
        let execution_id = Uuid::new_v4().to_string();
        let mut task_states = HashMap::new();
        task_states.insert("task1".to_string(), TaskState {
            task_id: "task1".to_string(),
            status: TaskStatus::Success,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            duration: Some(Duration::from_secs(5)),
            stdout: "Hello World".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            retry_count: 0,
            error_message: None,
            output_data: Some(serde_json::json!({"result": "success"})),
            metadata: HashMap::new(),
        });

        let execution_state = ExecutionState {
            execution_id: execution_id.clone(),
            dag_id: "test_dag".to_string(),
            status: ExecutionStatus::Success,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            total_duration: Some(Duration::from_secs(10)),
            task_states,
            metadata: HashMap::new(),
        };

        // Save state
        state_manager.save_execution_state(&execution_state).await.unwrap();

        // Load state
        let loaded_state = state_manager.load_execution_state(&execution_id).await.unwrap();
        assert!(loaded_state.is_some());
        
        let loaded_state = loaded_state.unwrap();
        assert_eq!(loaded_state.execution_id, execution_id);
        assert_eq!(loaded_state.dag_id, "test_dag");
        assert_eq!(loaded_state.status, ExecutionStatus::Success);
        assert_eq!(loaded_state.task_states.len(), 1);
        assert!(loaded_state.task_states.contains_key("task1"));

        state_manager.close().await;
    }

    #[tokio::test]
    async fn test_task_history() {
        let config = StateConfig {
            database_path: ":memory:".to_string(),
            ..Default::default()
        };

        let state_manager = StateManager::new(config).await.unwrap();

        // Create multiple execution states for the same task
        for i in 0..3 {
            let execution_id = Uuid::new_v4().to_string();
            let mut task_states = HashMap::new();
            task_states.insert("task1".to_string(), TaskState {
                task_id: "task1".to_string(),
                status: if i == 1 { TaskStatus::Failed } else { TaskStatus::Success },
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                duration: Some(Duration::from_secs(5)),
                stdout: format!("Output {}", i),
                stderr: String::new(),
                exit_code: Some(if i == 1 { 1 } else { 0 }),
                retry_count: 0,
                error_message: if i == 1 { Some("Test error".to_string()) } else { None },
                output_data: None,
                metadata: HashMap::new(),
            });

            let execution_state = ExecutionState {
                execution_id,
                dag_id: "test_dag".to_string(),
                status: if i == 1 { ExecutionStatus::Failed } else { ExecutionStatus::Success },
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                total_duration: Some(Duration::from_secs(10)),
                task_states,
                metadata: HashMap::new(),
            };

            state_manager.save_execution_state(&execution_state).await.unwrap();
        }

        // Get task history
        let history = state_manager.get_task_history("test_dag", "task1").await.unwrap();
        assert_eq!(history.len(), 3);

        // Verify one failed execution
        let failed_count = history.iter().filter(|h| h.status == TaskStatus::Failed).count();
        assert_eq!(failed_count, 1);

        state_manager.close().await;
    }

    #[tokio::test]
    async fn test_cleanup_old_executions() {
        let config = StateConfig {
            database_path: ":memory:".to_string(),
            retention_period: Duration::from_secs(1), // Very short retention for testing
            max_executions: 2,
            ..Default::default()
        };

        let state_manager = StateManager::new(config).await.unwrap();

        // Create old execution
        let old_execution = ExecutionState {
            execution_id: Uuid::new_v4().to_string(),
            dag_id: "test_dag".to_string(),
            status: ExecutionStatus::Success,
            started_at: Utc::now() - chrono::Duration::seconds(10),
            completed_at: Some(Utc::now() - chrono::Duration::seconds(5)),
            total_duration: Some(Duration::from_secs(5)),
            task_states: HashMap::new(),
            metadata: HashMap::new(),
        };

        state_manager.save_execution_state(&old_execution).await.unwrap();

        // Wait for retention period to pass
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Create new executions
        for i in 0..3 {
            let execution_state = ExecutionState {
                execution_id: Uuid::new_v4().to_string(),
                dag_id: "test_dag".to_string(),
                status: ExecutionStatus::Success,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                total_duration: Some(Duration::from_secs(5)),
                task_states: HashMap::new(),
                metadata: HashMap::new(),
            };

            state_manager.save_execution_state(&execution_state).await.unwrap();
        }

        // Cleanup should remove old and excess executions
        let deleted_count = state_manager.cleanup_old_executions().await.unwrap();
        assert!(deleted_count > 0);

        // Verify cleanup worked
        let stats = state_manager.get_execution_stats().await.unwrap();
        assert!(stats.total_executions <= 2); // max_executions limit

        state_manager.close().await;
    }
}