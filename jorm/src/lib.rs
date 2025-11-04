// Jorm - Simplified DAG Execution Engine
// Core library exports

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::module_inception)]

pub mod core;
pub mod executor;
pub mod nlp;
pub mod parser;
pub mod scheduler;
pub mod server;

// Re-export main types for convenience
pub use core::{
    dag::Dag,
    engine::JormEngine,
    task::{Task, TaskType},
};
pub use executor::TaskExecutor;
pub use nlp::generator::{DagEdit, DagPreview, NlpProcessor};
pub use parser::dag_parser::DagParser;

// Error types
pub use core::error::JormError;

// Result type alias
pub type Result<T> = std::result::Result<T, JormError>;
