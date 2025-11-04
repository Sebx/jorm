use crate::core::engine::TaskResult;
use crate::core::{error::JormError, task::TaskType};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

pub struct JormExecutor;

impl Default for JormExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl JormExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(
        &self,
        task_name: &str,
        task_type: &TaskType,
    ) -> Result<TaskResult, JormError> {
        match task_type {
            TaskType::Jorm {
                command,
                args,
                working_dir,
            } => {
                self.execute_jorm_command(task_name, command, args.as_ref(), working_dir.as_deref())
                    .await
            }
            _ => Err(JormError::ExecutionError(format!(
                "Jorm executor cannot handle task type: {:?}",
                task_type
            ))),
        }
    }

    async fn execute_jorm_command(
        &self,
        task_name: &str,
        command: &str,
        args: Option<&Vec<String>>,
        working_dir: Option<&str>,
    ) -> Result<TaskResult, JormError> {
        if command.trim().is_empty() {
            return Err(JormError::ExecutionError("Empty jorm command".to_string()));
        }

        // Get the current executable path
        let current_exe = std::env::current_exe().map_err(|e| {
            JormError::ExecutionError(format!("Failed to get current executable path: {}", e))
        })?;

        // Create command to run jorm with the specified subcommand and args
        let mut cmd = Command::new(&current_exe);
        cmd.arg(command);

        if let Some(args_vec) = args {
            cmd.args(args_vec);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Set working directory if specified
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        // Execute with timeout (5 minutes for jorm commands as they might be complex)
        let timeout_duration = Duration::from_secs(300);

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
            Ok(Err(e)) => Err(JormError::ExecutionError(format!(
                "Failed to execute jorm command '{}': {}",
                command, e
            ))),
            Err(_) => Err(JormError::ExecutionError(format!(
                "Jorm command '{}' timed out after {} seconds",
                command,
                timeout_duration.as_secs()
            ))),
        }
    }
}
