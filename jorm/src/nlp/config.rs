//! Configuration for Tier 3 transformer-based NLP
//!
//! This module defines configuration structures and validation for the Tier 3 NLP system.

#[cfg(feature = "tier3")]
use std::path::PathBuf;

#[cfg(feature = "tier3")]
use serde::{Deserialize, Serialize};

/// Configuration for Tier 3 transformer-based NLP
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Whether Tier 3 is enabled
    pub enabled: bool,
    
    /// Path to the transformer model
    pub model_path: PathBuf,
    
    /// Type of model to use
    pub model_type: ModelType,
    
    /// Device to run inference on
    pub device: DeviceType,
    
    /// Minimum confidence threshold for intent classification
    pub intent_threshold: f32,
    
    /// Minimum confidence threshold for entity extraction
    pub entity_threshold: f32,
    
    /// Minimum similarity threshold for semantic matching
    pub similarity_threshold: f32,
    
    /// Maximum time allowed for inference in milliseconds
    pub max_inference_time_ms: u64,
    
    /// Maximum total time for NLP processing in milliseconds
    pub max_total_time_ms: u64,
    
    /// Whether to fall back to Tier 2 on failure
    pub fallback_to_tier2: bool,
}

/// Type of transformer model
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    /// MiniLM-L6-v2 (22MB, fast, good accuracy)
    MiniLM,
    
    /// DistilBERT (66MB, slower, better accuracy)
    DistilBERT,
    
    /// BERT-tiny (17MB, fastest, acceptable accuracy)
    BERTTiny,
    
    /// Custom model with path
    Custom(String),
}

/// Device type for inference
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    /// CPU-only inference
    CPU,
    
    /// CUDA GPU acceleration
    CUDA,
    
    /// Metal GPU acceleration (macOS)
    Metal,
}

#[cfg(feature = "tier3")]
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            model_path: PathBuf::from("models/minilm-l6-v2"),
            model_type: ModelType::MiniLM,
            device: DeviceType::CPU,
            intent_threshold: 0.7,
            entity_threshold: 0.6,
            similarity_threshold: 0.8,
            max_inference_time_ms: 200,
            max_total_time_ms: 500,
            fallback_to_tier2: true,
        }
    }
}

#[cfg(feature = "tier3")]
impl Config {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate thresholds are in valid range [0.0, 1.0]
        if self.intent_threshold < 0.0 || self.intent_threshold > 1.0 {
            return Err(format!(
                "intent_threshold must be between 0.0 and 1.0, got {}",
                self.intent_threshold
            ));
        }
        
        if self.entity_threshold < 0.0 || self.entity_threshold > 1.0 {
            return Err(format!(
                "entity_threshold must be between 0.0 and 1.0, got {}",
                self.entity_threshold
            ));
        }
        
        if self.similarity_threshold < 0.0 || self.similarity_threshold > 1.0 {
            return Err(format!(
                "similarity_threshold must be between 0.0 and 1.0, got {}",
                self.similarity_threshold
            ));
        }
        
        // Validate time limits are reasonable
        if self.max_inference_time_ms == 0 {
            return Err("max_inference_time_ms must be greater than 0".to_string());
        }
        
        if self.max_total_time_ms == 0 {
            return Err("max_total_time_ms must be greater than 0".to_string());
        }
        
        if self.max_inference_time_ms > self.max_total_time_ms {
            return Err(format!(
                "max_inference_time_ms ({}) cannot exceed max_total_time_ms ({})",
                self.max_inference_time_ms, self.max_total_time_ms
            ));
        }
        
