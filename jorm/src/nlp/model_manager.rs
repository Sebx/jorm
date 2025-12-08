//! Model manager for loading and managing transformer models
//!
//! This module handles loading transformer models, tokenization, and embedding generation
//! using the Candle framework.

#[cfg(feature = "tier3")]
use std::path::PathBuf;
#[cfg(feature = "tier3")]
use std::time::Instant;

#[cfg(feature = "tier3")]
use candle_core::{Device, Tensor, IndexOp};
#[cfg(feature = "tier3")]
use candle_nn::VarBuilder;
#[cfg(feature = "tier3")]
use candle_transformers::models::bert::{BertModel, Config as BertConfig};

#[cfg(feature = "tier3")]
use tokenizers::Tokenizer;

#[cfg(feature = "tier3")]
use crate::core::error::JormError;
#[cfg(feature = "tier3")]
use crate::nlp::config::{Config, DeviceType};
#[cfg(feature = "tier3")]
use crate::nlp::errors::Tier3Error;

/// Manages transformer model loading and inference
#[cfg(feature = "tier3")]
pub struct ModelManager {
    model_path: PathBuf,
    model: Option<BertModel>,
    tokenizer: Option<Tokenizer>,
    device: Device,
    config: Config,
    memory_usage_bytes: usize,
}

#[cfg(feature = "tier3")]
impl ModelManager {
    /// Create a new ModelManager with the specified configuration
    pub fn new(config: Config) -> Result<Self, JormError> {
        // Validate configuration
        config.validate().map_err(|e| {
            JormError::from(Tier3Error::ConfigError(e))
        })?;
        
        // Select device based on configuration
        let device = Self::select_device(&config.device)?;
        
        Ok(Self {
            model_path: config.model_path.clone(),
            model: None,
            tokenizer: None,
            device,
            config,
            memory_usage_bytes: 0,
        })
    }

    /// Select the appropriate device for inference
    fn select_device(device_type: &DeviceType) -> Result<Device, JormError> {
        match device_type {
            DeviceType::CPU => Ok(Device::Cpu),
            DeviceType::CUDA => {
                #[cfg(feature = "cuda")]
                {
                    Device::cuda_if_available(0).map_err(|e| {
                        JormError::from(Tier3Error::DeviceNotAvailable(format!(
                            "CUDA device not available: {}",
                            e
                        )))
                    })
                }
                #[cfg(not(feature = "cuda"))]
                {
                    Err(JormError::from(Tier3Error::DeviceNotAvailable(
                        "CUDA support not compiled in".to_string(),
                    )))
                }
            }
            DeviceType::Metal => {
                #[cfg(feature = "metal")]
                {
                    Device::new_metal(0).map_err(|e| {
                        JormError::from(Tier3Error::DeviceNotAvailable(format!(
                            "Metal device not available: {}",
                            e
                        )))
                    })
                }
                #[cfg(not(feature = "metal"))]
                {
                    Err(JormError::from(Tier3Error::DeviceNotAvailable(
                        "Metal support not compiled in".to_string(),
                    )))
                }
            }
        }
    }

    /// Check if the model is loaded
    pub fn is_loaded(&self) -> bool {
        self.model.is_some() && self.tokenizer.is_some()
    }

