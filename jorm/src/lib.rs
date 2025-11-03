// Jorm - Simplified DAG Execution Engine
// Core library exports

pub mod core;
pub mod parser;
pub mod executor;
pub mod nlp;
pub mod server;
pub mod scheduler;

// Re-export main types for convenience
pub use core::{engine::JormEngine, dag::Dag, task::{Task, TaskType}};
pub use parser::dag_parser::DagParser;
pub use executor::TaskExecutor;
pub use nlp::generator::{NlpProcessor, DagPreview, DagEdit};

// Error types
pub use core::error::JormError;

// Result type alias
pub type Result<T> = std::result::Result<T, JormError>;