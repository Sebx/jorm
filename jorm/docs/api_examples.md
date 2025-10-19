# JORM Native Executor API Examples

This document provides comprehensive examples for using the JORM native executor API.

## Table of Contents

- [Basic Executor Usage](#basic-executor-usage)
- [Shell Task Executor](#shell-task-executor)
- [Python Task Executor](#python-task-executor)
- [HTTP Task Executor](#http-task-executor)
- [File Task Executor](#file-task-executor)
- [Configuration Examples](#configuration-examples)
- [Error Handling](#error-handling)
- [State Management](#state-management)
- [Resource Monitoring](#resource-monitoring)

## Basic Executor Usage

### Simple DAG Execution

```rust
use jorm::executor::{NativeExecutor, ExecutorConfig};
use jorm::parser::parse_dag_file;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create executor with default configuration
    let config = ExecutorConfig::default();
    let executor = NativeExecutor::new(config);

    // Parse and execute a DAG
    let dag = parse_dag_file("workflow.txt").await?;
    let result = executor.execute_dag(&dag).await?;

    // Print results
    println!("Execution Status: {:?}", result.status);
    println!("Total Tasks: {}", result.task_results.len());
    println!("Duration: {:?}", result.total_duration);

    // Print individual task results
    for (task_id, task_result) in &result.task_results {
        println!("Task {}: {:?}", task_id, task_result.status);
        if !task_result.stdout.is_empty() {
            println!("  Output: {}", task_result.stdout);
        }
        if !task_result.stderr.is_empty() {
            println!("  Error: {}", task_result.stderr);
        }
    }

    Ok(())
}
```

### Custom Configuration

```rust
use jorm::executor::{ExecutorConfig, ResourceLimits};
use std::time::Duration;

let config = ExecutorConfig {
    max_concurrent_tasks: 8,
    default_timeout: Duration::from_secs(600), // 10 minutes
    enable_state_persistence: true,
    enable_resource_throttling: true,
    resource_limits: Some(ResourceLimits {
        max_cpu_percent: 80.0,
        max_memory_percent: 70.0,
        max_concurrent_tasks: 6,
    }),
    ..Default::default()
};

let executor = NativeExecutor::new(config);
```

## Shell Task Executor

### Basic Shell Commands

```rust
use jorm::executor::{Task, TaskConfig, ShellTaskExecutor, ExecutionContext};

// Simple command
let task = Task::new(
    "hello".to_string(),
    "Hello World".to_string(),
    "shell".to_string(),
    TaskConfig::Shell {
        command: "echo 'Hello, World!'".to_string(),
        working_dir: None,
        shell: None,
    },
);

// With working directory
let task = Task::new(
    "list_files".to_string(),
    "List Files".to_string(),
    "shell".to_string(),
    TaskConfig::Shell {
        command: "ls -la".to_string(),
        working_dir: Some("/tmp".to_string()),
        shell: None,
    },
);

// With specific shell
let task = Task::new(
    "bash_script".to_string(),
    "Bash Script".to_string(),
    "shell".to_string(),
    TaskConfig::Shell {
        command: "#!/bin/bash\necho $SHELL\ndate".to_string(),
        working_dir: None,
        shell: Some("bash".to_string()),
    },
);
```

### Platform-Specific Examples

```rust
// Windows PowerShell
let windows_task = Task::new(
    "powershell_task".to_string(),
    "PowerShell Command".to_string(),
    "shell".to_string(),
    TaskConfig::Shell {
        command: "Get-Process | Where-Object {$_.CPU -gt 100}".to_string(),
        working_dir: None,
        shell: Some("powershell".to_string()),
    },
);

// Unix shell with environment variables
let unix_task = Task::new(
    "env_task".to_string(),
    "Environment Task".to_string(),
    "shell".to_string(),
    TaskConfig::Shell {
        command: "export MY_VAR=hello && echo $MY_VAR".to_string(),
        working_dir: None,
        shell: Some("bash".to_string()),
    },
);
```

### Custom Shell Executor

```rust
use jorm::executor::ShellTaskExecutor;
use std::time::Duration;

// Custom timeout
let executor = ShellTaskExecutor::with_timeout(Duration::from_secs(120));

// Custom shell
let executor = ShellTaskExecutor::with_shell("zsh".to_string());

// Execute task
let context = ExecutionContext::new("dag".to_string(), "task".to_string(), Default::default());
let result = executor.execute(&task, &context).await?;
```

## Python Task Executor

### Python Scripts

```rust
use jorm::executor::{Task, TaskConfig};

// Simple Python script
let task = Task::new(
    "python_hello".to_string(),
    "Python Hello".to_string(),
    "python_script".to_string(),
    TaskConfig::PythonScript {
        script: "print('Hello from Python!')".to_string(),
        args: vec![],
        python_path: None,
    },
);

// Python script with arguments
let task = Task::new(
    "data_analysis".to_string(),
    "Data Analysis".to_string(),
    "python_script".to_string(),
    TaskConfig::PythonScript {
        script: "analyze_data.py".to_string(),
        args: vec![
            "--input".to_string(),
            "data.csv".to_string(),
            "--output".to_string(),
            "results.json".to_string(),
            "--verbose".to_string(),
        ],
        python_path: Some("/usr/bin/python3".to_string()),
    },
);
```

### Python Functions

```rust
use std::collections::HashMap;

// Simple function call
let task = Task::new(
    "math_operation".to_string(),
    "Math Operation".to_string(),
    "python_function".to_string(),
    TaskConfig::PythonFunction {
        module: "math".to_string(),
        function: "sqrt".to_string(),
        args: vec![serde_json::Value::Number(serde_json::Number::from(16))],
        kwargs: HashMap::new(),
    },
);

// Function with keyword arguments
let mut kwargs = HashMap::new();
kwargs.insert("sep".to_string(), serde_json::Value::String(" | ".to_string()));
kwargs.insert("end".to_string(), serde_json::Value::String("\n\n".to_string()));

let task = Task::new(
    "print_data".to_string(),
    "Print Data".to_string(),
    "python_function".to_string(),
    TaskConfig::PythonFunction {
        module: "builtins".to_string(),
        function: "print".to_string(),
        args: vec![
            serde_json::Value::String("Hello".to_string()),
            serde_json::Value::String("World".to_string()),
        ],
        kwargs,
    },
);
```

### Custom Python Executor

```rust
use jorm::executor::PythonTaskExecutor;
use std::time::Duration;

// Custom Python path
let executor = PythonTaskExecutor::with_python_path("/opt/python/bin/python3".to_string());

// Custom timeout
let executor = PythonTaskExecutor::with_timeout(Duration::from_secs(300));
```

## HTTP Task Executor

### Basic HTTP Requests

```rust
use jorm::executor::{Task, TaskConfig, AuthConfig};
use std::collections::HashMap;

// Simple GET request
let task = Task::new(
    "api_status".to_string(),
    "Check API Status".to_string(),
    "http".to_string(),
    TaskConfig::HttpRequest {
        method: "GET".to_string(),
        url: "https://api.example.com/status".to_string(),
        headers: HashMap::new(),
        body: None,
        auth: None,
        timeout: None,
    },
);

// POST request with JSON body
let mut headers = HashMap::new();
headers.insert("Content-Type".to_string(), "application/json".to_string());
headers.insert("User-Agent".to_string(), "JORM/1.0".to_string());

let task = Task::new(
    "create_user".to_string(),
    "Create User".to_string(),
    "http".to_string(),
    TaskConfig::HttpRequest {
        method: "POST".to_string(),
        url: "https://api.example.com/users".to_string(),
        headers,
        body: Some(r#"{"name": "John Doe", "email": "john@example.com"}"#.to_string()),
        auth: None,
        timeout: Some(Duration::from_secs(30)),
    },
);
```

### Authentication Examples

```rust
// Bearer token authentication
let task = Task::new(
    "authenticated_request".to_string(),
    "Authenticated Request".to_string(),
    "http".to_string(),
    TaskConfig::HttpRequest {
        method: "GET".to_string(),
        url: "https://api.example.com/protected".to_string(),
        headers: HashMap::new(),
        body: None,
        auth: Some(AuthConfig::Bearer {
            token: "your-jwt-token-here".to_string(),
        }),
        timeout: None,
    },
);

// Basic authentication
let task = Task::new(
    "basic_auth_request".to_string(),
    "Basic Auth Request".to_string(),
    "http".to_string(),
    TaskConfig::HttpRequest {
        method: "GET".to_string(),
        url: "https://api.example.com/secure".to_string(),
        headers: HashMap::new(),
        body: None,
        auth: Some(AuthConfig::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        }),
        timeout: None,
    },
);

// API key authentication
let task = Task::new(
    "api_key_request".to_string(),
    "API Key Request".to_string(),
    "http".to_string(),
    TaskConfig::HttpRequest {
        method: "GET".to_string(),
        url: "https://api.example.com/data".to_string(),
        headers: HashMap::new(),
        body: None,
        auth: Some(AuthConfig::ApiKey {
            key: "your-api-key".to_string(),
            header: "X-API-Key".to_string(),
        }),
        timeout: None,
    },
);
```

### Custom HTTP Executor

```rust
use jorm::executor::HttpTaskExecutor;
use std::time::Duration;

// Custom timeout
let executor = HttpTaskExecutor::with_timeout(Duration::from_secs(60))?;

// Custom client configuration
let executor = HttpTaskExecutor::with_client_config(|builder| {
    builder
        .timeout(Duration::from_secs(30))
        .user_agent("MyApp/1.0")
        .danger_accept_invalid_certs(false)
})?;
```

## File Task Executor

### File Operations

```rust
use jorm::executor::{Task, TaskConfig};
use std::collections::HashMap;

// Copy file
let task = Task::new(
    "copy_file".to_string(),
    "Copy File".to_string(),
    "file".to_string(),
    TaskConfig::FileOperation {
        operation: "copy".to_string(),
        source: Some("/path/to/source.txt".to_string()),
        destination: Some("/path/to/destination.txt".to_string()),
        options: HashMap::new(),
    },
);

// Move file
let task = Task::new(
    "move_file".to_string(),
    "Move File".to_string(),
    "file".to_string(),
    TaskConfig::FileOperation {
        operation: "move".to_string(),
        source: Some("/path/to/source.txt".to_string()),
        destination: Some("/path/to/new_location.txt".to_string()),
        options: HashMap::new(),
    },
);

// Delete file
let task = Task::new(
    "delete_file".to_string(),
    "Delete File".to_string(),
    "file".to_string(),
    TaskConfig::FileOperation {
        operation: "delete".to_string(),
        source: Some("/path/to/file.txt".to_string()),
        destination: None,
        options: HashMap::new(),
    },
);

// Create file with content
let mut options = HashMap::new();
options.insert("content".to_string(), serde_json::Value::String("Hello, World!".to_string()));
options.insert("create_dirs".to_string(), serde_json::Value::Bool(true));

let task = Task::new(
    "create_file".to_string(),
    "Create File".to_string(),
    "file".to_string(),
    TaskConfig::FileOperation {
        operation: "create".to_string(),
        source: None,
        destination: Some("/path/to/new_file.txt".to_string()),
        options,
    },
);
```

## Configuration Examples

### Executor Configuration

```rust
use jorm::executor::{ExecutorConfig, ResourceLimits};
use std::time::Duration;

// Development configuration
let dev_config = ExecutorConfig {
    max_concurrent_tasks: 4,
    default_timeout: Duration::from_secs(300),
    enable_state_persistence: false,
    enable_resource_throttling: false,
    resource_limits: None,
    ..Default::default()
};

// Production configuration
let prod_config = ExecutorConfig {
    max_concurrent_tasks: 16,
    default_timeout: Duration::from_secs(1800), // 30 minutes
    enable_state_persistence: true,
    enable_resource_throttling: true,
    resource_limits: Some(ResourceLimits {
        max_cpu_percent: 85.0,
        max_memory_percent: 80.0,
        max_concurrent_tasks: 12,
    }),
    ..Default::default()
};

// High-performance configuration
let perf_config = ExecutorConfig {
    max_concurrent_tasks: 32,
    default_timeout: Duration::from_secs(3600), // 1 hour
    enable_state_persistence: true,
    enable_resource_throttling: true,
    resource_limits: Some(ResourceLimits {
        max_cpu_percent: 95.0,
        max_memory_percent: 90.0,
        max_concurrent_tasks: 24,
    }),
    ..Default::default()
};
```

### Task Configuration

```rust
use jorm::executor::{Task, TaskRetryConfig, BackoffStrategy};
use std::time::Duration;

// Task with retry configuration
let task = Task::new(
    "flaky_task".to_string(),
    "Flaky Task".to_string(),
    "http".to_string(),
    TaskConfig::HttpRequest {
        method: "GET".to_string(),
        url: "https://unreliable-api.com/data".to_string(),
        headers: HashMap::new(),
        body: None,
        auth: None,
        timeout: Some(Duration::from_secs(30)),
    },
)
.with_timeout(Duration::from_secs(60))
.with_retry_config(TaskRetryConfig {
    max_attempts: 3,
    initial_delay: Duration::from_secs(1),
    max_delay: Duration::from_secs(30),
    backoff_strategy: BackoffStrategy::Exponential { multiplier: 2.0 },
    retry_on_exit_codes: vec![1, 2, 130],
    retry_on_timeout: true,
});

// Task with environment variables
let task = Task::new(
    "env_task".to_string(),
    "Environment Task".to_string(),
    "shell".to_string(),
    TaskConfig::Shell {
        command: "echo $MY_VAR".to_string(),
        working_dir: None,
        shell: None,
    },
)
.with_env_var("MY_VAR".to_string(), "hello_world".to_string())
.with_env_var("DEBUG".to_string(), "true".to_string());
```

## Error Handling

### Comprehensive Error Handling

```rust
use jorm::executor::{ExecutorError, ExecutionStatus, TaskStatus};

async fn execute_with_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeExecutor::new(ExecutorConfig::default());
    let dag = parse_dag_file("workflow.txt").await?;

    match executor.execute_dag(&dag).await {
        Ok(result) => {
            match result.status {
                ExecutionStatus::Success => {
                    println!("✅ All tasks completed successfully");
                    
                    // Print task details
                    for (task_id, task_result) in &result.task_results {
                        println!("Task {}: completed in {:?}", task_id, task_result.duration);
                    }
                }
                ExecutionStatus::Failed => {
                    println!("❌ Execution failed");
                    
                    // Find and report failed tasks
                    let failed_tasks: Vec<_> = result.task_results.iter()
                        .filter(|(_, r)| r.status == TaskStatus::Failed)
                        .collect();
                    
                    for (task_id, task_result) in failed_tasks {
                        println!("Failed task {}: {}", task_id, 
                            task_result.error_message.as_deref().unwrap_or("Unknown error"));
                        
                        if !task_result.stderr.is_empty() {
                            println!("  stderr: {}", task_result.stderr);
                        }
                        
                        if let Some(exit_code) = task_result.exit_code {
                            println!("  exit code: {}", exit_code);
                        }
                    }
                }
                _ => println!("Execution status: {:?}", result.status),
            }
            
            // Print execution metrics
            println!("Execution metrics:");
            println!("  Total tasks: {}", result.metrics.total_tasks);
            println!("  Successful: {}", result.metrics.successful_tasks);
            println!("  Failed: {}", result.metrics.failed_tasks);
            println!("  Peak concurrency: {}", result.metrics.peak_concurrent_tasks);
            if let Some(duration) = result.total_duration {
                println!("  Total duration: {:?}", duration);
            }
        }
        Err(ExecutorError::TaskExecutionFailed { task_id, source }) => {
            eprintln!("Task '{}' execution failed: {}", task_id, source);
        }
        Err(ExecutorError::DependencyCycle) => {
            eprintln!("DAG contains dependency cycles - cannot execute");
        }
        Err(ExecutorError::TaskTimeout { task_id, timeout }) => {
            eprintln!("Task '{}' timed out after {:?}", task_id, timeout);
        }
        Err(ExecutorError::ResourceLimitExceeded { resource }) => {
            eprintln!("Resource limit exceeded: {}", resource);
        }
        Err(ExecutorError::ConfigurationError { message }) => {
            eprintln!("Configuration error: {}", message);
        }
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
        }
    }

    Ok(())
}
```

## State Management

### Enabling State Persistence

```rust
use jorm::executor::{NativeExecutor, ExecutorConfig, StateConfig};
use std::time::Duration;

async fn with_state_management() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExecutorConfig::default();
    let state_config = StateConfig {
        database_url: "sqlite:jorm_executions.db".to_string(),
        enable_checkpoints: true,
        checkpoint_interval: Duration::from_secs(30),
    };

    let executor = NativeExecutor::with_state_management(config, state_config).await?;
    
    // Execute DAG with state persistence
    let dag = parse_dag_file("long_running_workflow.txt").await?;
    let result = executor.execute_dag(&dag).await?;
    
    println!("Execution {} completed with state persistence", result.execution_id);
    Ok(())
}
```

### Execution Recovery

```rust
async fn resume_execution() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExecutorConfig::default();
    let state_config = StateConfig {
        database_url: "sqlite:jorm_executions.db".to_string(),
        enable_checkpoints: true,
        checkpoint_interval: Duration::from_secs(30),
    };

    let executor = NativeExecutor::with_state_management(config, state_config).await?;
    
    // List recoverable executions
    let recoverable = executor.list_recoverable_executions().await?;
    
    if let Some(execution) = recoverable.first() {
        println!("Resuming execution: {}", execution.execution_id);
        
        // Load the original DAG
        let dag = parse_dag_file("workflow.txt").await?;
        
        // Resume execution
        let result = executor.resume_execution(&execution.execution_id, &dag).await?;
        
        println!("Resumed execution completed: {:?}", result.status);
    } else {
        println!("No recoverable executions found");
    }
    
    Ok(())
}
```

## Resource Monitoring

### Enabling Resource Monitoring

```rust
use jorm::executor::{ExecutorConfig, ResourceLimits};

async fn with_resource_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExecutorConfig {
        enable_resource_throttling: true,
        resource_limits: Some(ResourceLimits {
            max_cpu_percent: 80.0,
            max_memory_percent: 70.0,
            max_concurrent_tasks: 8,
        }),
        ..Default::default()
    };

    let executor = NativeExecutor::new(config);
    
    // Start resource monitoring
    executor.start_resource_monitoring().await?;
    
    // Execute DAG with resource monitoring
    let dag = parse_dag_file("resource_intensive_workflow.txt").await?;
    let result = executor.execute_dag(&dag).await?;
    
    // Get final resource usage
    if let Some(usage) = executor.get_resource_usage().await {
        println!("Final resource usage:");
        println!("  CPU: {:.1}%", usage.cpu_percent);
        println!("  Memory: {:.1}%", usage.memory_percent);
        println!("  Running tasks: {}", usage.running_tasks);
    }
    
    // Stop resource monitoring
    executor.stop_resource_monitoring().await;
    
    Ok(())
}
```

### Custom Resource Limits

```rust
use jorm::executor::{ResourceLimits, ResourceMonitor};
use std::sync::Arc;

// Create custom resource limits
let limits = ResourceLimits {
    max_cpu_percent: 90.0,      // Allow up to 90% CPU usage
    max_memory_percent: 85.0,   // Allow up to 85% memory usage
    max_concurrent_tasks: 16,   // Maximum 16 concurrent tasks
};

// Create resource monitor
let monitor = Arc::new(ResourceMonitor::new(limits));

// Use with executor configuration
let config = ExecutorConfig {
    enable_resource_throttling: true,
    resource_limits: Some(limits),
    max_concurrent_tasks: 20, // Higher than resource limit for flexibility
    ..Default::default()
};
```

This completes the comprehensive API documentation and examples for the JORM native executor.
##
 Advanced Usage Examples

### Custom Task Registry

```rust
use jorm::executor::{TaskRegistry, NativeExecutor, ExecutorConfig};

// Create custom registry
let mut registry = TaskRegistry::new();

// Register custom executors
registry.register("custom_shell", Box::new(ShellTaskExecutor::with_shell("zsh".to_string())));
registry.register("custom_python", Box::new(PythonTaskExecutor::with_python_path("/opt/python/bin/python3".to_string())));

// Create executor with custom registry
let config = ExecutorConfig::default();
let executor = NativeExecutor::with_task_registry(config, registry);
```

### Environment Variable Management

```rust
use jorm::executor::{EnvironmentManager, EnvironmentConfig, InterpolationContext};
use std::collections::HashMap;

// Create environment configuration
let mut executor_env = HashMap::new();
executor_env.insert("APP_ENV".to_string(), "production".to_string());
executor_env.insert("LOG_LEVEL".to_string(), "info".to_string());

let env_config = EnvironmentConfig {
    variables: HashMap::from([
        ("DATABASE_URL".to_string(), "${DB_HOST}:${DB_PORT}/myapp".to_string()),
    ]),
    inherit_parent: false, // Don't inherit for security
    inherit_executor: true,
    exclude: vec!["TEMP".to_string(), "TMP".to_string()],
    enable_interpolation: true,
    secure_prefixes: vec!["PASSWORD".to_string(), "SECRET".to_string(), "TOKEN".to_string()],
};

let manager = EnvironmentManager::new(executor_env, env_config);

// Create interpolation context
let mut context = InterpolationContext::new();
context.add_variable("DB_HOST".to_string(), "localhost".to_string());
context.add_variable("DB_PORT".to_string(), "5432".to_string());

// Build task environment
let task_env = HashMap::from([
    ("TASK_ID".to_string(), "my_task".to_string()),
]);

let env_vars = manager.build_task_environment(&task_env, Some(&context))?;

// Log environment (secure values will be masked)
EnvironmentManager::log_environment(&env_vars, "my_task");
```

### Parallel Execution with Resource Monitoring

```rust
use jorm::executor::{NativeExecutor, ExecutorConfig, ResourceLimits};
use std::time::Duration;

async fn execute_with_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExecutorConfig {
        max_concurrent_tasks: 16,
        enable_resource_throttling: true,
        resource_limits: Some(ResourceLimits {
            max_cpu_percent: 85.0,
            max_memory_percent: 80.0,
            max_concurrent_tasks: 12,
            monitoring_interval: Duration::from_secs(5),
            throttle_threshold: 0.9,
        }),
        ..Default::default()
    };

    let executor = NativeExecutor::new(config);
    
    // Start resource monitoring
    executor.start_resource_monitoring().await?;
    
    // Execute DAG
    let dag = parse_dag_file("resource_intensive_workflow.txt").await?;
    let result = executor.execute_dag(&dag).await?;
    
    // Monitor resource usage during execution
    tokio::spawn(async move {
        loop {
            if let Some(usage) = executor.get_resource_usage().await {
                println!("CPU: {:.1}%, Memory: {:.1}%, Tasks: {}", 
                    usage.cpu_percent, usage.memory_percent, usage.running_tasks);
                
                if usage.cpu_percent > 90.0 {
                    println!("⚠️ High CPU usage detected!");
                }
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
    
    // Stop monitoring when done
    executor.stop_resource_monitoring().await;
    
    println!("Execution completed: {:?}", result.status);
    Ok(())
}
```

### Task Chaining with Output Passing

```rust
use jorm::executor::{Task, TaskConfig, EnvironmentManager, InterpolationContext};
use std::collections::HashMap;

// Create tasks that pass data between each other
let task1 = Task::new(
    "fetch_data".to_string(),
    "Fetch Data".to_string(),
    "http".to_string(),
    TaskConfig::HttpRequest {
        method: "GET".to_string(),
        url: "https://api.example.com/data".to_string(),
        headers: HashMap::new(),
        body: None,
        auth: None,
        timeout: None,
    },
);

let task2 = Task::new(
    "process_data".to_string(),
    "Process Data".to_string(),
    "python_script".to_string(),
    TaskConfig::PythonScript {
        script: "process.py".to_string(),
        args: vec!["--input".to_string(), "${task.fetch_data.stdout}".to_string()],
        python_path: None,
    },
).add_dependency("fetch_data".to_string());

let task3 = Task::new(
    "save_results".to_string(),
    "Save Results".to_string(),
    "shell".to_string(),
    TaskConfig::Shell {
        command: "echo '${task.process_data.stdout}' > results.txt".to_string(),
        working_dir: None,
        shell: None,
    },
).add_dependency("process_data".to_string());
```

### Retry Configuration Examples

```rust
use jorm::executor::{Task, TaskRetryConfig, BackoffStrategy};
use std::time::Duration;

// Aggressive retry for flaky network requests
let network_task = Task::new(
    "api_call".to_string(),
    "API Call".to_string(),
    "http".to_string(),
    TaskConfig::HttpRequest {
        method: "GET".to_string(),
        url: "https://unreliable-api.com/data".to_string(),
        headers: HashMap::new(),
        body: None,
        auth: None,
        timeout: Some(Duration::from_secs(30)),
    },
).with_retry_config(TaskRetryConfig {
    max_attempts: 5,
    initial_delay: Duration::from_secs(1),
    max_delay: Duration::from_secs(60),
    backoff_strategy: BackoffStrategy::Exponential { multiplier: 2.0 },
    retry_on_exit_codes: vec![500, 502, 503, 504],
    retry_on_timeout: true,
});

// Conservative retry for critical operations
let critical_task = Task::new(
    "deploy".to_string(),
    "Deploy Application".to_string(),
    "shell".to_string(),
    TaskConfig::Shell {
        command: "kubectl apply -f deployment.yaml".to_string(),
        working_dir: None,
        shell: None,
    },
).with_retry_config(TaskRetryConfig {
    max_attempts: 2, // Only one retry
    initial_delay: Duration::from_secs(10),
    max_delay: Duration::from_secs(10),
    backoff_strategy: BackoffStrategy::Fixed,
    retry_on_exit_codes: vec![1], // Only retry on general failure
    retry_on_timeout: false,
});
```

### Performance Benchmarking

```rust
use jorm::executor::{PerformanceBenchmark, MetricsCollector};

async fn benchmark_execution() -> Result<(), Box<dyn std::error::Error>> {
    let benchmark = PerformanceBenchmark::new("DAG Execution Benchmark".to_string());
    
    // Benchmark native executor
    let native_result = benchmark.run_benchmark(|| async {
        let executor = NativeExecutor::new(ExecutorConfig::default());
        let dag = parse_dag_file("benchmark_dag.txt").await?;
        executor.execute_dag(&dag).await
    }).await?;
    
    println!("Native Executor Benchmark:");
    println!("  Average time: {:?}", native_result.average_duration);
    println!("  Min time: {:?}", native_result.min_duration);
    println!("  Max time: {:?}", native_result.max_duration);
    println!("  Throughput: {:.2} tasks/sec", native_result.throughput);
    
    // Compare with baseline if available
    if let Some(baseline) = load_baseline_results()? {
        let comparison = benchmark.compare_with_baseline(&baseline)?;
        println!("Performance vs baseline: {:.1}% improvement", comparison.improvement_percent);
    }
    
    Ok(())
}
```

### Complex DAG with Mixed Task Types

```rust
use jorm::executor::{Task, TaskConfig, AuthConfig};
use std::collections::HashMap;

async fn create_complex_dag() -> Vec<Task> {
    let mut tasks = Vec::new();
    
    // 1. Setup task - create directories
    tasks.push(Task::new(
        "setup".to_string(),
        "Setup Environment".to_string(),
        "shell".to_string(),
        TaskConfig::Shell {
            command: "mkdir -p data logs temp".to_string(),
            working_dir: Some("/workspace".to_string()),
            shell: None,
        },
    ));
    
    // 2. Download data via HTTP
    tasks.push(Task::new(
        "download_data".to_string(),
        "Download Dataset".to_string(),
        "http".to_string(),
        TaskConfig::HttpRequest {
            method: "GET".to_string(),
            url: "https://data.example.com/dataset.csv".to_string(),
            headers: HashMap::from([
                ("User-Agent".to_string(), "JORM/1.0".to_string()),
            ]),
            body: None,
            auth: Some(AuthConfig::ApiKey {
                key: "${API_KEY}".to_string(),
                header: "X-API-Key".to_string(),
            }),
            timeout: Some(Duration::from_secs(300)),
        },
    ).add_dependency("setup".to_string()));
    
    // 3. Save downloaded data to file
    tasks.push(Task::new(
        "save_data".to_string(),
        "Save Data to File".to_string(),
        "file".to_string(),
        TaskConfig::FileOperation {
            operation: "create".to_string(),
            source: None,
            destination: Some("data/dataset.csv".to_string()),
            options: HashMap::from([
                ("content".to_string(), serde_json::json!("${task.download_data.stdout}")),
                ("create_dirs".to_string(), serde_json::json!(true)),
            ]),
        },
    ).add_dependency("download_data".to_string()));
    
    // 4. Process data with Python
    tasks.push(Task::new(
        "process_data".to_string(),
        "Process Dataset".to_string(),
        "python_script".to_string(),
        TaskConfig::PythonScript {
            script: "scripts/process_data.py".to_string(),
            args: vec![
                "--input".to_string(),
                "data/dataset.csv".to_string(),
                "--output".to_string(),
                "data/processed.json".to_string(),
                "--format".to_string(),
                "json".to_string(),
            ],
            python_path: Some("/opt/python/bin/python3".to_string()),
        },
    ).add_dependency("save_data".to_string()));
    
    // 5. Generate report
    tasks.push(Task::new(
        "generate_report".to_string(),
        "Generate Report".to_string(),
        "python_function".to_string(),
        TaskConfig::PythonFunction {
            module: "report_generator".to_string(),
            function: "create_report".to_string(),
            args: vec![
                serde_json::json!("data/processed.json"),
                serde_json::json!("reports/analysis.html"),
            ],
            kwargs: HashMap::from([
                ("template".to_string(), serde_json::json!("analysis_template.html")),
                ("include_charts".to_string(), serde_json::json!(true)),
            ]),
        },
    ).add_dependency("process_data".to_string()));
    
    // 6. Upload report
    tasks.push(Task::new(
        "upload_report".to_string(),
        "Upload Report".to_string(),
        "http".to_string(),
        TaskConfig::HttpRequest {
            method: "POST".to_string(),
            url: "https://reports.example.com/upload".to_string(),
            headers: HashMap::from([
                ("Content-Type".to_string(), "multipart/form-data".to_string()),
            ]),
            body: Some("@reports/analysis.html".to_string()),
            auth: Some(AuthConfig::Bearer {
                token: "${UPLOAD_TOKEN}".to_string(),
            }),
            timeout: Some(Duration::from_secs(120)),
        },
    ).add_dependency("generate_report".to_string()));
    
    // 7. Cleanup temporary files
    tasks.push(Task::new(
        "cleanup".to_string(),
        "Cleanup Temporary Files".to_string(),
        "file".to_string(),
        TaskConfig::FileOperation {
            operation: "delete".to_string(),
            source: Some("temp".to_string()),
            destination: None,
            options: HashMap::from([
                ("recursive".to_string(), serde_json::json!(true)),
            ]),
        },
    ).add_dependency("upload_report".to_string()));
    
    tasks
}
```

### Error Recovery and Resilience

```rust
use jorm::executor::{ExecutorError, ExecutionStatus, TaskStatus};

async fn resilient_execution() -> Result<(), Box<dyn std::error::Error>> {
    let executor = NativeExecutor::new(ExecutorConfig::default());
    let dag = parse_dag_file("critical_workflow.txt").await?;
    
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 3;
    
    loop {
        match executor.execute_dag(&dag).await {
            Ok(result) => {
                match result.status {
                    ExecutionStatus::Success => {
                        println!("✅ Execution completed successfully");
                        break;
                    }
                    ExecutionStatus::Failed => {
                        // Analyze failures and decide on recovery strategy
                        let failed_tasks: Vec<_> = result.task_results.iter()
                            .filter(|(_, r)| r.status == TaskStatus::Failed)
                            .collect();
                        
                        if failed_tasks.len() == 1 && retry_count < MAX_RETRIES {
                            let (task_id, task_result) = failed_tasks[0];
                            
                            // Check if it's a recoverable error
                            if is_recoverable_error(task_result) {
                                retry_count += 1;
                                println!("🔄 Retrying execution (attempt {}/{})", retry_count, MAX_RETRIES);
                                
                                // Wait before retry with exponential backoff
                                let delay = Duration::from_secs(2_u64.pow(retry_count));
                                tokio::time::sleep(delay).await;
                                continue;
                            }
                        }
                        
                        println!("❌ Execution failed with {} failed tasks", failed_tasks.len());
                        for (task_id, task_result) in failed_tasks {
                            println!("  - {}: {}", task_id, 
                                task_result.error_message.as_deref().unwrap_or("Unknown error"));
                        }
                        break;
                    }
                    _ => {
                        println!("⚠️ Execution ended with status: {:?}", result.status);
                        break;
                    }
                }
            }
            Err(ExecutorError::ResourceLimitExceeded { resource }) => {
                println!("⚠️ Resource limit exceeded: {}. Reducing concurrency and retrying.", resource);
                
                // Create new executor with reduced concurrency
                let reduced_config = ExecutorConfig {
                    max_concurrent_tasks: executor.config().max_concurrent_tasks / 2,
                    ..executor.config().clone()
                };
                let reduced_executor = NativeExecutor::new(reduced_config);
                
                match reduced_executor.execute_dag(&dag).await {
                    Ok(result) => {
                        println!("✅ Execution completed with reduced concurrency");
                        break;
                    }
                    Err(e) => {
                        println!("❌ Execution failed even with reduced concurrency: {}", e);
                        return Err(e.into());
                    }
                }
            }
            Err(ExecutorError::RecoveryError { execution_id, .. }) => {
                println!("🔄 Attempting to resume execution: {}", execution_id);
                
                // Try to resume from checkpoint
                match executor.resume_execution(&execution_id, &dag).await {
                    Ok(result) => {
                        println!("✅ Successfully resumed execution");
                        break;
                    }
                    Err(e) => {
                        println!("❌ Failed to resume execution: {}", e);
                        return Err(e.into());
                    }
                }
            }
            Err(e) => {
                println!("❌ Unrecoverable error: {}", e);
                return Err(e.into());
            }
        }
    }
    
    Ok(())
}

fn is_recoverable_error(task_result: &TaskResult) -> bool {
    // Define recoverable error conditions
    if let Some(exit_code) = task_result.exit_code {
        match exit_code {
            1 | 2 => true,  // General errors that might be transient
            130 => true,    // Interrupted (Ctrl+C)
            _ => false,
        }
    } else {
        // No exit code might indicate timeout or system error
        task_result.stderr.contains("timeout") || 
        task_result.stderr.contains("connection") ||
        task_result.stderr.contains("network")
    }
}
```

This completes the comprehensive API documentation for the JORM native executor, covering all major components, configuration options, and usage patterns.