# JORM Documentation

Welcome to the JORM documentation. This directory contains guides and references for using the lightweight DAG execution engine.

## Documentation Overview

### 📚 Core Documentation

| Document | Description | Audience |
|----------|-------------|----------|
| [API Examples](api_examples.md) | Code examples and usage patterns | Developers |
| [Troubleshooting Guide](troubleshooting_guide.md) | Common issues and solutions | All Users |

### 🚀 Quick Start

New to JORM? Start here:

1. **Installation**: `cargo install jorm`
2. **Basic Usage**: Check out [API Examples](api_examples.md#basic-usage)
3. **First DAG**: Create and run your first DAG

### 🎯 Use Case Guides

#### For Developers
- [API Examples](api_examples.md) - Learn how to use JORM programmatically
- [Task Types](api_examples.md#task-types) - Understand different task types
- [Error Handling](api_examples.md#error-handling) - Implement basic error handling

#### For System Administrators
- [Troubleshooting Guide](troubleshooting_guide.md) - Resolve common issues
- [HTTP Server](api_examples.md#http-server) - Set up webhook endpoints
- [Scheduling](api_examples.md#scheduling) - Configure cron-based scheduling

## Key Features

### 🚀 Core Functionality
- **Simple DAG Execution**: Execute workflows with dependency resolution
- **Multiple Task Types**: Shell, Python, Rust, HTTP, and file operations
- **Natural Language Processing**: Generate DAGs from descriptions
- **Lightweight Design**: Minimal dependencies and fast startup

### 🔧 Additional Features
- **HTTP Server**: Webhook endpoints for remote execution
- **Basic Scheduling**: Cron-based scheduling with daemon mode
- **Simple Syntax**: Easy-to-learn text-based DAG format
- **Error Handling**: Clear error messages and basic retry logic

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    🦀 JORM DAG Engine                       │
├─────────────────────────────────────────────────────────────┤
│  CLI Interface             │  HTTP Server    │  Scheduler   │
│  ├─ Command processing     │  ├─ Webhooks    │  ├─ Cron     │
│  ├─ Argument validation    │  ├─ Auth        │  ├─ Daemon   │
│  └─ Output formatting      │  └─ Logging     │  └─ Jobs     │
├─────────────────────────────────────────────────────────────┤
│  Core Engine                                                │
│  ├─ Natural Language Processor (Lightweight AI)            │
│  ├─ DAG Parser (Text-based format)                         │
│  ├─ Task Executor (Shell, Python, Rust, HTTP, File)       │
│  └─ Dependency Resolution                                   │
└─────────────────────────────────────────────────────────────┘
```

## Getting Started Examples

### Basic DAG Execution

```bash
# Execute a DAG from file
jorm execute my-workflow.txt

# Generate DAG from natural language
jorm generate "Copy files and run tests"

# Execute from natural language directly
jorm execute-nl "Build project and deploy"

# Start HTTP server
jorm server --port 8080

# Schedule a DAG
jorm schedule my-workflow.txt "0 2 * * *"
```

### Simple DAG Example

```text
# hello-world.txt
task greeting {
    type: shell
    command: "echo 'Hello from JORM!'"
}

task calculation {
    type: python
    script: "calc.py"
    args: ["2", "2"]
    depends_on: [greeting]
}

task status_check {
    type: http
    method: "GET"
    url: "https://httpbin.org/status/200"
    depends_on: [greeting]
}

task log_result {
    type: file_copy
    source: "input.txt"
    destination: "results.txt"
    depends_on: [calculation, status_check]
}
```

## Natural Language Examples

### Generate DAGs from Descriptions

```bash
# Simple workflow
jorm generate "Copy config files, build project, run tests"

# Data processing pipeline
jorm generate "Download CSV data, process with Python, backup results"

# Deployment workflow
jorm generate "Build Docker image, push to registry, deploy to staging"
```

### Generated DAG Preview

```text
# Generated from: "Copy config files, build project, run tests"
task copy_config {
    type: file_copy
    source: "config.template"
    destination: "config.prod"
}

task build_project {
    type: rust
    command: "cargo build --release"
    depends_on: [copy_config]
}

task run_tests {
    type: rust
    command: "cargo test"
    depends_on: [build_project]
}
```

## Common Use Cases

### CI/CD Pipelines
```text
task checkout {
    type: shell
    command: "git clone https://github.com/user/repo.git"
}

task install {
    type: shell
    command: "npm install"
    working_dir: "./repo"
    depends_on: [checkout]
}

task test {
    type: shell
    command: "npm test"
    working_dir: "./repo"
    depends_on: [install]
}

task build {
    type: shell
    command: "npm run build"
    working_dir: "./repo"
    depends_on: [test]
}

task deploy {
    type: shell
    command: "./deploy.sh"
    working_dir: "./repo"
    depends_on: [build]
}
```

### Data Processing
```text
task extract {
    type: python
    script: "extract_data.py"
    args: ["--source", "database"]
}

task transform {
    type: python
    script: "transform_data.py"
    args: ["--input", "raw_data.csv"]
    depends_on: [extract]
}

task validate {
    type: python
    script: "validate_data.py"
    args: ["--input", "processed_data.csv"]
    depends_on: [transform]
}

task load {
    type: python
    script: "load_data.py"
    args: ["--target", "warehouse"]
    depends_on: [validate]
}
```

## Best Practices

### DAG Design
1. **Simple Dependencies**: Keep dependency chains straightforward
2. **Task Naming**: Use descriptive names for tasks
3. **Working Directories**: Specify working directories for clarity
4. **Error Messages**: Include meaningful error context

### Security
1. **File Permissions**: Secure DAG files and working directories
2. **Network Security**: Use HTTPS for HTTP tasks
3. **Authentication**: Use authentication tokens for HTTP server
4. **Path Validation**: Validate file paths in file operations

### Reliability
1. **Task Validation**: Validate DAG syntax before execution
2. **Error Handling**: Handle task failures gracefully
3. **Logging**: Monitor execution logs for issues
4. **Testing**: Test DAGs in development before production

## Support and Community

### Getting Help
- **Documentation**: Start with this documentation
- **GitHub Issues**: [Report bugs and request features](https://github.com/jorm-project/jorm/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jorm-project/jorm/discussions)

### Contributing
- **Source Code**: [GitHub Repository](https://github.com/jorm-project/jorm)
- **Documentation**: Submit PRs to improve documentation
- **Bug Reports**: Use the issue template for bug reports
- **Feature Requests**: Discuss new features in GitHub Discussions

## Changelog and Releases

### Latest Release: v0.1.0
- ✨ Lightweight DAG execution engine
- 🤖 Natural language processing for DAG generation
- 🌐 HTTP server for webhook triggers
- ⏰ Basic cron scheduling with daemon mode
- 📝 Simple text-based DAG syntax
- 🔧 Multiple task types (shell, Python, Rust, HTTP, file operations)

## License

JORM is released under the [MIT License](../LICENSE). See the license file for details.

---

**Need help?** Check the [Troubleshooting Guide](troubleshooting_guide.md) or reach out to the community!