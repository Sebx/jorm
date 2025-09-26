//! Task executor implementations
//! 
//! This module contains concrete implementations of the TaskExecutor trait
//! for different task types (shell, HTTP, file operations).

pub mod shell;
pub mod http;
pub mod file;
pub mod python;

pub use shell::*;
pub use http::*;
pub use file::*;
pub use python::*;
