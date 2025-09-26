//! Task executor implementations
//!
//! This module contains concrete implementations of the TaskExecutor trait
//! for different task types (shell, HTTP, file operations).

pub mod file;
pub mod http;
pub mod python;
pub mod shell;

pub use file::*;
pub use http::*;
pub use python::*;
pub use shell::*;
