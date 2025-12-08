//! Error types for Tier 3 transformer-based NLP
//!
//! This module defines error types specific to the Tier 3 NLP system.

#[cfg(feature = "tier3")]
use thiserror::Error;

#[cfg(feature = "tier3")]
use crate::core::error::JormError;

/// Errors that can occur in Tier 3 NLP processing
#[cfg(feature = "tier3")]
#[derive(Debug, Error)]
pub enum Tier3Error {
    /// Model file not found at the specified path
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    /// Model is not loaded when it should be
    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),
    
    /// Failed to load the transformer model
    #[error("Model loading failed: {0}")]
    ModelLoadError(String),
    
    /// Failed to tokenize input text
    #[error("Tokenization failed: {0}")]
    TokenizationError(String),
    
    /// Failed to run inference on the model
    #[error("Inference failed: {0}")]
    InferenceError(String),
    
    /// Failed to classify intent
    #[error("Intent classification failed: {0}")]
    IntentClassificationError(String),
    
    /// Failed to extract entities
    #[error("Entity extraction failed: {0}")]
    EntityExtractionError(String),
    
    /// Inference took too long and exceeded the timeout
    #[error("Inference timeout: exceeded {0}ms")]
    InferenceTimeout(u64),
    
    /// Confidence score is below the required threshold
    #[error("Insufficient confidence: {0}")]
    LowConfidence(f32),
    
    /// Configuration is invalid
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
    
    /// Device not available (e.g., CUDA requested but not available)
    #[error("Device not available: {0}")]
    DeviceNotAvailable(String),
    
    /// Semantic matching failed
    #[error("Semantic matching failed: {0}")]
    SemanticMatchError(String),
    
    /// Context analysis failed
    #[error("Context analysis failed: {0}")]
    ContextAnalysisError(String),
    
    /// Task generation failed
    #[error("Task generation failed: {0}")]
    TaskGenerationError(String),
    
    /// IO error occurred
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

#[cfg(feature = "tier3")]
impl From<Tier3Error> for JormError {
    fn from(error: Tier3Error) -> Self {
        match error {
            Tier3Error::ModelNotFound(msg) 
            | Tier3Error::ModelNotLoaded(msg)
            | Tier3Error::ModelLoadError(msg)
            | Tier3Error::TokenizationError(msg)
            | Tier3Error::InferenceError(msg)
            | Tier3Error::IntentClassificationError(msg)
            | Tier3Error::EntityExtractionError(msg)
            | Tier3Error::ConfigError(msg)
            | Tier3Error::DeviceNotAvailable(msg)
            | Tier3Error::SemanticMatchError(msg)
            | Tier3Error::ContextAnalysisError(msg)
            | Tier3Error::TaskGenerationError(msg) => {
                JormError::NlpError(format!("Tier 3 NLP error: {}", msg))
            }
            Tier3Error::InferenceTimeout(ms) => {
                JormError::NlpError(format!("Tier 3 inference timeout: exceeded {}ms", ms))
            }
            Tier3Error::LowConfidence(confidence) => {
                JormError::NlpError(format!("Tier 3 low confidence: {}", confidence))
            }
            Tier3Error::IoError(e) => JormError::IoError(e),
            Tier3Error::JsonError(e) => JormError::JsonError(e),
        }
    }
}

#[cfg(all(test, feature = "tier3"))]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let error = Tier3Error::ModelNotFound("test.model".to_string());
        assert_eq!(error.to_string(), "Model not found: test.model");
        
        let error = Tier3Error::InferenceTimeout(500);
        assert_eq!(error.to_string(), "Inference timeout: exceeded 500ms");
        
        let error = Tier3Error::LowConfidence(0.5);
        assert_eq!(error.to_string(), "Insufficient confidence: 0.5");
    }
    
    #[test]
    fn test_error_conversion_to_jorm_error() {
        let tier3_error = Tier3Error::ModelLoadError("Failed to load".to_string());
        let jorm_error: JormError = tier3_error.into();
        
        match jorm_error {
            JormError::NlpError(msg) => {
                assert!(msg.contains("Tier 3 NLP error"));
                assert!(msg.contains("Failed to load"));
            }
            _ => panic!("Expected NlpError variant"),
        }
    }
    
    #[test]
    fn test_timeout_error_conversion() {
        let tier3_error = Tier3Error::InferenceTimeout(1000);
        let jorm_error: JormError = tier3_error.into();
        
        match jorm_error {
            JormError::NlpError(msg) => {
                assert!(msg.contains("timeout"));
                assert!(msg.contains("1000ms"));
            }
            _ => panic!("Expected NlpError variant"),
        }
    }
    
    #[test]
    fn test_low_confidence_error_conversion() {
        let tier3_error = Tier3Error::LowConfidence(0.3);
        let jorm_error: JormError = tier3_error.into();
        
        match jorm_error {
            JormError::NlpError(msg) => {
                assert!(msg.contains("low confidence"));
                assert!(msg.contains("0.3"));
            }
            _ => panic!("Expected NlpError variant"),
        }
    }
}
