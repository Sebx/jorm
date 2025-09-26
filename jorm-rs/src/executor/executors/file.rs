//! File operation task executor implementation

use crate::executor::{
    ExecutionContext, TaskResult, ExecutorError, Task, TaskConfig, TaskStatus
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::fs;

/// File operation task executor for file system operations
pub struct FileTaskExecutor {
    /// Default timeout for file operations
    default_timeout: Duration,
}

impl FileTaskExecutor {
    /// Create a new file task executor
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(60), // 1 minute default
        }
    }
    
    /// Create a new file task executor with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            default_timeout: timeout,
        }
    }
    
    /// Execute a file copy operation
    async fn execute_copy_operation(
        &self,
        source: &str,
        destination: &str,
        options: &HashMap<String, serde_json::Value>,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError> {
        let start_time = Instant::now();
        let started_at = Utc::now();
        
        println!("📁 Copying file: {} -> {}", source, destination);
        
        let source_path = Path::new(source);
        let dest_path = Path::new(destination);
        
        // Validate source file exists
        if !source_path.exists() {
            return Err(ExecutorError::ConfigurationError {
                message: format!("Source file does not exist: {}", source),
            });
        }
        
        // Create destination directory if it doesn't exist
        if let Some(parent) = dest_path.parent() {
            if !parent.exists() {
                let create_dirs = options.get("create_dirs")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                
                if create_dirs {
                    fs::create_dir_all(parent).await
                        .map_err(|e| ExecutorError::TaskExecutionFailed {
                            task_id: context.task_id.clone(),
                            source: anyhow::anyhow!("Failed to create destination directory: {}", e),
                        })?;
                    println!("📂 Created directory: {}", parent.display());
                } else {
                    return Err(ExecutorError::ConfigurationError {
                        message: format!("Destination directory does not exist: {}", parent.display()),
                    });
                }
            }
        }
        
        // Check if destination exists and handle overwrite
        if dest_path.exists() {
            let overwrite = options.get("overwrite")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            if !overwrite {
                return Err(ExecutorError::ConfigurationError {
                    message: format!("Destination file already exists: {}", destination),
                });
            }
        }
        
        // Perform the copy operation
        let copy_result = fs::copy(source_path, dest_path).await;
        
        let duration = start_time.elapsed();
        let completed_at = Utc::now();
        
        match copy_result {
            Ok(bytes_copied) => {
                println!("✅ File copied successfully: {} bytes", bytes_copied);
                
                let mut metadata = HashMap::new();
                metadata.insert("operation".to_string(), serde_json::json!("copy"));
                metadata.insert("source".to_string(), serde_json::json!(source));
                metadata.insert("destination".to_string(), serde_json::json!(destination));
                metadata.insert("bytes_copied".to_string(), serde_json::json!(bytes_copied));
                
                Ok(TaskResult {
                    task_id: context.task_id.clone(),
                    status: TaskStatus::Success,
                    stdout: format!("Copied {} bytes from {} to {}", bytes_copied, source, destination),
                    stderr: String::new(),
                    exit_code: Some(0),
                    duration,
                    retry_count: 0,
                    started_at,
                    completed_at,
                    output_data: Some(serde_json::json!({
                        "operation": "copy",
                        "bytes_copied": bytes_copied,
                        "source": source,
                        "destination": destination
                    })),
                    error_message: None,
                    metadata,
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to copy file: {}", e);
                println!("❌ {}", error_msg);
                
                Err(ExecutorError::TaskExecutionFailed {
                    task_id: context.task_id.clone(),
                    source: anyhow::anyhow!(e),
                })
            }
        }
    }  
  /// Execute a file move operation
    async fn execute_move_operation(
        &self,
        source: &str,
        destination: &str,
        options: &HashMap<String, serde_json::Value>,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError> {
        let start_time = Instant::now();
        let started_at = Utc::now();
        
        println!("📁 Moving file: {} -> {}", source, destination);
        
        let source_path = Path::new(source);
        let dest_path = Path::new(destination);
        
        // Validate source file exists
        if !source_path.exists() {
            return Err(ExecutorError::ConfigurationError {
                message: format!("Source file does not exist: {}", source),
            });
        }
        
        // Get file size before moving
        let file_size = fs::metadata(source_path).await
            .map_err(|e| ExecutorError::TaskExecutionFailed {
                task_id: context.task_id.clone(),
                source: anyhow::anyhow!("Failed to get file metadata: {}", e),
            })?
            .len();
        
        // Create destination directory if it doesn't exist
        if let Some(parent) = dest_path.parent() {
            if !parent.exists() {
                let create_dirs = options.get("create_dirs")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                
                if create_dirs {
                    fs::create_dir_all(parent).await
                        .map_err(|e| ExecutorError::TaskExecutionFailed {
                            task_id: context.task_id.clone(),
                            source: anyhow::anyhow!("Failed to create destination directory: {}", e),
                        })?;
                    println!("📂 Created directory: {}", parent.display());
                } else {
                    return Err(ExecutorError::ConfigurationError {
                        message: format!("Destination directory does not exist: {}", parent.display()),
                    });
                }
            }
        }
        
        // Check if destination exists and handle overwrite
        if dest_path.exists() {
            let overwrite = options.get("overwrite")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            if !overwrite {
                return Err(ExecutorError::ConfigurationError {
                    message: format!("Destination file already exists: {}", destination),
                });
            }
        }
        
        // Perform the move operation
        let move_result = fs::rename(source_path, dest_path).await;
        
        let duration = start_time.elapsed();
        let completed_at = Utc::now();
        
        match move_result {
            Ok(()) => {
                println!("✅ File moved successfully: {} bytes", file_size);
                
                let mut metadata = HashMap::new();
                metadata.insert("operation".to_string(), serde_json::json!("move"));
                metadata.insert("source".to_string(), serde_json::json!(source));
                metadata.insert("destination".to_string(), serde_json::json!(destination));
                metadata.insert("file_size".to_string(), serde_json::json!(file_size));
                
                Ok(TaskResult {
                    task_id: context.task_id.clone(),
                    status: TaskStatus::Success,
                    stdout: format!("Moved {} bytes from {} to {}", file_size, source, destination),
                    stderr: String::new(),
                    exit_code: Some(0),
                    duration,
                    retry_count: 0,
                    started_at,
                    completed_at,
                    output_data: Some(serde_json::json!({
                        "operation": "move",
                        "file_size": file_size,
                        "source": source,
                        "destination": destination
                    })),
                    error_message: None,
                    metadata,
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to move file: {}", e);
                println!("❌ {}", error_msg);
                
                Err(ExecutorError::TaskExecutionFailed {
                    task_id: context.task_id.clone(),
                    source: anyhow::anyhow!(e),
                })
            }
        }
    }

    /// Execute a file delete operation
    async fn execute_delete_operation(
        &self,
        source: &str,
        options: &HashMap<String, serde_json::Value>,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError> {
        let start_time = Instant::now();
        let started_at = Utc::now();
        
        println!("🗑️ Deleting file: {}", source);
        
        let source_path = Path::new(source);
        
        // Validate source file exists
        if !source_path.exists() {
            let ignore_missing = options.get("ignore_missing")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            if ignore_missing {
                println!("⚠️ File does not exist (ignored): {}", source);
                
                let duration = start_time.elapsed();
                let completed_at = Utc::now();
                
                let mut metadata = HashMap::new();
                metadata.insert("operation".to_string(), serde_json::json!("delete"));
                metadata.insert("source".to_string(), serde_json::json!(source));
                metadata.insert("ignored_missing".to_string(), serde_json::json!(true));
                
                return Ok(TaskResult {
                    task_id: context.task_id.clone(),
                    status: TaskStatus::Success,
                    stdout: format!("File does not exist (ignored): {}", source),
                    stderr: String::new(),
                    exit_code: Some(0),
                    duration,
                    retry_count: 0,
                    started_at,
                    completed_at,
                    output_data: Some(serde_json::json!({
                        "operation": "delete",
                        "source": source,
                        "ignored_missing": true
                    })),
                    error_message: None,
                    metadata,
                });
            } else {
                return Err(ExecutorError::ConfigurationError {
                    message: format!("Source file does not exist: {}", source),
                });
            }
        }
        
        // Get file size before deleting
        let file_size = fs::metadata(source_path).await
            .map_err(|e| ExecutorError::TaskExecutionFailed {
                task_id: context.task_id.clone(),
                source: anyhow::anyhow!("Failed to get file metadata: {}", e),
            })?
            .len();
        
        // Perform the delete operation
        let delete_result = if source_path.is_dir() {
            let recursive = options.get("recursive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            if recursive {
                fs::remove_dir_all(source_path).await
            } else {
                fs::remove_dir(source_path).await
            }
        } else {
            fs::remove_file(source_path).await
        };
        
        let duration = start_time.elapsed();
        let completed_at = Utc::now();
        
        match delete_result {
            Ok(()) => {
                println!("✅ File deleted successfully: {} bytes", file_size);
                
                let mut metadata = HashMap::new();
                metadata.insert("operation".to_string(), serde_json::json!("delete"));
                metadata.insert("source".to_string(), serde_json::json!(source));
                metadata.insert("file_size".to_string(), serde_json::json!(file_size));
                
                Ok(TaskResult {
                    task_id: context.task_id.clone(),
                    status: TaskStatus::Success,
                    stdout: format!("Deleted {} bytes from {}", file_size, source),
                    stderr: String::new(),
                    exit_code: Some(0),
                    duration,
                    retry_count: 0,
                    started_at,
                    completed_at,
                    output_data: Some(serde_json::json!({
                        "operation": "delete",
                        "file_size": file_size,
                        "source": source
                    })),
                    error_message: None,
                    metadata,
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to delete file: {}", e);
                println!("❌ {}", error_msg);
                
                Err(ExecutorError::TaskExecutionFailed {
                    task_id: context.task_id.clone(),
                    source: anyhow::anyhow!(e),
                })
            }
        }
    }

    /// Execute a file create operation
    async fn execute_create_operation(
        &self,
        destination: &str,
        options: &HashMap<String, serde_json::Value>,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError> {
        let start_time = Instant::now();
        let started_at = Utc::now();
        
        println!("📝 Creating file: {}", destination);
        
        let dest_path = Path::new(destination);
        
        // Create destination directory if it doesn't exist
        if let Some(parent) = dest_path.parent() {
            if !parent.exists() {
                let create_dirs = options.get("create_dirs")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true); // Default to true for create operations
                
                if create_dirs {
                    fs::create_dir_all(parent).await
                        .map_err(|e| ExecutorError::TaskExecutionFailed {
                            task_id: context.task_id.clone(),
                            source: anyhow::anyhow!("Failed to create destination directory: {}", e),
                        })?;
                    println!("📂 Created directory: {}", parent.display());
                } else {
                    return Err(ExecutorError::ConfigurationError {
                        message: format!("Destination directory does not exist: {}", parent.display()),
                    });
                }
            }
        }
        
        // Check if destination exists and handle overwrite
        if dest_path.exists() {
            let overwrite = options.get("overwrite")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            if !overwrite {
                return Err(ExecutorError::ConfigurationError {
                    message: format!("Destination file already exists: {}", destination),
                });
            }
        }
        
        // Get content to write
        let content = options.get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        // Perform the create operation
        let create_result = fs::write(dest_path, content).await;
        
        let duration = start_time.elapsed();
        let completed_at = Utc::now();
        
        match create_result {
            Ok(()) => {
                let bytes_written = content.len();
                println!("✅ File created successfully: {} bytes", bytes_written);
                
                let mut metadata = HashMap::new();
                metadata.insert("operation".to_string(), serde_json::json!("create"));
                metadata.insert("destination".to_string(), serde_json::json!(destination));
                metadata.insert("bytes_written".to_string(), serde_json::json!(bytes_written));
                
                Ok(TaskResult {
                    task_id: context.task_id.clone(),
                    status: TaskStatus::Success,
                    stdout: format!("Created file {} with {} bytes", destination, bytes_written),
                    stderr: String::new(),
                    exit_code: Some(0),
                    duration,
                    retry_count: 0,
                    started_at,
                    completed_at,
                    output_data: Some(serde_json::json!({
                        "operation": "create",
                        "bytes_written": bytes_written,
                        "destination": destination
                    })),
                    error_message: None,
                    metadata,
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to create file: {}", e);
                println!("❌ {}", error_msg);
                
                Err(ExecutorError::TaskExecutionFailed {
                    task_id: context.task_id.clone(),
                    source: anyhow::anyhow!(e),
                })
            }
        }
    }
}

#[async_trait]
impl crate::executor::TaskExecutor for FileTaskExecutor {
    async fn execute(&self, task: &Task, context: &ExecutionContext) -> Result<TaskResult, ExecutorError> {
        // Extract file operation configuration from task
        match &task.config {
            TaskConfig::FileOperation { operation, source, destination, options } => {
                match operation.as_str() {
                    "copy" => {
                        let src = source.as_ref().ok_or_else(|| ExecutorError::ConfigurationError {
                            message: "Copy operation requires source".to_string(),
                        })?;
                        let dest = destination.as_ref().ok_or_else(|| ExecutorError::ConfigurationError {
                            message: "Copy operation requires destination".to_string(),
                        })?;
                        
                        self.execute_copy_operation(src, dest, options, context).await
                    }
                    "move" => {
                        let src = source.as_ref().ok_or_else(|| ExecutorError::ConfigurationError {
                            message: "Move operation requires source".to_string(),
                        })?;
                        let dest = destination.as_ref().ok_or_else(|| ExecutorError::ConfigurationError {
                            message: "Move operation requires destination".to_string(),
                        })?;
                        
                        self.execute_move_operation(src, dest, options, context).await
                    }
                    "delete" => {
                        let src = source.as_ref().ok_or_else(|| ExecutorError::ConfigurationError {
                            message: "Delete operation requires source".to_string(),
                        })?;
                        
                        self.execute_delete_operation(src, options, context).await
                    }
                    "create" => {
                        let dest = destination.as_ref().ok_or_else(|| ExecutorError::ConfigurationError {
                            message: "Create operation requires destination".to_string(),
                        })?;
                        
                        self.execute_create_operation(dest, options, context).await
                    }
                    _ => Err(ExecutorError::UnsupportedTaskType {
                        task_type: format!("Unsupported file operation: {}", operation),
                    }),
                }
            }
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: format!("Expected file operation task, got: {:?}", task.config),
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
            TaskConfig::FileOperation { operation, source, destination, options: _ } => {
                // Validate operation type
                match operation.as_str() {
                    "copy" | "move" => {
                        if source.is_none() {
                            return Err(ExecutorError::ConfigurationError {
                                message: format!("{} operation requires source", operation),
                            });
                        }
                        if destination.is_none() {
                            return Err(ExecutorError::ConfigurationError {
                                message: format!("{} operation requires destination", operation),
                            });
                        }
                    }
                    "delete" => {
                        if source.is_none() {
                            return Err(ExecutorError::ConfigurationError {
                                message: "Delete operation requires source".to_string(),
                            });
                        }
                    }
                    "create" => {
                        if destination.is_none() {
                            return Err(ExecutorError::ConfigurationError {
                                message: "Create operation requires destination".to_string(),
                            });
                        }
                    }
                    _ => {
                        return Err(ExecutorError::ConfigurationError {
                            message: format!("Unsupported file operation: {}", operation),
                        });
                    }
                }
                
                Ok(())
            }
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: "Expected file operation task configuration".to_string(),
            }),
        }
    }
}

impl Default for FileTaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{ExecutorConfig, ExecutionContext, TaskExecutor};
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_file_executor_creation() {
        let executor = FileTaskExecutor::new();
        assert_eq!(executor.task_type(), "file");
        assert!(executor.supports_parallel());
        assert_eq!(executor.default_timeout(), Duration::from_secs(60));
    }
    
    #[tokio::test]
    async fn test_file_executor_with_custom_timeout() {
        let timeout = Duration::from_secs(120);
        let executor = FileTaskExecutor::with_timeout(timeout);
        assert_eq!(executor.default_timeout(), timeout);
    }
    
    #[tokio::test]
    async fn test_file_task_validation() {
        let executor = FileTaskExecutor::new();
        
        // Valid copy task
        let valid_copy_task = Task::new(
            "test".to_string(),
            "Test Task".to_string(),
            "file".to_string(),
            TaskConfig::FileOperation {
                operation: "copy".to_string(),
                source: Some("source.txt".to_string()),
                destination: Some("dest.txt".to_string()),
                options: HashMap::new(),
            },
        );
        
        assert!(executor.validate_task(&valid_copy_task).is_ok());
        
        // Invalid copy task (missing source)
        let invalid_copy_task = Task::new(
            "test".to_string(),
            "Test Task".to_string(),
            "file".to_string(),
            TaskConfig::FileOperation {
                operation: "copy".to_string(),
                source: None,
                destination: Some("dest.txt".to_string()),
                options: HashMap::new(),
            },
        );
        
        assert!(executor.validate_task(&invalid_copy_task).is_err());
        
        // Valid delete task
        let valid_delete_task = Task::new(
            "test".to_string(),
            "Test Task".to_string(),
            "file".to_string(),
            TaskConfig::FileOperation {
                operation: "delete".to_string(),
                source: Some("file.txt".to_string()),
                destination: None,
                options: HashMap::new(),
            },
        );
        
        assert!(executor.validate_task(&valid_delete_task).is_ok());
        
        // Invalid operation
        let invalid_operation_task = Task::new(
            "test".to_string(),
            "Test Task".to_string(),
            "file".to_string(),
            TaskConfig::FileOperation {
                operation: "invalid".to_string(),
                source: Some("file.txt".to_string()),
                destination: None,
                options: HashMap::new(),
            },
        );
        
        assert!(executor.validate_task(&invalid_operation_task).is_err());
    }    #
[tokio::test]
    async fn test_file_copy_operation() {
        let executor = FileTaskExecutor::new();
        let config = ExecutorConfig::default();
        let context = ExecutionContext::new(
            "test_dag".to_string(),
            "test_task".to_string(),
            config,
        );
        
        // Create temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("source.txt");
        let dest_path = temp_dir.path().join("dest.txt");
        
        // Create source file
        fs::write(&source_path, "Hello, World!").await.unwrap();
        
        let task = Task::new(
            "copy_test".to_string(),
            "Copy Test".to_string(),
            "file".to_string(),
            TaskConfig::FileOperation {
                operation: "copy".to_string(),
                source: Some(source_path.to_string_lossy().to_string()),
                destination: Some(dest_path.to_string_lossy().to_string()),
                options: HashMap::new(),
            },
        );
        
        let result = executor.execute(&task, &context).await.unwrap();
        
        assert_eq!(result.status, TaskStatus::Success);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("Copied"));
        
        // Verify file was copied
        assert!(dest_path.exists());
        let dest_content = fs::read_to_string(&dest_path).await.unwrap();
        assert_eq!(dest_content, "Hello, World!");
        
        // Check metadata
        assert!(result.metadata.contains_key("bytes_copied"));
        assert_eq!(result.metadata["bytes_copied"], serde_json::json!(13));
    }
    
    #[tokio::test]
    async fn test_file_create_operation() {
        let executor = FileTaskExecutor::new();
        let config = ExecutorConfig::default();
        let context = ExecutionContext::new(
            "test_dag".to_string(),
            "test_task".to_string(),
            config,
        );
        
        // Create temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("new_file.txt");
        
        let mut options = HashMap::new();
        options.insert("content".to_string(), serde_json::json!("New file content"));
        
        let task = Task::new(
            "create_test".to_string(),
            "Create Test".to_string(),
            "file".to_string(),
            TaskConfig::FileOperation {
                operation: "create".to_string(),
                source: None,
                destination: Some(file_path.to_string_lossy().to_string()),
                options,
            },
        );
        
        let result = executor.execute(&task, &context).await.unwrap();
        
        assert_eq!(result.status, TaskStatus::Success);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("Created file"));
        
        // Verify file was created
        assert!(file_path.exists());
        let file_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(file_content, "New file content");
        
        // Check metadata
        assert!(result.metadata.contains_key("bytes_written"));
        assert_eq!(result.metadata["bytes_written"], serde_json::json!(16));
    }
    
    #[tokio::test]
    async fn test_file_delete_operation() {
        let executor = FileTaskExecutor::new();
        let config = ExecutorConfig::default();
        let context = ExecutionContext::new(
            "test_dag".to_string(),
            "test_task".to_string(),
            config,
        );
        
        // Create temporary directory and file for testing
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("to_delete.txt");
        
        // Create file to delete
        fs::write(&file_path, "Delete me!").await.unwrap();
        assert!(file_path.exists());
        
        let task = Task::new(
            "delete_test".to_string(),
            "Delete Test".to_string(),
            "file".to_string(),
            TaskConfig::FileOperation {
                operation: "delete".to_string(),
                source: Some(file_path.to_string_lossy().to_string()),
                destination: None,
                options: HashMap::new(),
            },
        );
        
        let result = executor.execute(&task, &context).await.unwrap();
        
        assert_eq!(result.status, TaskStatus::Success);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("Deleted"));
        
        // Verify file was deleted
        assert!(!file_path.exists());
        
        // Check metadata
        assert!(result.metadata.contains_key("file_size"));
        assert_eq!(result.metadata["file_size"], serde_json::json!(10));
    }
}