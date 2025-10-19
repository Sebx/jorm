#![allow(clippy::all)]
//! JORM - Pure Rust DAG Engine
//!
//! A high-performance, pure Rust implementation of a Directed Acyclic Graph (DAG) execution engine.
//! This library provides the core functionality for parsing, validating, and executing DAGs.

pub mod ai;
pub mod application;
pub mod domain;
pub mod executor;
pub mod infrastructure;
pub mod observability;
pub mod parser;
pub mod scheduler;
pub mod shebang;

// Re-export commonly used types for easier access
pub use executor::{ExecutionStatus, ExecutorConfig, NativeExecutor, TaskStatus};
pub use parser::{parse_dag_file, validate_dag, Dag};
pub use scheduler::{CronScheduler, Schedule, ScheduledJob, SchedulerConfig};
