//! Shell task executor implementation

use crate::executor::{ExecutionContext, ExecutorError, Task, TaskConfig, TaskResult, TaskStatus};
use async_trait::async_trait;
use chrono::Utc;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

/// Shell task executor for running shell commands
pub struct ShellTaskExecutor {
    /// Default timeout for shell commands
    default_timeout: Duration,

    /// Default shell to use (platform-specific)
    default_shell: String,
}

impl ShellTaskExecutor {
    /// Create a new shell task executor
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(300), // 5 minutes
            default_shell: Self::get_default_shell(),
        }
    }

    /// Create a new shell task executor with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            default_timeout: timeout,
            default_shell: Self::get_default_shell(),
        }
    }

    /// Create a new shell task executor with custom shell
    pub fn with_shell(shell: String) -> Self {
        Self {
            default_timeout: Duration::from_secs(300),
            default_shell: shell,
        }
    }

    /// Get the default shell for the current platform
    fn get_default_shell() -> String {
        if cfg!(target_os = "windows") {
            "cmd".to_string()
        } else {
            "sh".to_string()
        }
    }

    /// Get shell command arguments for the current platform
    fn get_shell_args(&self, shell: &str, command: &str) -> Vec<String> {
        if shell == "cmd" || shell.ends_with("cmd.exe") {
            vec!["/C".to_string(), command.to_string()]
        } else if shell == "powershell" || shell.ends_with("powershell.exe") {
            vec!["-Command".to_string(), command.to_string()]
        } else {
            // Unix-like shells (sh, bash, zsh, etc.)
            vec!["-c".to_string(), command.to_string()]
        }
    }

    /// Execute a shell command with the given configuration
    async fn execute_shell_command(
        &self,
        command: &str,
        working_dir: Option<&str>,
        shell: Option<&str>,
        context: &ExecutionContext,
        task_timeout: Duration,
    ) -> Result<TaskResult, ExecutorError> {
        let start_time = Instant::now();
        let started_at = Utc::now();

        // Determine shell to use
        let shell_cmd = shell.unwrap_or(&self.default_shell);
        let shell_args = self.get_shell_args(shell_cmd, command);

        println!(
            "🐚 Executing shell command: {command} (shell: {shell_cmd})"
        );

        // Create the command
        let mut cmd = Command::new(shell_cmd);
        cmd.args(&shell_args);

        // Set working directory if specified
        if let Some(dir) = working_dir.or_else(|| {
            context
                .effective_working_directory()
                .map(|p| p.to_str().unwrap_or("."))
        }) {
            cmd.current_dir(dir);
            println!("📁 Working directory: {dir}");
        }

        // Set environment variables from context
        for (key, value) in context.env_vars_for_process() {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // Execute with timeout
        let execution_result = timeout(task_timeout, cmd.output()).await;

        let duration = start_time.elapsed();
        let completed_at = Utc::now();

        match execution_result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();

                let status = if output.status.success() {
                    TaskStatus::Success
                } else {
                    TaskStatus::Failed
                };

                println!("✅ Shell command completed with exit code: {exit_code:?}");
                if !stdout.is_empty() {
                    println!("📤 stdout: {}", stdout.trim());
                }
                if !stderr.is_empty() {
                    println!("📤 stderr: {}", stderr.trim());
                }

                Ok(TaskResult {
                    task_id: context.task_id.clone(),
                    status,
                    stdout,
                    stderr,
                    exit_code,
                    duration,
                    retry_count: 0,
                    started_at,
                    completed_at,
                    output_data: None,
                    error_message: if status == TaskStatus::Failed {
                        Some(format!("Command failed with exit code: {exit_code:?}"))
                    } else {
                        None
                    },
                    metadata: std::collections::HashMap::new(),
                })
            }
            Ok(Err(io_error)) => {
                let error_msg = format!("Failed to execute shell command: {io_error}");
                println!("❌ {error_msg}");

                Err(ExecutorError::TaskExecutionFailed {
                    task_id: context.task_id.clone(),
                    source: anyhow::anyhow!(io_error),
                })
            }
            Err(_timeout_error) => {
                let error_msg = format!("Shell command timed out after {task_timeout:?}");
                println!("⏰ {error_msg}");

                Err(ExecutorError::TaskTimeout {
                    task_id: context.task_id.clone(),
                    timeout: task_timeout,
                })
            }
        }
    }
}

