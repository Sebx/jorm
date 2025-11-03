use crate::core::{error::JormError, task::TaskType};
use crate::core::engine::TaskResult;
use std::path::Path;
use tokio::fs;

pub struct FileExecutor;

impl FileExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, task_name: &str, task_type: &TaskType) -> Result<TaskResult, JormError> {
        match task_type {
            TaskType::FileCopy { source, destination } => {
                self.copy_file(task_name, source, destination).await
            }
            TaskType::FileMove { source, destination } => {
                self.move_file(task_name, source, destination).await
            }
            TaskType::FileDelete { path } => {
                self.delete_file(task_name, path).await
            }
            _ => Err(JormError::ExecutionError(
                format!("File executor cannot handle task type: {:?}", task_type)
            )),
        }
    }

    async fn copy_file(
        &self,
        task_name: &str,
        source: &str,
        destination: &str,
    ) -> Result<TaskResult, JormError> {
        // Validate source file exists
        if !Path::new(source).exists() {
            return Ok(TaskResult {
                task_name: task_name.to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("Source file does not exist: {}", source)),
            });
        }

        // Create destination directory if it doesn't exist
        if let Some(parent) = Path::new(destination).parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent).await {
                    return Ok(TaskResult {
                        task_name: task_name.to_string(),
                        success: false,
                        output: String::new(),
                        error: Some(format!("Failed to create destination directory: {}", e)),
                    });
                }
            }
        }

        // Perform the copy operation
        match fs::copy(source, destination).await {
            Ok(bytes_copied) => Ok(TaskResult {
                task_name: task_name.to_string(),
                success: true,
                output: format!("Successfully copied {} bytes from '{}' to '{}'", bytes_copied, source, destination),
                error: None,
            }),
            Err(e) => Ok(TaskResult {
                task_name: task_name.to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("Failed to copy file from '{}' to '{}': {}", source, destination, e)),
            }),
        }
    }

    async fn move_file(
        &self,
        task_name: &str,
        source: &str,
        destination: &str,
    ) -> Result<TaskResult, JormError> {
        // Validate source file exists
        if !Path::new(source).exists() {
            return Ok(TaskResult {
                task_name: task_name.to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("Source file does not exist: {}", source)),
            });
        }

        // Create destination directory if it doesn't exist
        if let Some(parent) = Path::new(destination).parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent).await {
                    return Ok(TaskResult {
                        task_name: task_name.to_string(),
                        success: false,
                        output: String::new(),
                        error: Some(format!("Failed to create destination directory: {}", e)),
                    });
                }
            }
        }

        // Perform the move operation
        match fs::rename(source, destination).await {
            Ok(_) => Ok(TaskResult {
                task_name: task_name.to_string(),
                success: true,
                output: format!("Successfully moved '{}' to '{}'", source, destination),
                error: None,
            }),
            Err(e) => Ok(TaskResult {
                task_name: task_name.to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("Failed to move file from '{}' to '{}': {}", source, destination, e)),
            }),
        }
    }

    async fn delete_file(
        &self,
        task_name: &str,
        path: &str,
    ) -> Result<TaskResult, JormError> {
        let file_path = Path::new(path);
        
        // Check if file exists
        if !file_path.exists() {
            return Ok(TaskResult {
                task_name: task_name.to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("File does not exist: {}", path)),
            });
        }

        // Check if it's a file or directory
        let is_dir = file_path.is_dir();
        
        // Perform the delete operation
        let result = if is_dir {
            fs::remove_dir_all(path).await
        } else {
            fs::remove_file(path).await
        };

        match result {
            Ok(_) => {
                let item_type = if is_dir { "directory" } else { "file" };
                Ok(TaskResult {
                    task_name: task_name.to_string(),
                    success: true,
                    output: format!("Successfully deleted {} '{}'", item_type, path),
                    error: None,
                })
            }
            Err(e) => {
                let item_type = if is_dir { "directory" } else { "file" };
                Ok(TaskResult {
                    task_name: task_name.to_string(),
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to delete {} '{}': {}", item_type, path, e)),
                })
            }
        }
    }
}