//! Shell task executor implementation
//!
//! This module provides the [`ShellTaskExecutor`] for executing shell commands and scripts.
//! It supports cross-platform execution with configurable shells, timeouts, and working directories.
//!
//! # Features
//!
//! - **Cross-platform**: Supports Windows (cmd, PowerShell) and Unix-like systems (sh, bash, zsh)
//! - **Configurable Shell**: Can use different shells per task or executor instance
//! - **Working Directory**: Supports setting working directory for command execution
//! - **Timeout Handling**: Configurable timeouts with proper process cleanup
//! - **Output Capture**: Captures both stdout and stderr streams
//! - **Exit Code Handling**: Properly handles and reports process exit codes
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```rust
//! use jorm::executor::{ShellTaskExecutor, Task, TaskConfig, ExecutionContext};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = ShellTaskExecutor::new();
//!
//! let task = Task::new(
//!     "hello".to_string(),
//!     "Hello World".to_string(),
//!     "shell".to_string(),
//!     TaskConfig::Shell {
//!         command: "echo 'Hello, World!'".to_string(),
//!         working_dir: None,
//!         shell: None,
//!     },
//! );
//!
//! let context = ExecutionContext::new("test_dag".to_string(), "hello".to_string(), Default::default());
//! let result = executor.execute(&task, &context).await?;
//!
//! assert_eq!(result.stdout.trim(), "Hello, World!");
//! # Ok(())
//! # }
//! ```
//!
//! ## With Custom Shell
//!
//! ```rust
//! use jorm::executor::{ShellTaskExecutor, Task, TaskConfig};
//!
//! let executor = ShellTaskExecutor::with_shell("bash".to_string());
//!
//! let task = Task::new(
//!     "bash_task".to_string(),
//!     "Bash Script".to_string(),
//!     "shell".to_string(),
//!     TaskConfig::Shell {
//!         command: "echo $SHELL".to_string(),
//!         working_dir: None,
//!         shell: Some("bash".to_string()),
//!     },
//! );
//! ```
//!
//! ## With Working Directory
//!
//! ```rust
//! use jorm::executor::{Task, TaskConfig};
//!
//! let task = Task::new(
//!     "list_files".to_string(),
//!     "List Files".to_string(),
//!     "shell".to_string(),
//!     TaskConfig::Shell {
//!         command: "ls -la".to_string(),
//!         working_dir: Some("/tmp".to_string()),
//!         shell: None,
//!     },
//! );
//! ```

use crate::executor::{ExecutionContext, ExecutorError, Task, TaskConfig, TaskResult, TaskStatus};
use async_trait::async_trait;
use chrono::Utc;
use std::process::Stdio;
use tokio::fs as tokio_fs;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

/// Shell task executor for running shell commands
///
/// The `ShellTaskExecutor` handles execution of shell commands and scripts across
/// different platforms. It automatically detects the appropriate shell for the
/// current platform and provides configurable timeouts and working directories.
///
/// # Platform Support
///
/// - **Windows**: Uses `cmd` by default, supports PowerShell
/// - **Unix-like**: Uses `sh` by default, supports bash, zsh, and other shells
///
/// # Configuration
///
/// The executor can be configured with:
/// - Custom default timeout
/// - Custom default shell
/// - Per-task shell override
/// - Per-task working directory
///
/// # Examples
///
/// ## Default Configuration
///
/// ```rust
/// use jorm::executor::ShellTaskExecutor;
///
/// let executor = ShellTaskExecutor::new();
/// // Uses platform default shell with 5-minute timeout
/// ```
///
/// ## Custom Timeout
///
/// ```rust
/// use jorm::executor::ShellTaskExecutor;
/// use std::time::Duration;
///
/// let executor = ShellTaskExecutor::with_timeout(Duration::from_secs(120));
/// // 2-minute timeout for all commands
/// ```
///
/// ## Custom Shell
///
/// ```rust
/// use jorm::executor::ShellTaskExecutor;
///
/// let executor = ShellTaskExecutor::with_shell("bash".to_string());
/// // Always use bash instead of platform default
/// ```
pub struct ShellTaskExecutor {
    /// Default timeout for shell commands
    default_timeout: Duration,

    /// Default shell to use (platform-specific)
    default_shell: String,
}