        Ok(())
    }
    
    /// Load configuration from a JSON file
    pub fn from_file(path: &PathBuf) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        let config: Config = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;
        
        config.validate()?;
        
        Ok(config)
    }
    
    /// Load configuration from environment variables
    /// 
    /// Supported environment variables:
    /// - JORM_TIER3_ENABLED: "true" or "false"
    /// - JORM_TIER3_MODEL_PATH: path to model directory
    /// - JORM_TIER3_DEVICE: "cpu", "cuda", or "metal"
    /// - JORM_TIER3_INTENT_THRESHOLD: float between 0.0 and 1.0
    /// - JORM_TIER3_ENTITY_THRESHOLD: float between 0.0 and 1.0
    /// - JORM_TIER3_SIMILARITY_THRESHOLD: float between 0.0 and 1.0
    /// - JORM_TIER3_MAX_INFERENCE_TIME_MS: positive integer
    /// - JORM_TIER3_MAX_TOTAL_TIME_MS: positive integer
    /// - JORM_TIER3_FALLBACK_TO_TIER2: "true" or "false"
    pub fn from_env() -> Result<Self, String> {
        let mut config = Self::default();
        
        if let Ok(enabled) = std::env::var("JORM_TIER3_ENABLED") {
            config.enabled = enabled.parse()
                .map_err(|_| format!("Invalid JORM_TIER3_ENABLED value: {}", enabled))?;
        }
        
        if let Ok(model_path) = std::env::var("JORM_TIER3_MODEL_PATH") {
            config.model_path = PathBuf::from(model_path);
        }
        
        if let Ok(device) = std::env::var("JORM_TIER3_DEVICE") {
            config.device = match device.to_lowercase().as_str() {
                "cpu" => DeviceType::CPU,
                "cuda" => DeviceType::CUDA,
                "metal" => DeviceType::Metal,
                _ => return Err(format!("Invalid JORM_TIER3_DEVICE value: {}", device)),
            };
        }
        
        if let Ok(threshold) = std::env::var("JORM_TIER3_INTENT_THRESHOLD") {
            config.intent_threshold = threshold.parse()
                .map_err(|_| format!("Invalid JORM_TIER3_INTENT_THRESHOLD value: {}", threshold))?;
        }
        
        if let Ok(threshold) = std::env::var("JORM_TIER3_ENTITY_THRESHOLD") {
            config.entity_threshold = threshold.parse()
                .map_err(|_| format!("Invalid JORM_TIER3_ENTITY_THRESHOLD value: {}", threshold))?;
        }
        
        if let Ok(threshold) = std::env::var("JORM_TIER3_SIMILARITY_THRESHOLD") {
            config.similarity_threshold = threshold.parse()
                .map_err(|_| format!("Invalid JORM_TIER3_SIMILARITY_THRESHOLD value: {}", threshold))?;
        }
        
        if let Ok(time) = std::env::var("JORM_TIER3_MAX_INFERENCE_TIME_MS") {
            config.max_inference_time_ms = time.parse()
                .map_err(|_| format!("Invalid JORM_TIER3_MAX_INFERENCE_TIME_MS value: {}", time))?;
        }
        
        if let Ok(time) = std::env::var("JORM_TIER3_MAX_TOTAL_TIME_MS") {
            config.max_total_time_ms = time.parse()
                .map_err(|_| format!("Invalid JORM_TIER3_MAX_TOTAL_TIME_MS value: {}", time))?;
        }
        
        if let Ok(fallback) = std::env::var("JORM_TIER3_FALLBACK_TO_TIER2") {
            config.fallback_to_tier2 = fallback.parse()
                .map_err(|_| format!("Invalid JORM_TIER3_FALLBACK_TO_TIER2 value: {}", fallback))?;
        }
        
        config.validate()?;
        
        Ok(config)
    }
    
    /// Save configuration to a JSON file
    pub fn to_file(&self, path: &PathBuf) -> Result<(), String> {
        self.validate()?;
        
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        std::fs::write(path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        
        Ok(())
    }
}

#[cfg(all(test, feature = "tier3"))]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.enabled);
        assert_eq!(config.intent_threshold, 0.7);
        assert_eq!(config.entity_threshold, 0.6);
        assert_eq!(config.similarity_threshold, 0.8);
        assert_eq!(config.max_inference_time_ms, 200);
        assert_eq!(config.max_total_time_ms, 500);
        assert!(config.fallback_to_tier2);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid intent threshold
        config.intent_threshold = 1.5;
        assert!(config.validate().is_err());
        config.intent_threshold = 0.7;
        
        // Invalid entity threshold
        config.entity_threshold = -0.1;
        assert!(config.validate().is_err());
        config.entity_threshold = 0.6;
        
        // Invalid similarity threshold
        config.similarity_threshold = 2.0;
        assert!(config.validate().is_err());
        config.similarity_threshold = 0.8;
        
        // Invalid time limits
        config.max_inference_time_ms = 0;
        assert!(config.validate().is_err());
        config.max_inference_time_ms = 200;
        
        config.max_total_time_ms = 0;
        assert!(config.validate().is_err());
        config.max_total_time_ms = 500;
        
        // Inference time exceeds total time
        config.max_inference_time_ms = 600;
        assert!(config.validate().is_err());
    }
}
