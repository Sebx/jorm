use crate::executor::{ExecutionContext, ExecutorError, Task, TaskExecutor, TaskResult, TaskStatus};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::fs as tokio_fs;

/// File task executor for file operations (copy, move, delete)
pub struct FileTaskExecutor {
    default_timeout: Duration,
}

impl FileTaskExecutor {
    /// Create a new file task executor
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(60),
        }
    }

    /// Execute a copy operation
    async fn execute_copy_operation(
        &self,
        source: &str,
        destination: &str,
        options: &HashMap<String, serde_json::Value>,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError> {
        let start_time = Instant::now();
        let started_at = Utc::now();

        let source_path = PathBuf::from(source);
        let dest_path = PathBuf::from(destination);

        // Resolve relative paths against context working directory
        let source_abs = if source_path.is_absolute() {
            source_path
        } else {
            context
                .effective_working_directory()
                .map(|d| d.join(&source_path))
                .unwrap_or_else(|| source_path.clone())
        };

        let dest_abs = if dest_path.is_absolute() {
            dest_path
        } else {
            context
                .effective_working_directory()
                .map(|d| d.join(&dest_path))
                .unwrap_or_else(|| dest_path.clone())
        };

        println!("📁 Copying {} -> {}", source_abs.display(), dest_abs.display());

        // Wait for source to exist with retries (handle OneDrive/AV delays)
        let mut source_exists = false;
        for attempt in 1..=6 {
            match tokio_fs::metadata(&source_abs).await {
                Ok(_) => {
                    source_exists = true;
                    break;
                }
                Err(e) => {
                    if attempt < 6 {
                        println!(
                            "⚠️ Source not present yet (attempt {}): {}",
                            attempt,
                            e
                        );
                        tokio::time::sleep(Duration::from_millis(25 * attempt)).await;
                    } else {
                        println!("❌ Source still missing after retries: {}", e);
                    }
                }
            }
        }

        if !source_exists {
            return Err(ExecutorError::ConfigurationError {
                message: format!("Source file does not exist: {}", source_abs.display()),
            });
        }

        // Ensure destination parent exists
        if let Some(parent) = dest_abs.parent() {
            if !parent.exists() {
                tokio_fs::create_dir_all(parent).await.map_err(|e| {
                    ExecutorError::IoError {
                        message: format!("Failed to create destination directory: {}", e),
                        source: e,
                    }
                })?;
            }
        }

        // Perform copy with retries
        let mut last_error = None;
        for attempt in 1..=3 {
            match tokio_fs::copy(&source_abs, &dest_abs).await {
                Ok(bytes) => {
                    let duration = start_time.elapsed();
                    let completed_at = Utc::now();

                    println!("✅ Copy succeeded: {} bytes", bytes);

                    let mut metadata = HashMap::new();
                    metadata.insert(
                        "operation".to_string(),
                        serde_json::json!("copy"),
                    );
                    metadata.insert(
                        "source".to_string(),
                        serde_json::json!(source_abs.display().to_string()),
                    );
                    metadata.insert(
                        "destination".to_string(),
                        serde_json::json!(dest_abs.display().to_string()),
                    );
                    metadata.insert("bytes".to_string(), serde_json::json!(bytes));

                    return Ok(TaskResult {
                        task_id: context.task_id.clone(),
                        status: TaskStatus::Success,
                        stdout: format!("Copied {} bytes", bytes),
                        stderr: String::new(),
                        exit_code: Some(0),
                        duration,
                        retry_count: attempt - 1,
                        started_at,
                        completed_at,
                        output_data: None,
                        error_message: None,
                        metadata,
                    });
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 3 {
                        println!("⚠️ Copy failed (attempt {}), retrying...", attempt);
                        tokio::time::sleep(Duration::from_millis(50u64 * u64::from(attempt))).await;
                    }
                }
            }
        }

        Err(ExecutorError::IoError {
            message: format!(
                "Failed to copy {} -> {} after 3 attempts",
                source_abs.display(),
                dest_abs.display()
            ),
            source: last_error.unwrap(),
        })
    }
}

#[async_trait]
impl TaskExecutor for FileTaskExecutor {
    async fn execute(
        &self,
        task: &Task,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError> {
        match &task.config {
            crate::executor::TaskConfig::File {
                operation,
                source,
                destination,
                options,
            } => match operation.as_str() {
                "copy" => {
                    let src = source.as_deref().ok_or_else(|| {
                        ExecutorError::ConfigurationError {
                            message: "Source path is required for copy operation".to_string(),
                        }
                    })?;
                    let dst = destination.as_deref().ok_or_else(|| {
                        ExecutorError::ConfigurationError {
                            message: "Destination path is required for copy operation".to_string(),
                        }
                    })?;
                    let opts = options;
                    self.execute_copy_operation(src, dst, opts, context).await
                }
                _ => Err(ExecutorError::ConfigurationError {
                    message: format!(
                        "Unsupported or missing file operation: {:?}",
                        operation
                    ),
                }),
            },
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: format!("Expected File task, got: {:?}", task.config),
            }),
        }
    }

    fn task_type(&self) -> &'static str {
        "file"
    }

    fn supports_parallel(&self) -> bool {
        true
    }

    fn default_timeout(&self) -> Duration {
        self.default_timeout
    }

    fn validate_task(&self, task: &Task) -> Result<(), ExecutorError> {
        match &task.config {
            crate::executor::TaskConfig::File {
                operation,
                source,
                destination,
                options: _,
            } => {
                match operation.as_str() {
                    "copy" => {
                        if source.is_none() {
                            return Err(ExecutorError::ConfigurationError {
                                message: "Source path is required for copy operation".to_string(),
                            });
                        }
                        if destination.is_none() {
                            return Err(ExecutorError::ConfigurationError {
                                message: "Destination path is required for copy operation".to_string(),
                            });
                        }
                    }
                    _ => {
                        return Err(ExecutorError::ConfigurationError {
                            message: format!(
                                "Unsupported file operation: {:?}",
                                operation
                            ),
                        });
                    }
                }
                Ok(())
            }
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: format!("Expected File task, got: {:?}", task.config),
            }),
        }
    }

    fn clone_box(&self) -> Box<dyn crate::executor::TaskExecutor> {
        Box::new(Self {
            default_timeout: self.default_timeout,
        })
    }
}

impl Default for FileTaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}