    /// Load the transformer model from disk
    pub fn load_model(&mut self) -> Result<(), JormError> {
        let start_time = Instant::now();
        
        // Check if model directory exists
        if !self.model_path.exists() {
            return Err(JormError::from(Tier3Error::ModelNotFound(format!(
                "Model directory not found: {}",
                self.model_path.display()
            ))));
        }
        
        // Load tokenizer
        let tokenizer_path = self.model_path.join("tokenizer.json");
        if !tokenizer_path.exists() {
            return Err(JormError::from(Tier3Error::ModelNotFound(format!(
                "Tokenizer file not found: {}",
                tokenizer_path.display()
            ))));
        }
        
        let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(|e| {
            JormError::from(Tier3Error::ModelLoadError(format!(
                "Failed to load tokenizer: {}",
                e
            )))
        })?;
        
        // Load model configuration
        let config_path = self.model_path.join("config.json");
        if !config_path.exists() {
            return Err(JormError::from(Tier3Error::ModelNotFound(format!(
                "Model config file not found: {}",
                config_path.display()
            ))));
        }
        
        let config_content = std::fs::read_to_string(&config_path).map_err(|e| {
            JormError::from(Tier3Error::ModelLoadError(format!(
                "Failed to read model config: {}",
                e
            )))
        })?;
        
        let bert_config: BertConfig = serde_json::from_str(&config_content).map_err(|e| {
            JormError::from(Tier3Error::ModelLoadError(format!(
                "Failed to parse model config: {}",
                e
            )))
        })?;
        
        // Load model weights (try safetensors first, then pytorch)
        let weights_path = if self.model_path.join("model.safetensors").exists() {
            self.model_path.join("model.safetensors")
        } else if self.model_path.join("pytorch_model.bin").exists() {
            self.model_path.join("pytorch_model.bin")
        } else {
            return Err(JormError::from(Tier3Error::ModelNotFound(
                "Model weights file not found (tried model.safetensors and pytorch_model.bin)".to_string(),
            )));
        };
        
        let vb = if weights_path.extension().and_then(|s| s.to_str()) == Some("safetensors") {
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &self.device) }
                .map_err(|e| {
                    JormError::from(Tier3Error::ModelLoadError(format!(
                        "Failed to load safetensors: {}",
                        e
                    )))
                })?
        } else {
            VarBuilder::from_pth(&weights_path, candle_core::DType::F32, &self.device)
                .map_err(|e| {
                    JormError::from(Tier3Error::ModelLoadError(format!(
                        "Failed to load pytorch weights: {}",
                        e
                    )))
                })?
        };
        
        // Create model
        let model = BertModel::load(vb, &bert_config).map_err(|e| {
            JormError::from(Tier3Error::ModelLoadError(format!(
                "Failed to create BERT model: {}",
                e
            )))
        })?;
        
        // Estimate memory usage (rough estimate based on model parameters)
        self.memory_usage_bytes = Self::estimate_model_size(&bert_config);
        
        self.model = Some(model);
        self.tokenizer = Some(tokenizer);
        
        let load_time = start_time.elapsed();
        println!("✅ Model loaded in {:.2}s (estimated size: {:.1}MB)", 
                 load_time.as_secs_f64(), 
                 self.memory_usage_bytes as f64 / 1_048_576.0);
        
        Ok(())
    }

    /// Estimate model size based on configuration
    fn estimate_model_size(config: &BertConfig) -> usize {
        // Rough estimate: hidden_size * num_layers * 4 (for weights) * 4 (bytes per float32)
        let params = config.hidden_size * config.num_hidden_layers * 4;
        params * 4 // 4 bytes per float32
    }

    /// Tokenize input text
    pub fn tokenize(&self, text: &str) -> Result<Vec<u32>, JormError> {
        let tokenizer = self.tokenizer.as_ref().ok_or_else(|| {
            JormError::from(Tier3Error::TokenizationError(
                "Tokenizer not loaded".to_string(),
            ))
        })?;
        
        let encoding = tokenizer
            .encode(text, true)
            .map_err(|e| {
                JormError::from(Tier3Error::TokenizationError(format!(
                    "Failed to tokenize text: {}",
                    e
                )))
            })?;
        
        let tokens = encoding.get_ids().to_vec();
        
        // Validate token length (most models have a max length of 512)
        const MAX_TOKEN_LENGTH: usize = 512;
        if tokens.len() > MAX_TOKEN_LENGTH {
            return Err(JormError::from(Tier3Error::TokenizationError(format!(
                "Input text too long: {} tokens (max: {})",
                tokens.len(),
                MAX_TOKEN_LENGTH
            ))));
        }
        
        Ok(tokens)
    }

    /// Generate embeddings for the given tokens
    pub fn generate_embeddings(&self, tokens: &[u32]) -> Result<Tensor, JormError> {
        let start_time = Instant::now();
        
        let model = self.model.as_ref().ok_or_else(|| {
            JormError::from(Tier3Error::InferenceError(
                "Model not loaded".to_string(),
            ))
        })?;
        
        // Convert tokens to tensor
        let token_ids = Tensor::new(tokens, &self.device).map_err(|e| {
            JormError::from(Tier3Error::InferenceError(format!(
                "Failed to create token tensor: {}",
                e
            )))
        })?;
        
        // Add batch dimension
        let token_ids = token_ids.unsqueeze(0).map_err(|e| {
            JormError::from(Tier3Error::InferenceError(format!(
                "Failed to add batch dimension: {}",
                e
            )))
        })?;
        
        // Create attention mask (all ones for valid tokens)
        let attention_mask = Tensor::ones(token_ids.shape(), candle_core::DType::U32, &self.device)
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to create attention mask: {}",
                    e
                )))
            })?;
        
        // Run forward pass (third parameter is token_type_ids, None for single sentence)
        let embeddings = model
            .forward(&token_ids, &attention_mask, None)
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Forward pass failed: {}",
                    e
                )))
            })?;
        
        // Extract [CLS] token embedding (first token) for sentence representation
        let sentence_embedding = embeddings
            .i((0, 0))
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to extract CLS embedding: {}",
                    e
                )))
            })?;
        
        // Normalize the embedding
        let normalized = Self::normalize_embedding(&sentence_embedding)?;
        
        let inference_time = start_time.elapsed();
        
        // Check if inference time exceeds threshold
        if inference_time.as_millis() > self.config.max_inference_time_ms as u128 {
            return Err(JormError::from(Tier3Error::InferenceTimeout(
                inference_time.as_millis() as u64,
            )));
        }
        
        Ok(normalized)
    }

    /// Normalize an embedding vector to unit length
    fn normalize_embedding(embedding: &Tensor) -> Result<Tensor, JormError> {
        // Calculate L2 norm
        let norm = embedding
            .sqr()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to square embedding: {}",
                    e
                )))
            })?
            .sum_all()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to sum embedding: {}",
                    e
                )))
            })?
            .sqrt()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to calculate sqrt: {}",
                    e
                )))
            })?;
        
        // Divide by norm to normalize
        let normalized = embedding.broadcast_div(&norm).map_err(|e| {
            JormError::from(Tier3Error::InferenceError(format!(
                "Failed to normalize embedding: {}",
                e
            )))
        })?;
        
        Ok(normalized)
    }

    /// Unload the model to free memory
    pub fn unload_model(&mut self) {
        self.model = None;
        self.tokenizer = None;
        self.memory_usage_bytes = 0;
        println!("🗑️  Model unloaded, memory freed");
    }

    /// Get estimated memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.memory_usage_bytes
    }

    /// Get the device being used for inference
    pub fn device(&self) -> &Device {
        &self.device
    }
}

