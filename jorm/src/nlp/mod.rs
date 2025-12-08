pub mod generator;

// Tier 3: Transformer-based NLP modules
#[cfg(feature = "tier3")]
pub mod config;
#[cfg(feature = "tier3")]
pub mod errors;
#[cfg(feature = "tier3")]
pub mod transformer_engine;
#[cfg(feature = "tier3")]
pub mod model_manager;
#[cfg(feature = "tier3")]
pub mod intent_classifier;
#[cfg(feature = "tier3")]
pub mod entity_extractor;
#[cfg(feature = "tier3")]
pub mod semantic_matcher;
#[cfg(feature = "tier3")]
pub mod context_analyzer;
#[cfg(feature = "tier3")]
pub mod semantic_types;

#[cfg(test)]
mod tests;
