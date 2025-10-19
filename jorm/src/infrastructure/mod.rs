/// Infrastructure layer - Ports and Adapters
/// 
/// This module implements the hexagonal architecture pattern, separating
/// the core business logic from infrastructure concerns through ports
/// (interfaces) and adapters (implementations).

pub mod ports;
pub mod adapters;
pub mod errors;

pub use ports::*;
pub use adapters::*;
pub use errors::*;
