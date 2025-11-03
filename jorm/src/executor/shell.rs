use crate::core::{error::JormError, task::TaskType};
use crate::core::engine::TaskResult;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

pub struct ShellExecutor;

impl ShellExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, task_name: &str, task_type: &TaskType) -> Result<TaskResult, JormError> {
        match task_type {
            TaskType::Shell { command, working_dir } => {
                self.execute_shell_command(task_name, command, working_dir.as_deref()).await
            }
            _ => Err(JormError::ExecutionError(
                format!("Shell executor cannot handle task type: {:?}", task_type)
            )),
        }
    }

    async fn execute_shell_command(
        &self,
        task_name: &str,
        command: &str,
        working_dir: Option<&str>,
    ) -> Result<TaskResult, JormError> {
        if command.trim().is_empty() {
            return Err(JormError::ExecutionError(
                "Empty shell command".to_string()
            ));
        }

        // Create command based on platform
        let mut cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.args(&["/C", command]);
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.args(&["-c", command]);
            cmd
        };

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set working directory if specified
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        // Execute with timeout (30 seconds default)
        let timeout_duration = Duration::from_secs(30);
        
        match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                let success = output.status.success();
                let error = if !stderr.is_empty() && !success {
                    Some(stderr)
                } else {
                    None
                };

                Ok(TaskResult {
                    task_name: task_name.to_string(),
                    success,
                    output: stdout,
                    error,
                })
            }
            Ok(Err(e)) => Err(JormError::ExecutionError(
                format!("Failed to execute shell command '{}': {}", command, e)
            )),
            Err(_) => Err(JormError::ExecutionError(
                format!("Shell command '{}' timed out after {} seconds", command, timeout_duration.as_secs())
            )),
        }
    }
}