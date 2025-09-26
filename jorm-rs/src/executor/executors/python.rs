//! Python task executor implementation

use crate::executor::{ExecutionContext, ExecutorError, Task, TaskConfig, TaskResult, TaskStatus};
use async_trait::async_trait;
use chrono::Utc;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

/// Python task executor for running Python scripts and functions
pub struct PythonTaskExecutor {
    /// Default timeout for Python tasks
    default_timeout: Duration,

    /// Python executable path
    python_path: String,
}

impl PythonTaskExecutor {
    /// Create a new Python task executor
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(300), // 5 minutes
            python_path: Self::find_python_executable(),
        }
    }

    /// Create a new Python task executor with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            default_timeout: timeout,
            python_path: Self::find_python_executable(),
        }
    }

    /// Create a new Python task executor with custom Python path
    pub fn with_python_path(python_path: String) -> Self {
        Self {
            default_timeout: Duration::from_secs(300),
            python_path,
        }
    }

    /// Find Python executable on the system
    fn find_python_executable() -> String {
        // Platform-specific Python executable names
        let candidates = if cfg!(target_os = "windows") {
            ["python", "py", "python3"]
        } else {
            ["python3", "python", "py"]
        };

        for candidate in &candidates {
            if std::process::Command::new(candidate)
                .arg("--version")
                .output()
                .is_ok()
            {
                return candidate.to_string();
            }
        }

        // Default fallback
        "python".to_string()
    }

    /// Execute a Python script
    async fn execute_python_script(
        &self,
        script: &str,
        args: &[String],
        working_dir: Option<&str>,
        context: &ExecutionContext,
        task_timeout: Duration,
    ) -> Result<TaskResult, ExecutorError> {
        let start_time = Instant::now();
        let started_at = Utc::now();

        // Create temporary Python file
        let temp_file = format!("temp_script_{}.py", context.task_id);
        let script_path = if let Some(dir) = working_dir {
            Path::new(dir).join(&temp_file)
        } else {
            Path::new(&temp_file).to_path_buf()
        };

        // Write script to temporary file
        println!("🔍 Writing Python script to: {}", script_path.display());
        println!("🔍 Script content:");
        for (i, line) in script.lines().enumerate() {
            println!("{:2}: '{}'", i + 1, line);
        }

        if let Err(e) = std::fs::write(&script_path, script) {
            return Err(ExecutorError::IoError {
                message: format!("Failed to write Python script: {e}"),
                source: e,
            });
        }

        println!("🐍 Executing Python script: {}", script_path.display());

        // Create the command
        let mut cmd = Command::new(&self.python_path);
        cmd.arg(&script_path);
        cmd.args(args);

        // Set UTF-8 environment for Windows compatibility
        cmd.env("PYTHONIOENCODING", "utf-8");
        cmd.env("PYTHONUTF8", "1");

        // Set working directory if specified
        if let Some(dir) = working_dir {
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

        // Clean up temporary file
        let _ = std::fs::remove_file(&script_path);

        match execution_result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(0);

                let status = if exit_code == 0 {
                    TaskStatus::Success
                } else {
                    TaskStatus::Failed
                };

                println!("🐍 Python script completed with exit code: {exit_code}");
                if !stdout.is_empty() {
                    println!("📤 Output: {stdout}");
                }
                if !stderr.is_empty() {
                    println!("⚠️  Errors: {stderr}");
                }

                Ok(TaskResult {
                    task_id: context.task_id.clone(),
                    status,
                    started_at,
                    completed_at,
                    duration,
                    stdout,
                    stderr,
                    exit_code: Some(exit_code),
                    retry_count: 0,
                    output_data: None,
                    error_message: if status == TaskStatus::Failed {
                        Some(format!("Python script failed with exit code {exit_code}"))
                    } else {
                        None
                    },
                    metadata: std::collections::HashMap::new(),
                })
            }
            Ok(Err(e)) => {
                let error_msg = format!("Python execution failed: {e}");
                println!("❌ {error_msg}");

                Err(ExecutorError::TaskExecutionFailed {
                    task_id: context.task_id.clone(),
                    source: e.into(),
                })
            }
            Err(_timeout_error) => {
                let error_msg = format!("Python script timed out after {task_timeout:?}");
                println!("⏰ {error_msg}");

                Err(ExecutorError::TaskTimeout {
                    task_id: context.task_id.clone(),
                    timeout: task_timeout,
                })
            }
        }
    }

    /// Execute a Python function
    async fn execute_python_function(
        &self,
        module: &str,
        function: &str,
        args: &[serde_json::Value],
        kwargs: &std::collections::HashMap<String, serde_json::Value>,
        working_dir: Option<&str>,
        context: &ExecutionContext,
        task_timeout: Duration,
    ) -> Result<TaskResult, ExecutorError> {
        let start_time = Instant::now();
        let started_at = Utc::now();

        // Create Python code to call the function
        let mut python_code = format!("import sys\nimport json\nimport {module}\n\n");

        // Add argument handling
        if !args.is_empty() {
            python_code.push_str(&format!(
                "args = {}\n",
                serde_json::to_string(args).unwrap_or_else(|_| "[]".to_string())
            ));
        }

        if !kwargs.is_empty() {
            python_code.push_str(&format!(
                "kwargs = {}\n",
                serde_json::to_string(kwargs).unwrap_or_else(|_| "{}".to_string())
            ));
        }

        // Add function call
        if !args.is_empty() && !kwargs.is_empty() {
            python_code.push_str(&format!(
                "result = {module}.{function}(*args, **kwargs)\n"
            ));
        } else if !args.is_empty() {
            python_code.push_str(&format!("result = {module}.{function}(*args)\n"));
        } else if !kwargs.is_empty() {
            python_code.push_str(&format!("result = {module}.{function}(**kwargs)\n"));
        } else {
            python_code.push_str(&format!("result = {module}.{function}()\n"));
        }

        python_code.push_str("print(json.dumps(result, default=str))\n");

        println!("🐍 Executing Python function: {module}.{function}");

        // Execute as a script
        self.execute_python_script(&python_code, &[], working_dir, context, task_timeout)
            .await
    }
}