#[async_trait]
impl crate::executor::TaskExecutor for ShellTaskExecutor {
    async fn execute(
        &self,
        task: &Task,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError> {
        // Extract shell configuration from task
        match &task.config {
            TaskConfig::Shell {
                command,
                working_dir,
                shell,
            } => {
                // Determine timeout
                let task_timeout = task.effective_timeout(self.default_timeout);

                // Execute the shell command
                self.execute_shell_command(
                    command,
                    working_dir.as_deref(),
                    shell.as_deref(),
                    context,
                    task_timeout,
                )
                .await
            }
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: format!("Expected Shell task, got: {:?}", task.config),
            }),
        }
    }

    fn task_type(&self) -> &'static str {
        "shell"
    }

    fn supports_parallel(&self) -> bool {
        true
    }

    fn default_timeout(&self) -> Duration {
        self.default_timeout
    }

    fn validate_task(&self, task: &Task) -> Result<(), ExecutorError> {
        match &task.config {
            TaskConfig::Shell {
                command,
                working_dir,
                shell: _,
            } => {
                // Validate command is not empty
                if command.trim().is_empty() {
                    return Err(ExecutorError::ConfigurationError {
                        message: "Shell command cannot be empty".to_string(),
                    });
                }

                // Validate working directory exists if specified
                if let Some(dir) = working_dir {
                    if !std::path::Path::new(dir).exists() {
                        return Err(ExecutorError::ConfigurationError {
                            message: format!("Working directory does not exist: {dir}"),
                        });
                    }
                }

                Ok(())
            }
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: "Expected Shell task configuration".to_string(),
            }),
        }
    }
}

impl Default for ShellTaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{ExecutionContext, ExecutorConfig, TaskExecutor};

    #[tokio::test]
    async fn test_shell_executor_creation() {
        let executor = ShellTaskExecutor::new();
        assert_eq!(executor.task_type(), "shell");
        assert!(executor.supports_parallel());
        assert_eq!(executor.default_timeout(), Duration::from_secs(300));
    }

    #[tokio::test]
    async fn test_shell_executor_with_custom_timeout() {
        let timeout = Duration::from_secs(60);
        let executor = ShellTaskExecutor::with_timeout(timeout);
        assert_eq!(executor.default_timeout(), timeout);
    }

    #[tokio::test]
    async fn test_shell_executor_with_custom_shell() {
        let executor = ShellTaskExecutor::with_shell("bash".to_string());
        assert_eq!(executor.default_shell, "bash");
    }

    #[tokio::test]
    async fn test_shell_args_generation() {
        let executor = ShellTaskExecutor::new();

        // Test Windows cmd
        let args = executor.get_shell_args("cmd", "echo hello");
        assert_eq!(args, vec!["/C", "echo hello"]);

        // Test PowerShell
        let args = executor.get_shell_args("powershell", "Write-Host hello");
        assert_eq!(args, vec!["-Command", "Write-Host hello"]);

        // Test Unix shell
        let args = executor.get_shell_args("sh", "echo hello");
        assert_eq!(args, vec!["-c", "echo hello"]);
    }

    #[tokio::test]
    async fn test_shell_task_validation() {
        let executor = ShellTaskExecutor::new();

        // Valid shell task
        let valid_task = Task::new(
            "test".to_string(),
            "Test Task".to_string(),
            "shell".to_string(),
            TaskConfig::Shell {
                command: "echo hello".to_string(),
                working_dir: None,
                shell: None,
            },
        );

        assert!(executor.validate_task(&valid_task).is_ok());

        // Invalid shell task (empty command)
        let invalid_task = Task::new(
            "test".to_string(),
            "Test Task".to_string(),
            "shell".to_string(),
            TaskConfig::Shell {
                command: "   ".to_string(), // Empty/whitespace command
                working_dir: None,
                shell: None,
            },
        );

        assert!(executor.validate_task(&invalid_task).is_err());
    }

    #[tokio::test]
    async fn test_simple_shell_execution() {
        let executor = ShellTaskExecutor::new();
        let config = ExecutorConfig::default();
        let context =
            ExecutionContext::new("test_dag".to_string(), "test_task".to_string(), config);

        let task = Task::new(
            "echo_test".to_string(),
            "Echo Test".to_string(),
            "shell".to_string(),
            TaskConfig::Shell {
                command: if cfg!(target_os = "windows") {
                    "echo Hello World".to_string()
                } else {
                    "echo 'Hello World'".to_string()
                },
                working_dir: None,
                shell: None,
            },
        );

        let result = executor.execute(&task, &context).await;
        assert!(result.is_ok());

        let task_result = result.unwrap();
        assert_eq!(task_result.status, TaskStatus::Success);
        assert!(task_result.stdout.contains("Hello World"));
        assert_eq!(task_result.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_shell_execution_with_failure() {
        let executor = ShellTaskExecutor::new();
        let config = ExecutorConfig::default();
        let context =
            ExecutionContext::new("test_dag".to_string(), "test_task".to_string(), config);

        let task = Task::new(
            "fail_test".to_string(),
            "Fail Test".to_string(),
            "shell".to_string(),
            TaskConfig::Shell {
                command: "exit 1".to_string(),
                working_dir: None,
                shell: None,
            },
        );

        let result = executor.execute(&task, &context).await;
        assert!(result.is_ok());

        let task_result = result.unwrap();
        assert_eq!(task_result.status, TaskStatus::Failed);
        assert_eq!(task_result.exit_code, Some(1));
        assert!(task_result.error_message.is_some());
    }
}
