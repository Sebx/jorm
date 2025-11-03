use crate::core::{error::JormError, task::TaskType};
use crate::core::engine::TaskResult;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

pub struct RustExecutor;

impl RustExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, task_name: &str, task_type: &TaskType) -> Result<TaskResult, JormError> {
        match task_type {
            TaskType::Rust { command, working_dir } => {
                self.execute_cargo_command(task_name, command, working_dir.as_deref()).await
            }
            _ => Err(JormError::ExecutionError(
                format!("Rust executor cannot handle task type: {:?}", task_type)
            )),
        }
    }

    async fn execute_cargo_command(
        &self,
        task_name: &str,
        command: &str,
        working_dir: Option<&str>,
    ) -> Result<TaskResult, JormError> {
        // Parse cargo command and arguments
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(JormError::ExecutionError(
                "Empty cargo command".to_string()
            ));
        }

        // Ensure the command starts with 'cargo'
        let (program, args) = if parts[0] == "cargo" {
            ("cargo", &parts[1..])
        } else {
            // If not starting with cargo, prepend it
            ("cargo", &parts[..])
        };

        // Create command
        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set working directory if specified
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        // Rust compilation can take longer, so use a longer timeout
        let timeout_duration = Duration::from_secs(300); // 5 minutes for builds
        
        match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                let success = output.status.success();
                
                // For Rust/Cargo, stderr often contains warnings and info, not just errors
                // So we include both stdout and stderr in output, and only treat it as error if command failed
                let combined_output = if !stdout.is_empty() && !stderr.is_empty() {
                    format!("{}\n{}", stdout, stderr)
                } else if !stdout.is_empty() {
                    stdout
                } else {
                    stderr.clone()
                };

                let error = if !success {
                    Some(format!("Cargo command failed: {}", stderr))
                } else {
                    None
                };

                Ok(TaskResult {
                    task_name: task_name.to_string(),
                    success,
                    output: combined_output,
                    error,
                })
            }
            Ok(Err(e)) => Err(JormError::ExecutionError(
                format!("Failed to execute cargo command '{}': {}", command, e)
            )),
            Err(_) => Err(JormError::ExecutionError(
                format!("Cargo command '{}' timed out after {} seconds", command, timeout_duration.as_secs())
            )),
        }
    }
}