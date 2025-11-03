use crate::core::{error::JormError, task::TaskType};
use crate::core::engine::TaskResult;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

pub struct PythonExecutor;

impl PythonExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, task_name: &str, task_type: &TaskType) -> Result<TaskResult, JormError> {
        match task_type {
            TaskType::Python { script, args, working_dir } => {
                self.execute_python_script(task_name, script, args.as_ref(), working_dir.as_deref()).await
            }
            _ => Err(JormError::ExecutionError(
                format!("Python executor cannot handle task type: {:?}", task_type)
            )),
        }
    }

    async fn execute_python_script(
        &self,
        task_name: &str,
        script: &str,
        args: Option<&Vec<String>>,
        working_dir: Option<&str>,
    ) -> Result<TaskResult, JormError> {
        // Create command - try python3 first, then python
        let mut cmd = Command::new("python3");
        cmd.arg(script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add script arguments if provided
        if let Some(script_args) = args {
            cmd.args(script_args);
        }

        // Set working directory if specified
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        // Try python3 first
        let timeout_duration = Duration::from_secs(60); // Python scripts might take longer
        
        let result = timeout(timeout_duration, cmd.output()).await;
        
        // If python3 fails, try python
        let output = match result {
            Ok(Ok(output)) => output,
            Ok(Err(_)) | Err(_) => {
                // Try with 'python' command
                let mut cmd = Command::new("python");
                cmd.arg(script)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                if let Some(script_args) = args {
                    cmd.args(script_args);
                }

                if let Some(dir) = working_dir {
                    cmd.current_dir(dir);
                }

                match timeout(timeout_duration, cmd.output()).await {
                    Ok(Ok(output)) => output,
                    Ok(Err(e)) => {
                        return Err(JormError::ExecutionError(
                            format!("Failed to execute Python script '{}': {}", script, e)
                        ));
                    }
                    Err(_) => {
                        return Err(JormError::ExecutionError(
                            format!("Python script '{}' timed out after {} seconds", script, timeout_duration.as_secs())
                        ));
                    }
                }
            }
        };

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
}