#[cfg(all(test, feature = "tier3"))]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config(model_path: PathBuf) -> Config {
        Config {
            enabled: true,
            model_path,
            model_type: crate::nlp::config::ModelType::MiniLM,
            device: DeviceType::CPU,
            intent_threshold: 0.7,
            entity_threshold: 0.6,
            similarity_threshold: 0.8,
            max_inference_time_ms: 200,
            max_total_time_ms: 500,
            fallback_to_tier2: true,
        }
    }

    #[test]
    fn test_model_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        
        let manager = ModelManager::new(config);
        assert!(manager.is_ok());
        
        let manager = manager.unwrap();
        assert!(!manager.is_loaded());
        assert_eq!(manager.memory_usage(), 0);
    }

    #[test]
    fn test_model_manager_invalid_config() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config(temp_dir.path().to_path_buf());
        
        // Invalid threshold
        config.intent_threshold = 1.5;
        
        let manager = ModelManager::new(config);
        assert!(manager.is_err());
    }

    #[test]
    fn test_device_selection_cpu() {
        let result = ModelManager::select_device(&DeviceType::CPU);
        assert!(result.is_ok());
    }

    #[test]
    fn test_model_not_loaded_error() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let manager = ModelManager::new(config).unwrap();
        
        // Try to tokenize without loading model
        let result = manager.tokenize("test text");
        assert!(result.is_err());
        
        // Try to generate embeddings without loading model
        let result = manager.generate_embeddings(&[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_model_directory_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let non_existent_path = temp_dir.path().join("non_existent_model");
        let config = create_test_config(non_existent_path);
        
        let mut manager = ModelManager::new(config).unwrap();
        let result = manager.load_model();
        
        assert!(result.is_err());
        match result {
            Err(JormError::NlpError(msg)) => {
                assert!(msg.contains("Model directory not found"));
            }
            _ => panic!("Expected ModelNotFound error"),
        }
    }

    #[test]
    fn test_tokenizer_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let model_dir = temp_dir.path().join("test_model");
        fs::create_dir(&model_dir).unwrap();
        
        let config = create_test_config(model_dir);
        let mut manager = ModelManager::new(config).unwrap();
        
        let result = manager.load_model();
        assert!(result.is_err());
        match result {
            Err(JormError::NlpError(msg)) => {
                assert!(msg.contains("Tokenizer file not found"));
            }
            _ => panic!("Expected tokenizer not found error"),
        }
    }

    #[test]
    fn test_memory_management() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let mut manager = ModelManager::new(config).unwrap();
        
        // Initially no memory used
        assert_eq!(manager.memory_usage(), 0);
        assert!(!manager.is_loaded());
        
        // After unload, memory should be freed
        manager.unload_model();
        assert_eq!(manager.memory_usage(), 0);
        assert!(!manager.is_loaded());
    }

    #[test]
    fn test_estimate_model_size() {
        // Create a minimal config for testing
        let config = BertConfig {
            vocab_size: 30522,
            hidden_size: 384,
            num_hidden_layers: 6,
            num_attention_heads: 12,
            intermediate_size: 1536,
            hidden_act: candle_transformers::models::bert::HiddenAct::Gelu,
            hidden_dropout_prob: 0.1,
            max_position_embeddings: 512,
            type_vocab_size: 2,
            initializer_range: 0.02,
            layer_norm_eps: 1e-12,
            classifier_dropout: None,
            model_type: None,
            pad_token_id: 0,
            position_embedding_type: candle_transformers::models::bert::PositionEmbeddingType::Absolute,
            use_cache: true,
        };
        
        let size = ModelManager::estimate_model_size(&config);
        
        // Should be non-zero
        assert!(size > 0);
        
        // Should be reasonable (a few MB for a small model)
        assert!(size < 100_000_000); // Less than 100MB
    }

    #[test]
    fn test_token_length_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let manager = ModelManager::new(config).unwrap();
        
        // Create a very long text that would exceed token limit
        let long_text = "word ".repeat(1000);
        
        // This should fail because tokenizer is not loaded
        let result = manager.tokenize(&long_text);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_embedding() {
        // Create a simple embedding tensor
        let device = Device::Cpu;
        let embedding = Tensor::new(&[3.0f32, 4.0f32], &device).unwrap();
        
        let normalized = ModelManager::normalize_embedding(&embedding).unwrap();
        
        // Check that the normalized vector has unit length
        let norm_squared = normalized
            .sqr()
            .unwrap()
            .sum_all()
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        
        // Should be approximately 1.0 (allowing for floating point errors)
        assert!((norm_squared - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_device_getter() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let manager = ModelManager::new(config).unwrap();
        
        let device = manager.device();
        // Should be CPU device as configured
        assert!(matches!(device, Device::Cpu));
    }
}
