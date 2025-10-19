/// Adapter implementations - Concrete infrastructure implementations
/// 
/// Adapters implement the port interfaces for specific technologies
/// (file system, SQLite, HTTP, etc.)

pub mod parsers;
pub mod storage;
pub mod executors;

pub use parsers::*;
pub use storage::*;
pub use executors::*;
