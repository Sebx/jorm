/// Parser adapters - Implementations of DagParser port

pub mod filesystem;
pub mod memory;

pub use filesystem::FileSystemDagParser;
pub use memory::InMemoryDagParser;
