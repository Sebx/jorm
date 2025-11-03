# JORM - Lightweight DAG Execution Engine

[![Crates.io](https://img.shields.io/crates/v/jorm.svg)](https://crates.io/crates/jorm)
[![Documentation](https://docs.rs/jorm/badge.svg)](https://docs.rs/jorm)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/jorm-project/jorm#license)

JORM is a lightweight, simple DAG (Directed Acyclic Graph) execution engine built in Rust. It focuses on core functionality with minimal dependencies, making it easy to use for workflow automation with powerful meta-capabilities.

## Features

- **Simple DAG Execution**: Execute workflows with shell commands, file operations, HTTP requests, Python scripts, and Rust commands
- **Natural Language Processing**: Generate DAGs from natural language descriptions using lightweight regex-based parsing
- **Meta-Capabilities**: JORM can recursively call itself to create, validate, and execute other workflows
- **Basic HTTP Server**: Trigger DAG execution via webhook endpoints
- **Simple Scheduling**: Cron-based scheduling for automated execution
- **Minimal Dependencies**: Lightweight design with only essential dependencies
- **Easy Syntax**: Simple text-based DAG format that's easy to learn and write

## Quick Start

### Installation

```bash
cargo install jorm
```

### Basic Usage

Create a simple DAG file (`workflow.txt`):

```
# Simple workflow example
task hello_world {
    type: shell
    command: "echo 'Hello, World!'"
}

task copy_file {
    type: file_copy
    source: "input.txt"
    destination: "output.txt"
    depends_on: [hello_world]
}

task notify {
    type: http
    method: "POST"
    url: "https://api.example.com/webhook"
    body: '{"status": "completed"}'
    depends_on: [copy_file]
}
```

Execute the DAG:

```bash
jorm execute --file workflow.txt
```

### Generate DAG from Natural Language

```bash
jorm generate --description "Copy file1.txt to backup folder, then run tests and send notification"
```

### Library Usage

```rust
use jorm::core::JormEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = JormEngine::new();
    
    // Execute from file
    let result = engine.execute_from_file("workflow.txt").await?;
    println!("Execution completed: {:?}", result);
    
    // Generate from natural language
    let dag_content = engine.generate_dag_from_nl("Build project and run tests").await?;
    println!("Generated DAG:\n{}", dag_content);
    
    Ok(())
}
```

## Task Types

JORM supports 7 different task types for comprehensive workflow automation:

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
    working_dir: "./scripts"
}
```

### Rust Tasks
```
task test {
    type: rust
    command: "cargo test"
    working_dir: "./my-project"
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
    body: '{"status": "completed"}'
}
```

### File Operations
```
task copy_backup {
    type: file_copy
    source: "./important-data"
    destination: "./backup/important-data"
}

task move_file {
    type: file_move
    source: "./temp/file.txt"
    destination: "./processed/file.txt"
}

task cleanup {
    type: file_delete
    path: "./temp/old-file.txt"
}
```

### Jorm Meta Tasks
```
task create_workflow {
    type: jorm
    command: "generate"
    args: ["--description", "copy files and run tests"]
}

task run_existing_dag {
    type: jorm
    command: "execute"
    args: ["--file", "other_workflow.txt"]
    depends_on: [create_workflow]
}

task validate_dag {
    type: jorm
    command: "validate"
    args: ["--file", "workflow.txt"]
}

task start_server {
    type: jorm
    command: "server"
    args: ["--port", "8080"]
}
```

## Natural Language Processing

Generate DAGs from natural language descriptions:

```bash
# Generate a DAG from description
jorm generate --description "Copy config files, build the project, run tests, and deploy to staging"

# Preview generated DAG before execution
jorm generate --preview --description "Download data, process with Python script, and send results via email"

# Execute directly from natural language
jorm generate --description "Clean up old logs and restart the service"
```

## Meta-Capabilities with Jorm Tasks

JORM can recursively call itself using the `jorm` task type, enabling powerful meta-workflows:

```
# Meta-workflow that creates and executes other workflows
task generate_build_workflow {
    type: jorm
    command: "generate"
    args: ["--description", "build project and run tests"]
}

task execute_generated_workflow {
    type: jorm
    command: "execute"
    args: ["--file", "generated_workflow.txt"]
    depends_on: [generate_build_workflow]
}

task validate_all_workflows {
    type: jorm
    command: "validate"
    args: ["--file", "*.txt"]
    depends_on: [execute_generated_workflow]
}
```

This enables workflows that can:
- **Create new DAGs** from natural language descriptions
- **Execute existing DAG files** with full dependency management
- **Validate workflow syntax** across multiple files
- **Start HTTP servers** as part of larger automation pipelines
- **Schedule other workflows** programmatically
- **Chain these operations** with complex dependencies

### Real-World Meta-Workflow Example

```
# Automated workflow management system
task generate_daily_reports {
    type: jorm
    command: "generate"
    args: ["--description", "download sales data, process with Python, generate PDF report"]
}

task execute_daily_reports {
    type: jorm
    command: "execute"
    args: ["--file", "daily_reports.txt"]
    depends_on: [generate_daily_reports]
}

task validate_all_workflows {
    type: jorm
    command: "validate"
    args: ["--file", "*.txt"]
    depends_on: [execute_daily_reports]
}

task start_monitoring_server {
    type: jorm
    command: "server"
    args: ["--port", "9090"]
    depends_on: [validate_all_workflows]
}
```

## HTTP Server

Start the HTTP server for webhook triggers:

```bash
# Start server on default port (8080)
jorm server

# Start server on custom port
jorm server --port 3000
```

API endpoints:

```bash
# Execute DAG from file
curl -X POST http://localhost:8080/execute \
  -H "Content-Type: application/json" \
  -d '{"dag_file": "workflow.txt"}'

# Execute from natural language
curl -X POST http://localhost:8080/execute/nl \
  -H "Content-Type: application/json" \
  -d '{"description": "Build project and run tests"}'

# Check execution status
curl http://localhost:8080/status/execution_id
```

## Scheduling

Schedule DAGs to run automatically using cron expressions:

```bash
# Schedule a DAG to run every day at 2 AM
jorm schedule workflow.txt "0 2 * * *"

# Start the scheduler daemon
jorm daemon start

# Stop the scheduler daemon
jorm daemon stop

# Check daemon status
jorm daemon status
```

## DAG Syntax

JORM uses a simple text-based format for defining workflows:

```
# Comments start with #
task task_name {
    type: shell
    command: "echo hello"
    working_dir: "/tmp"  # optional
}

task http_request {
    type: http
    method: "POST"
    url: "https://api.example.com/webhook"
    headers: {
        "Content-Type": "application/json"
    }
    body: '{"status": "completed"}'
    depends_on: [task_name]
}

task python_script {
    type: python
    script: "process_data.py"
    args: ["--input", "data.csv"]
    working_dir: "./scripts"
    depends_on: [http_request]
}
```

### Dependency Declaration

Tasks can depend on other tasks using the `depends_on` field:

```
task final_task {
    type: shell
    command: "echo 'All done!'"
    depends_on: [task1, task2, task3]
}
```

## CLI Commands

- `jorm execute --file <dag_file>` - Execute a DAG from file
- `jorm generate --description <description>` - Generate and execute DAG from natural language
- `jorm generate --preview --description <description>` - Preview generated DAG without execution
- `jorm validate --file <dag_file>` - Validate DAG syntax
- `jorm server [--port <port>]` - Start HTTP server for webhooks
- `jorm schedule <dag_file> <cron_expr>` - Schedule DAG execution
- `jorm daemon <start|stop|status>` - Manage scheduler daemon

## Examples

### Simple Build and Test Workflow

```
task build {
    type: rust
    command: "cargo build --release"
    working_dir: "./my-project"
}

task test {
    type: rust
    command: "cargo test"
    working_dir: "./my-project"
    depends_on: [build]
}

task notify_success {
    type: http
    method: "POST"
    url: "https://hooks.slack.com/webhook"
    body: '{"text": "Build and tests completed successfully!"}'
    depends_on: [test]
}
```

### Data Processing Pipeline

```
task download_data {
    type: shell
    command: "wget https://example.com/data.csv -O data.csv"
}

task process_data {
    type: python
    script: "process.py"
    args: ["data.csv", "processed_data.json"]
    depends_on: [download_data]
}

task backup_results {
    type: file_copy
    source: "processed_data.json"
    destination: "./backup/processed_data.json"
    depends_on: [process_data]
}

task cleanup {
    type: file_delete
    path: "data.csv"
    depends_on: [backup_results]
}
```

### Meta-Workflow: Dynamic Pipeline Generation

```
# Workflow that creates and manages other workflows
task create_test_suite {
    type: jorm
    command: "generate"
    args: ["--description", "run unit tests, integration tests, and generate coverage report"]
}

task execute_test_suite {
    type: jorm
    command: "execute"
    args: ["--file", "test_suite.txt"]
    depends_on: [create_test_suite]
}

task create_deployment_pipeline {
    type: jorm
    command: "generate"
    args: ["--description", "build Docker image, push to registry, deploy to staging"]
    depends_on: [execute_test_suite]
}

task execute_deployment {
    type: jorm
    command: "execute"
    args: ["--file", "deployment_pipeline.txt"]
    depends_on: [create_deployment_pipeline]
}

task validate_all_pipelines {
    type: jorm
    command: "validate"
    args: ["--file", "*.txt"]
    depends_on: [execute_deployment]
}
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Quick Reference

### Task Types Summary
| Type | Purpose | Example Use Case |
|------|---------|------------------|
| `shell` | Execute shell commands | Build scripts, system commands |
| `python` | Run Python scripts | Data processing, ML workflows |
| `rust` | Execute Rust/Cargo commands | Build, test, run Rust projects |
| `http` | Make HTTP requests | API calls, webhooks, notifications |
| `file_copy` | Copy files/directories | Backup, deployment preparation |
| `file_move` | Move files/directories | File organization, cleanup |
| `file_delete` | Delete files/directories | Cleanup, temporary file removal |
| `jorm` | Execute JORM commands | Meta-workflows, recursive execution |

### Natural Language Examples
- `"copy config files and run tests"` → File copy + Rust test tasks
- `"10 bash tasks that print numbers"` → 10 sequential shell tasks
- `"build project, run tests, deploy"` → Multi-step CI/CD pipeline
- `"create new dag and execute it"` → Meta-workflow with jorm tasks

## Support

- [Documentation](https://docs.rs/jorm)
- [GitHub Issues](https://github.com/jorm-project/jorm/issues)
- [Discussions](https://github.com/jorm-project/jorm/discussions)