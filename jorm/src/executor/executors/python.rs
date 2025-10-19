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

        // Create temporary Python file (unique per execution to avoid collisions).
        // Instead of relying on process CWD or task working dir (which may change concurrently),
        // Generate a safe temporary filename using a hash of the execution and task IDs
        let exec_hash = format!("{:x}", md5::compute(&context.execution_id));
        let task_hash = format!("{:x}", md5::compute(&context.task_id));
        let exec_id = exec_hash.chars().take(8).collect::<String>();
        let temp_file = format!("py_{}_{}_{}.py", 
            exec_id,
            task_hash.chars().take(8).collect::<String>(),
            std::process::id()
        );

        let base_temp = std::env::temp_dir();
        let exec_dir = base_temp.join(format!("jorm_exec_{}", exec_id));
        if let Err(e) = std::fs::create_dir_all(&exec_dir) {
            return Err(ExecutorError::IoError {
                message: format!("Failed to create execution temp dir {}: {e}", exec_dir.display()),
                source: e,
            });
        }

        let script_path = exec_dir.join(&temp_file);

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

        // Diagnostic: check that the file exists and print metadata immediately
        match std::fs::metadata(&script_path) {
            Ok(meta) => match meta.modified() {
                Ok(m) => println!("✅ Script written. path='{}' size={} bytes modified={:?}", script_path.display(), meta.len(), m),
                Err(_) => println!("✅ Script written. path='{}' size={} bytes (modified time unavailable)", script_path.display(), meta.len()),
            },
            Err(e) => println!("⚠️  Failed to stat written script '{}': {e}", script_path.display()),
        }

        println!("🐍 Executing Python script: {}", script_path.display());

        // Extra diagnostics: canonicalize path, try opening the file from this process,
        // and list parent directory contents to detect external interference (antivirus/OneDrive)
        match std::fs::canonicalize(&script_path) {
            Ok(canon) => println!("🔗 Canonical path: {}", canon.display()),
            Err(e) => println!("⚠️  Failed to canonicalize '{}': {e}", script_path.display()),
        }

        match std::fs::File::open(&script_path) {
            Ok(mut f) => {
                use std::io::Read;
                let mut buf = [0u8; 64];
                match f.read(&mut buf) {
                    Ok(n) => println!("📖 Read {} bytes from script (preview): {}", n, String::from_utf8_lossy(&buf[..n])),
                    Err(e) => println!("⚠️  Failed to read from script '{}': {e}", script_path.display()),
                }
            }
            Err(e) => println!("⚠️  Failed to open script for reading '{}': {e}", script_path.display()),
        }

        if let Some(parent) = script_path.parent() {
            match std::fs::read_dir(parent) {
                Ok(rd) => {
                    println!("📂 Listing parent directory '{}' entries:", parent.display());
                    for entry in rd.flatten().take(50) {
                        if let Ok(md) = entry.metadata() {
                            println!(" - {} ({} bytes)", entry.path().display(), md.len());
                        } else {
                            println!(" - {} (metadata error)", entry.path().display());
                        }
                    }
                }
                Err(e) => println!("⚠️  Failed to read parent dir '{}': {e}", parent.display()),
            }
        }

        // Small delay / visibility wait to give the OS a moment to flush and
        // ensure the script file is visible to child processes (helps on Windows).
        for attempt in 1..=6 {
            match std::fs::metadata(&script_path) {
                Ok(_) => break,
                Err(e) => {
                    if attempt < 6 {
                        println!("⚠️ Waiting for script visibility (attempt {}): {}", attempt, e);
                        tokio::time::sleep(Duration::from_millis(25 * attempt)).await;
                        continue;
                    } else {
                        println!("⚠️ Script still not visible after retries: {}", e);
                    }
                }
            }
        }

        // Create the command
        let mut cmd = Command::new(&self.python_path);

        // On Windows some long-paths include the "\\?\\" prefix; Python/launcher
        // may not accept that form when passed as argv. Canonicalize and strip it.
        let script_arg = match std::fs::canonicalize(&script_path) {
            Ok(p) => {
                let s = p.to_string_lossy().to_string();
                if s.starts_with(r"\\?\\") {
                    s.trim_start_matches(r"\\?\\").to_string()
                } else {
                    s
                }
            }
            Err(_) => script_path.to_string_lossy().to_string(),
        };

        // short pause already handled by visibility wait above
        cmd.arg(script_arg);
        cmd.args(args);

        // Set UTF-8 environment for Windows compatibility
        cmd.env("PYTHONIOENCODING", "utf-8");
        cmd.env("PYTHONUTF8", "1");

        // Set working directory if specified, but only if it exists and is a directory.
        // Avoid calling current_dir on invalid paths (on Windows this causes os error 267).
        if let Some(dir) = working_dir {
            let dir_path = Path::new(dir);
            if dir_path.exists() && dir_path.is_dir() {
                match std::fs::canonicalize(dir_path) {
                    Ok(canon) => {
                        cmd.current_dir(&canon);
                        println!("📁 Working directory (canonicalized): {}", canon.display());
                    }
                    Err(_) => {
                        // Fall back to using the provided path if canonicalization fails
                        cmd.current_dir(dir_path);
                        println!("📁 Working directory: {} (canonicalization failed)", dir_path.display());
                    }
                }
            } else {
                println!("⚠️  Skipping setting working directory '{}': does not exist or is not a directory", dir_path.display());
            }
        }

        // Set environment variables from context
        for (key, value) in context.env_vars_for_process() {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // Execute with timeout and handle ERROR_DIRECTORY
        let execution_result = match timeout(task_timeout, cmd.output()).await {
            Ok(result) => {
                match result {
                    Ok(output) => Ok(output),
                    Err(io_error) => {
                        let raw = io_error.raw_os_error();
                        // If spawn failed with ERROR_DIRECTORY (267) on Windows, retry without current_dir
                        if raw == Some(267) {
                            println!("⚠️ Python spawn failed with ERROR_DIRECTORY (267). Retrying without current_dir...");
                            
                            // Build a fresh command without current_dir and reuse env/stdio
                            let mut retry_cmd = Command::new(&self.python_path);
                            retry_cmd.arg(&script_path);
                            retry_cmd.args(args);
                            retry_cmd.env("PYTHONIOENCODING", "utf-8");
                            retry_cmd.env("PYTHONUTF8", "1");

                            // Re-apply context environment but skip current_dir
                            for (key, value) in context.env_vars_for_process() {
                                retry_cmd.env(key, value);
                            }

                            retry_cmd.stdout(Stdio::piped());
                            retry_cmd.stderr(Stdio::piped());
                            retry_cmd.stdin(Stdio::null());

                            // Execute retry with timeout
                            match timeout(task_timeout, retry_cmd.output()).await {
                                Ok(Ok(output)) => Ok(output),
                                Ok(Err(e)) => Err(e),
                                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
                            }
                        } else {
                            Err(io_error)
                        }
                    }
                }
            },
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
        };

        let duration = start_time.elapsed();
        let completed_at = Utc::now();

        // Decide whether to keep scripts for debugging via env var
        let keep_scripts = std::env::var("JORM_KEEP_PY_SCRIPTS").is_ok();

        // Collect final result first, then do cleanup/logging so we can inspect the written file if needed
        let final_result: Result<TaskResult, ExecutorError> = match execution_result {
            Ok(output) => {
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
            Err(e) => {
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
        };

        // Cleanup: remove the temporary script unless debug variable is set
        if keep_scripts {
            println!("🛑 Keeping temporary Python script for debugging: {}", script_path.display());
        } else {
            match std::fs::remove_file(&script_path) {
                Ok(_) => println!("🧹 Removed temporary script: {}", script_path.display()),
                Err(e) => println!("⚠️  Failed to remove temporary script '{}': {e}", script_path.display()),
            }
        }

        final_result
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
            } => {
                self.execute_python_function(
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
                module,
                function,
                args: _,
                kwargs: _,
            } => {
                if module.is_empty() {
                    return Err(ExecutorError::ConfigurationError {
                        message: "Python module name cannot be empty".to_string(),
                    });
                }
                if function.is_empty() {
                    return Err(ExecutorError::ConfigurationError {
                        message: "Python function name cannot be empty".to_string(),
                    });
                }
                Ok(())
            }
            _ => Err(ExecutorError::UnsupportedTaskType {
                task_type: format!("Expected Python task, got: {:?}", task.config),
            }),
        }
    }

    fn clone_box(&self) -> Box<dyn crate::executor::TaskExecutor> {
        Box::new(Self {
            python_path: self.python_path.clone(),
            default_timeout: self.default_timeout,
        })
    }
}

impl Default for PythonTaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}