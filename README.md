# 🦀 JORM - Job Orchestration and Resource Management

A high-performance DAG (Directed Acyclic Graph) execution engine built with Rust, featuring clean architecture and comprehensive testing.

## ✨ Features

- **🏗️ Modular Architecture**: Clean separation of concerns with core, executor, parser, and scheduler modules
- **🧪 38 Tests**: Comprehensive test coverage across unit and integration tests
- **📊 Multiple Executors**: Shell, Python, Rust, HTTP, File, and Jorm task executors
- **🤖 AI Integration**: Natural language DAG generation with OpenAI
- **⚡ High Performance**: Async/await throughout, optimized execution
- **� STcheduling**: Cron-based scheduling with daemon support

## 🚀 Quick Start

```bash
# Build and test
cd jorm
cargo build --release
cargo test

# Run CLI
cargo run -- --help
cargo run -- run path/to/my_dag.txt
cargo run -- validate path/to/my_dag.txt
cargo run -- generate "Create a data pipeline"
cargo run -- analyze path/to/my_dag.txt

# Configure logging
export RUST_LOG=info
export JORM_LOG_JSON=1  # JSON logging for production
```

## 📖 Architecture

```
┌─────────────────────────────────────────┐
│      CLI & HTTP Server                  │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│      Core Engine                        │
│  • JormEngine • DAG • Task              │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│      Modules                            │
│  • Parser • Executor • Scheduler • NLP  │
└─────────────────────────────────────────┘
```

## 🏗️ Usage Example

```rust
use jorm::JormEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create engine
    let engine = JormEngine::new().await?;
    
    // Execute DAG from file
    let result = engine.execute_from_file("workflow.txt").await?;
    println!("Execution completed: {:?}", result);
    
    // Generate DAG from natural language
    let dag_content = engine.generate_dag_from_nl("Build project and run tests").await?;
    println!("Generated DAG:\n{}", dag_content);
    
    Ok(())
}
```

## 📊 Metrics

- **Tests**: 38 (21 unit + 7 integration + 10 unit tests)
- **Modules**: 5 core modules (core, executor, parser, scheduler, nlp)
- **Executors**: 6 different task executors supported
- **Performance**: Fast startup, async execution

## 🔧 Development

```bash
# Code quality checks
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo audit

# Run specific test suites
cargo test unit
cargo test integration
cargo test --lib
```

### Project Structure
```
jorm/
├── src/
│   ├── core/                # Core DAG and task logic
│   ├── executor/            # Task executors (shell, python, etc.)
│   ├── parser/              # DAG file parsing
│   ├── scheduler/           # Cron scheduling and daemon
│   ├── server/              # HTTP server
│   ├── nlp/                 # Natural language processing
│   └── main.rs              # CLI entry point
└── tests/                   # Integration and unit tests
```

## 📚 Documentation

- Examples in `jorm/examples/` directory
- API documentation: `cargo doc --open`
- Test files demonstrate usage patterns

## 📄 License

MIT License - see LICENSE file for details

---

**Status**: ✅ Active Development | **Tests**: ✅ 38 passing | **Modules**: ✅ 5 core modules

**Built with ❤️ using Rust**