impl ShellTaskExecutor {
    /// Create a new shell task executor with default settings
    ///
    /// Creates a new shell executor with:
    /// - 5-minute default timeout
    /// - Platform-appropriate default shell (cmd on Windows, sh on Unix-like)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jorm::executor::ShellTaskExecutor;
    ///
    /// let executor = ShellTaskExecutor::new();
    /// ```
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(300), // 5 minutes
            default_shell: Self::get_default_shell(),
        }
    }

    /// Create a new shell task executor with custom timeout
    ///
    /// Creates a shell executor with a custom default timeout. Individual tasks
    /// can still override this timeout in their configuration.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The default timeout for shell commands
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jorm::executor::ShellTaskExecutor;
    /// use std::time::Duration;
    ///
    /// let executor = ShellTaskExecutor::with_timeout(Duration::from_secs(120));
    /// // Commands will timeout after 2 minutes by default
    /// ```
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            default_timeout: timeout,
            default_shell: Self::get_default_shell(),
        }
    }

    /// Create a new shell task executor with custom shell
    ///
    /// Creates a shell executor that uses a specific shell by default.
    /// Individual tasks can still override this shell in their configuration.
    ///
    /// # Arguments
    ///
    /// * `shell` - The shell command to use (e.g., "bash", "zsh", "powershell")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jorm::executor::ShellTaskExecutor;
    ///
    /// let executor = ShellTaskExecutor::with_shell("bash".to_string());
    /// // All commands will use bash by default
    /// ```
    ///
    /// # Platform Considerations
    ///
    /// Ensure the specified shell is available on the target platform:
    /// - Unix-like: "sh", "bash", "zsh", "fish", etc.
    /// - Windows: "cmd", "powershell", "pwsh", etc.
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

        

        // Determine effective working directory for diagnostics.
        // Prefer the task-specified working dir, then the context-provided
        // working directory, but only if the candidate actually exists and
        // is a directory. Otherwise fall back to the current process cwd so
        // that path resolution and prechecks operate on a real directory.
        let effective_dir: Option<std::path::PathBuf> = {
            // helper to convert &str to PathBuf and check
            let try_dir = |s: &str| {
                let p = std::path::PathBuf::from(s);
                if p.exists() && p.is_dir() {
                    Some(p)
                } else {
                    None
                }
            };

            if let Some(dir) = working_dir {
                if let Some(valid) = try_dir(dir) {
                    Some(valid)
                } else {
                    None
                }
            } else if let Some(ctx_dir) = context.effective_working_directory() {
                if ctx_dir.exists() && ctx_dir.is_dir() {
                    Some(ctx_dir.to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
            .or_else(|| std::env::current_dir().ok())
        };

    if let Some(dir_path) = &effective_dir {
            // Ensure command runs in the effective directory so shell built-ins like
            // `copy` operate on the same relative paths that were used by file tasks.
            // Only set current_dir when the path exists and is a directory to avoid
            // ERROR_DIRECTORY issues on Windows.
            if dir_path.exists() && dir_path.is_dir() {
                // Try to canonicalize for safety, but fall back to the raw path
                let dir_to_set = std::fs::canonicalize(dir_path).unwrap_or_else(|_| dir_path.clone());
                match dir_to_set.to_str() {
                    Some(ds) => {
                        cmd.current_dir(ds);
                    }
                    None => {
                        // If the path cannot be converted to string, set using PathBuf
                        cmd.current_dir(dir_to_set);
                    }
                }
            } else {
                // This branch should not happen because effective_dir is only
                // Some when the path exists and is a directory, but keep the
                // message for completeness.
                println!("⚠️ Not setting current_dir because path is missing or not a dir: {}", dir_path.display());
            }

            println!("📁 Effective working directory: {}", dir_path.display());

            // List files in the effective directory for debugging
            match std::fs::read_dir(dir_path) {
                Ok(entries) => {
                    println!("📂 Listing cwd entries:");
                    for e in entries.flatten().take(20) {
                        if let Ok(meta) = e.metadata() {
                            println!(" - {} ({})", e.path().display(), meta.len());
                        } else {
                            println!(" - {}", e.path().display());
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️ Failed to list cwd {}: {}", dir_path.display(), e);
                }
            }
        }

        // General robustness: detect tokens in the command that look like file paths
        // (contain slashes/backslashes or a dot) and perform a short metadata retry
        // loop so that subsequent shell launches don't fail due to transient
        // filesystem races (common on Windows with OneDrive/antivirus). This is a
        // conservative heuristic that only waits briefly and logs diagnostics.
        let precheck_base = effective_dir
            .as_ref()
            .map(|p| p.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));

        let mut candidate_paths: Vec<std::path::PathBuf> = Vec::new();
        for raw_tok in command.split_whitespace() {
            // trim common quote characters
            let tok = raw_tok.trim_matches(|c| c == '\'' || c == '"' || c == ',');
            // simple heuristic: contains path separators or a dot (e.g. file.ext)
            if tok.contains('\\') || tok.contains('/') || tok.contains('.') {
                // skip tokens that look like redirection operators
                if tok == ">" || tok == ">>" || tok == "<" {
                    continue;
                }

                let p = std::path::Path::new(tok);
                let abs = if p.is_absolute() { p.to_path_buf() } else { precheck_base.join(p) };
                candidate_paths.push(abs);
            }
        }

        if !candidate_paths.is_empty() {
            println!("🔎 Pre-exec candidate paths to verify: {}", candidate_paths.len());
            for cp in &candidate_paths {
                let mut ok = false;
                // Increase attempts and backoff slightly to tolerate transient visibility delays
                for attempt in 1..=6 {
                    match tokio_fs::metadata(cp).await {
                        Ok(m) => {
                            // If it's a file or directory, consider it available
                            if m.is_file() || m.is_dir() {
                                ok = true;
                                break;
                            }
                        }
                        Err(e) => {
                            if attempt < 6 {
                                println!("⚠️ Precheck: '{}' missing (attempt {}): {}", cp.display(), attempt, e);
                                tokio::time::sleep(Duration::from_millis(25 * attempt)).await;
                                continue;
                            } else {
                                println!("⚠️ Precheck final: '{}' still missing: {}", cp.display(), e);
                            }
                        }
                    }
                }
                if !ok {
                    println!("⚠️ Proceeding despite missing precheck path: {}", cp.display());
                }
            }
        }

        // Heuristic and robustness: if using Windows cmd and the command starts
        // with 'copy ', try to perform the copy programmatically using async
        // tokio::fs::copy with a short retry loop. This avoids fragile cmd built-ins
        // that are sensitive to quoting and transient filesystem races. If the
        // programmatic copy fails, fall back to executing the original cmd.
        if shell_cmd == "cmd" || shell_cmd.ends_with("cmd.exe") {
            if command.trim_start().to_lowercase().starts_with("copy ") {
                // split on whitespace roughly (this will not fully respect
                // complex quoting, but it covers the simple test cases used in
                // integration tests)
                let parts: Vec<&str> = command.split_whitespace().collect();
                if parts.len() >= 3 {
                    let src = parts[1];
                    let dst = parts[2];

                    let base = effective_dir
                        .as_ref()
                        .map(|p| p.clone())
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));

                    let src_path = std::path::Path::new(src);
                    let dst_path = std::path::Path::new(dst);

                    let src_abs = if src_path.is_absolute() {
                        src_path.to_path_buf()
                    } else {
                        base.join(src_path)
                    };
                    let dst_abs = if dst_path.is_absolute() {
                        dst_path.to_path_buf()
                    } else {
                        base.join(dst_path)
                    };

                    println!(
                        "🔁 Detected cmd copy (programmatic attempt); resolved src -> {} dst -> {}",
                        src_abs.display(), dst_abs.display()
                    );

                    // Try to wait for the source to appear (short retry loop)
                    let mut src_ok = false;
                    for attempt in 1..=6 {
                        match tokio_fs::metadata(&src_abs).await {
                            Ok(_) => {
                                src_ok = true;
                                break;
                            }
                            Err(e) => {
                                if attempt < 6 {
                                    println!("⚠️ Source not present yet (attempt {}): {}", attempt, e);
                                    tokio::time::sleep(Duration::from_millis(25 * attempt)).await;
                                    continue;
                                } else {
                                    println!("❌ Source still missing after retries: {}", e);
                                }
                            }
                        }
                    }

                    if src_ok {
                        // Ensure destination parent exists
                        if let Some(parent) = dst_abs.parent() {
                            if !parent.exists() {
                                if let Err(e) = tokio_fs::create_dir_all(parent).await {
                                    println!("⚠️ Failed to create destination parent: {}", e);
                                } else {
                                    println!("📂 Created directory: {}", parent.display());
                                }
                            }
                        }

                        // Attempt programmatic copy
                        match tokio_fs::copy(&src_abs, &dst_abs).await {
                            Ok(bytes) => {
                                let duration = start_time.elapsed();
                                let completed_at = Utc::now();

                                println!("✅ Programmatic file copy succeeded: {} bytes", bytes);

                                let mut metadata = std::collections::HashMap::new();
                                metadata.insert("operation".to_string(), serde_json::json!("copy"));
                                metadata.insert("source".to_string(), serde_json::json!(src_abs.display().to_string()));
                                metadata.insert("destination".to_string(), serde_json::json!(dst_abs.display().to_string()));

                                return Ok(TaskResult {
                                    task_id: context.task_id.clone(),
                                    status: TaskStatus::Success,
                                    stdout: format!("Copied {bytes} bytes from {} to {}", src_abs.display(), dst_abs.display()),
                                    stderr: String::new(),
                                    exit_code: Some(0),
                                    duration,
                                    retry_count: 0,
                                    started_at,
                                    completed_at,
                                    output_data: Some(serde_json::json!({
                                        "operation": "copy",
                                        "bytes_copied": bytes,
                                        "source": src_abs.display().to_string(),
                                        "destination": dst_abs.display().to_string()
                                    })),
                                    error_message: None,
                                    metadata,
                                });
                            }
                            Err(e) => {
                                println!("⚠️ Programmatic copy failed: {}. Falling back to shell copy.", e);
                                // fall through to shell execution below
                            }
                        }
                    } else {
                        println!("⚠️ Source file not available for programmatic copy; falling back to shell copy.");
                        // fall through to shell execution below
                    }
                }
            }
        }

        // Before attempting to spawn the shell command, ensure we've normalized any
        // long-path prefixes in the current_dir (the cmd.current_dir call above used
        // a cleaned string). If the spawn fails with ERROR_DIRECTORY we'll retry
        // below without current_dir.

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
                let raw = io_error.raw_os_error();
                // If the spawn failed with ERROR_DIRECTORY (267) on Windows, a
                // common cause is that the current_dir passed to the child process
                // was not valid at the moment of spawn. Retry once without
                // setting current_dir to avoid that transient failure.
                if raw == Some(267) {
                    println!("⚠️ Spawn failed with ERROR_DIRECTORY (267). Retrying without current_dir...");

                    // Build a fresh command without current_dir and reuse env/stdio
                    let mut retry_cmd = Command::new(shell_cmd);
                    retry_cmd.args(&shell_args);

                    for (key, value) in context.env_vars_for_process() {
                        retry_cmd.env(key, value);
                    }
                    retry_cmd.stdout(Stdio::piped());
                    retry_cmd.stderr(Stdio::piped());
                    retry_cmd.stdin(Stdio::null());

                    match timeout(task_timeout, retry_cmd.output()).await {
                        Ok(Ok(output)) => {
                            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                            let exit_code = output.status.code();

                            let status = if output.status.success() {
                                TaskStatus::Success
                            } else {
                                TaskStatus::Failed
                            };

                            println!("✅ Shell command completed with exit code: {exit_code:?} (retry without cwd)");
                            if !stdout.is_empty() {
                                println!("📤 stdout: {}", stdout.trim());
                            }
                            if !stderr.is_empty() {
                                println!("📤 stderr: {}", stderr.trim());
                            }

                            return Ok(TaskResult {
                                task_id: context.task_id.clone(),
                                status,
                                stdout,
                                stderr,
                                exit_code,
                                duration,
                                retry_count: 1,
                                started_at,
                                completed_at,
                                output_data: None,
                                error_message: if status == TaskStatus::Failed {
                                    Some(format!("Command failed with exit code: {exit_code:?} (retry)"))
                                } else {
                                    None
                                },
                                metadata: std::collections::HashMap::new(),
                            });
                        }
                        Ok(Err(retry_err)) => {
                            let error_msg = format!("Failed to execute shell command on retry: {retry_err}");
                            println!("❌ {error_msg}");
                            return Err(ExecutorError::TaskExecutionFailed {
                                task_id: context.task_id.clone(),
                                source: anyhow::anyhow!(retry_err),
                            });
                        }
                        Err(_) => {
                            let error_msg = format!("Shell command timed out after {task_timeout:?} on retry");
                            println!("⏰ {error_msg}");
                            return Err(ExecutorError::TaskTimeout {
                                task_id: context.task_id.clone(),
                                timeout: task_timeout,
                            });
                        }
                    }
                }

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

    fn clone_box(&self) -> Box<dyn crate::executor::TaskExecutor> {
        Box::new(Self {
            default_timeout: self.default_timeout,
            default_shell: self.default_shell.clone(),
        })
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