#[async_trait]
impl crate::executor::TaskExecutor for PythonTaskExecutor {
    async fn execute(
        &self,
        task: &Task,
        context: &ExecutionContext,
    ) -> Result<TaskResult, ExecutorError> {
        // Determine timeout
        let task_timeout = task.effective_timeout(self.default_timeout);

        // Extract Python configuration from task
        match &task.config {
            TaskConfig::PythonScript {
                script,
                args,
                python_path,
            } => {
                let python_exec = python_path.as_ref().unwrap_or(&self.python_path);
                let executor = Self::with_python_path(python_exec.clone());
                executor
                    .execute_python_script(
                        script,
                        args,
                        context
                            .effective_working_directory()
                            .map(|p| p.to_str().unwrap_or(".")),
                        context,
                        task_timeout,
                    )
                    .await
            }
            TaskConfig::PythonFunction {
                module,
                function,
                args,
                kwargs,
                python_path,
            } => {
                let python_exec = python_path.as_ref().unwrap_or(&self.python_path);
                let executor = Self::with_python_path(python_exec.clone());
                executor
                    .execute_python_function(
                        module,
                        function,
                        args,
                        kwargs,
                        context
                            .effective_working_directory()
                            .map(|p| p.to_str().unwrap_or(".")),
                        context,
                        task_timeout,
                    )
                    .await
            }
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: format!("Expected Python task, got: {:?}", task.config),
            }),
        }
    }

    fn task_type(&self) -> &'static str {
        "python"
    }

    fn supports_parallel(&self) -> bool {
        true
    }

    fn default_timeout(&self) -> Duration {
        self.default_timeout
    }

    fn validate_task(&self, task: &Task) -> Result<(), ExecutorError> {
        match &task.config {
            TaskConfig::PythonScript { script, .. } => {
                if script.is_empty() {
                    return Err(ExecutorError::ConfigurationError {
                        message: "Python script cannot be empty".to_string(),
                    });
                }
                Ok(())
            }
            TaskConfig::PythonFunction {
                module, function, ..
            } => {
                if module.is_empty() || function.is_empty() {
                    return Err(ExecutorError::ConfigurationError {
                        message: "Python module and function cannot be empty".to_string(),
                    });
                }
                Ok(())
            }
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: format!("Expected Python task, got: {:?}", task.config),
            }),
        }
    }
}
