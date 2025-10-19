/// Domain layer - Core business entities and logic
/// 
/// This module contains the pure business logic of JORM, independent of
/// infrastructure concerns. It follows Domain-Driven Design principles.

pub mod entities;
pub mod value_objects;
pub mod services;
pub mod repositories;
pub mod errors;

pub use entities::*;
pub use value_objects::*;
pub use services::*;
pub use repositories::*;
pub use errors::*;
