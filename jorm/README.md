# JORM - DAG Execution Engine

[![Crates.io](https://img.shields.io/crates/v/jorm.svg)](https://crates.io/crates/jorm)
[![Documentation](https://docs.rs/jorm/badge.svg)](https://docs.rs/jorm)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/sebx/jorm#license)

JORM is a fast, reliable DAG (Directed Acyclic Graph) execution engine built in Rust. It provides high-performance parallel task execution with comprehensive error handling, state management, and resource monitoring.

## Features

- **High-Performance Execution**: Native Rust implementation with true parallel task execution
- **Comprehensive Task Support**: Shell commands, Python scripts, HTTP requests, and file operations
- **Advanced Environment Management**: Secure variable handling with interpolation and task context
- **Task Output Chaining**: Pass data between tasks with parameter validation and type safety
- **Resource Monitoring**: CPU and memory usage tracking with intelligent throttling
- **State Persistence**: Execution recovery and checkpointing capabilities
- **Flexible Scheduling**: Cron-based scheduling with multiple trigger types
- **AI Integration**: Natural language DAG generation and analysis (Phase 3)

## Quick Start

### Installation

```bash
cargo install jorm
```

### Basic Usage

Create a simple DAG file (`hello.txt`):

```
# Simple Hello World DAG
task hello_world {
    type: shell
    command: "echo 'Hello, World!'"
}

task goodbye {
    type: shell
    command: "echo 'Goodbye!'"
    depends_on: [hello_world]
}
```

Execute the DAG:

```bash
jorm execute hello.txt
```

### Library Usage

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

    println!("Execution completed with status: {:?}", result.status);
    Ok(())
}
```

## Task Types

### Shell Tasks
```
task build {
    type: shell
    command: "cargo build --release"
    working_dir: "./my-project"
}
```

### Python Tasks
```
task analyze {
    type: python
    script: "analyze_data.py"
    args: ["--input", "data.csv"]
}
```

### HTTP Tasks
```
task api_call {
    type: http
    method: "POST"
    url: "https://api.example.com/webhook"
    headers: {
        "Content-Type": "application/json"
    }
    data: {"status": "completed"}
}
```

### File Operations
```
task backup {
    type: file
    operation: "copy"
    source: "./important-data"
    destination: "./backup/important-data"
}
```

## Advanced Features

### Environment Variable Management

```
task deploy {
    type: shell
    command: "deploy.sh ${ENVIRONMENT} ${VERSION}"
    environment: {
        ENVIRONMENT: "production"
        VERSION: "${task.build.stdout}"
        API_KEY: "${secure.api_key}"
    }
}
```

### Task Output Chaining

```
task process_data {
    type: python
    script: "process.py"
    args: ["${task.extract.stdout}"]
}

task notify {
    type: http
    method: "POST"
    url: "https://hooks.slack.com/webhook"
    data: {
        "message": "Processed ${task.process_data.data.record_count} records"
    }
}
```

### Resource Monitoring

```rust
use jorm::executor::{ExecutorConfig, ResourceLimits};

let config = ExecutorConfig {
    enable_resource_throttling: true,
    resource_limits: Some(ResourceLimits {
        max_cpu_percent: 80.0,
        max_memory_percent: 70.0,
        max_concurrent_tasks: 8,
    }),
    ..Default::default()
};
```

## Configuration

JORM can be configured through:

- Command-line arguments
- Configuration files (TOML, YAML, JSON)
- Environment variables
- Programmatic configuration

Example configuration file (`jorm.toml`):

```toml
[executor]
max_concurrent_tasks = 10
default_timeout = "5m"
enable_resource_throttling = true

[executor.resource_limits]
max_cpu_percent = 80.0
max_memory_percent = 70.0

[logging]
level = "info"
format = "json"

[scheduler]
enable_cron = true
timezone = "UTC"
```

## Performance

JORM is designed for high-performance execution:

- **Parallel Execution**: Independent tasks run concurrently
- **Resource-Aware**: Intelligent task scheduling based on system resources
- **Minimal Overhead**: Native Rust implementation with zero-cost abstractions
- **Efficient I/O**: Async I/O throughout the execution pipeline

Benchmarks show JORM can execute complex DAGs with hundreds of tasks efficiently while maintaining low memory usage and high throughput.

## State Management and Recovery

JORM provides robust state management:

```rust
use jorm::executor::{NativeExecutor, StateConfig};

let state_config = StateConfig {
    database_url: "sqlite:jorm_state.db".to_string(),
    enable_checkpoints: true,
    checkpoint_interval: Duration::from_secs(30),
};

let executor = NativeExecutor::with_state_management(config, state_config).await?;

// Resume interrupted execution
let result = executor.resume_execution("execution_id", &dag).await?;
```

## CLI Commands

- `jorm execute <dag_file>` - Execute a DAG
- `jorm validate <dag_file>` - Validate DAG syntax
- `jorm schedule <dag_file> <cron_expr>` - Schedule DAG execution
- `jorm status` - Show execution status
- `jorm resume <execution_id>` - Resume interrupted execution
- `jorm interactive` - Start interactive mode with AI assistance

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Roadmap

- [x] **Phase 1**: Core DAG execution engine
- [x] **Phase 2**: Native Rust executor with parallel execution
- [x] **Phase 3**: Advanced features (environment management, output chaining)
- [ ] **Phase 4**: Enhanced AI integration and natural language processing
- [ ] **Phase 5**: Distributed execution and cloud integration

## Support

- [Documentation](https://docs.rs/jorm)
- [GitHub Issues](https://github.com/sebx/jorm/issues)
- [Discussions](https://github.com/sebx/jorm/discussions)