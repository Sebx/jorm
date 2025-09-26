//! Simplified model manager for on-demand LLM loading and caching

use anyhow::{Result, Context};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Model loading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model repository on Hugging Face
    pub repo: String,
    /// Model filename
    pub filename: String,
    /// Maximum model size in MB
    pub max_size_mb: u64,
    /// Cache directory
    pub cache_dir: PathBuf,
    /// Whether to use ONNX runtime
    pub use_onnx: bool,
}

/// Model loading status
#[derive(Debug, Clone, PartialEq)]
pub enum ModelStatus {
    NotLoaded,
    Loading,
    Loaded,
    Error(String),
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub name: String,
    pub version: String,
    pub size_bytes: u64,
    pub download_date: chrono::DateTime<chrono::Utc>,
    pub model_type: String,
}

/// Simplified model manager for on-demand loading
pub struct ModelManager {
    /// Model configuration
    config: ModelConfig,
    /// Current model status
    status: Arc<RwLock<ModelStatus>>,
    /// Model metadata
    metadata: Arc<RwLock<Option<ModelMetadata>>>,
    /// Model cache (simplified)
    cache: Arc<RwLock<Option<SimplifiedModelCache>>>,
}

/// Simplified model cache
pub struct SimplifiedModelCache {
    /// Model path
    pub model_path: PathBuf,
    /// Model metadata
    pub metadata: ModelMetadata,
}

impl ModelManager {
    /// Create a new model manager
    pub fn new(config: ModelConfig) -> Result<Self> {
        let cache_dir = config.cache_dir.clone();
        std::fs::create_dir_all(&cache_dir)
            .context("Failed to create cache directory")?;

        Ok(Self {
            config,
            status: Arc::new(RwLock::new(ModelStatus::NotLoaded)),
            metadata: Arc::new(RwLock::new(None)),
            cache: Arc::new(RwLock::new(None)),
        })
    }

    /// Check if model is available (loaded or can be loaded)
    pub async fn is_available(&self) -> bool {
        let status = self.status.read().await;
        matches!(*status, ModelStatus::Loaded)
    }

    /// Get current model status
    pub async fn get_status(&self) -> ModelStatus {
        self.status.read().await.clone()
    }

    /// Load model on-demand
    pub async fn load_model(&self) -> Result<()> {
        // Check if already loaded
        if self.is_available().await {
            return Ok(());
        }

        // Set loading status
        {
            let mut status = self.status.write().await;
            *status = ModelStatus::Loading;
        }

        println!("🔄 Loading model: {}", self.config.repo);
        println!("📁 Cache directory: {}", self.config.cache_dir.display());

        // Try to load from cache first
        if let Ok(cached_model) = self.load_from_cache().await {
            let mut cache = self.cache.write().await;
            *cache = Some(cached_model);
            
            let mut status = self.status.write().await;
            *status = ModelStatus::Loaded;
            println!("✅ Model loaded from cache");
            return Ok(());
        }

        // Simulate model download and loading
        println!("📥 Simulating model download...");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Create a mock model cache
        let model_cache = SimplifiedModelCache {
            model_path: self.config.cache_dir.join(&self.config.filename),
            metadata: ModelMetadata {
                name: self.config.repo.clone(),
                version: "1.0".to_string(),
                size_bytes: self.config.max_size_mb * 1024 * 1024,
                download_date: chrono::Utc::now(),
                model_type: if self.config.use_onnx { "onnx".to_string() } else { "candle".to_string() },
            },
        };

        // Save to cache
        self.save_to_cache(&model_cache).await?;
        
        let mut cache = self.cache.write().await;
        *cache = Some(model_cache);
        
        let mut status = self.status.write().await;
        *status = ModelStatus::Loaded;
        println!("✅ Model loaded successfully");

        Ok(())
    }

    /// Get model cache (must be loaded)
    pub async fn get_model_cache(&self) -> Result<Arc<SimplifiedModelCache>> {
        let cache = self.cache.read().await;
        cache.as_ref()
            .map(|c| Arc::new(c.clone()))
            .ok_or_else(|| anyhow::anyhow!("Model not loaded"))
    }

    /// Load model from cache
    async fn load_from_cache(&self) -> Result<SimplifiedModelCache> {
        let metadata_path = self.config.cache_dir.join("metadata.json");
        if !metadata_path.exists() {
            return Err(anyhow::anyhow!("No cached model found"));
        }

        let metadata: ModelMetadata = serde_json::from_str(
            &tokio::fs::read_to_string(&metadata_path).await?
        )?;

        let model_path = self.config.cache_dir.join(&self.config.filename);
        if !model_path.exists() {
            return Err(anyhow::anyhow!("Cached model file not found"));
        }

        Ok(SimplifiedModelCache {
            model_path,
            metadata,
        })
    }

    /// Save model to cache
    async fn save_to_cache(&self, model_cache: &SimplifiedModelCache) -> Result<()> {
        // Create cache directory if it doesn't exist
        tokio::fs::create_dir_all(&self.config.cache_dir).await?;

        // Create a mock model file
        let model_path = self.config.cache_dir.join(&self.config.filename);
        tokio::fs::write(&model_path, format!("Mock model file for {}", self.config.repo)).await?;

        // Save metadata
        let metadata_path = self.config.cache_dir.join("metadata.json");
        tokio::fs::write(
            &metadata_path,
            serde_json::to_string_pretty(&model_cache.metadata)?
        ).await?;

        Ok(())
    }

    /// Get model metadata
    pub async fn get_metadata(&self) -> Option<ModelMetadata> {
        self.metadata.read().await.clone()
    }

    /// Clear model cache
    pub async fn clear_cache(&self) -> Result<()> {
        if self.config.cache_dir.exists() {
            tokio::fs::remove_dir_all(&self.config.cache_dir).await?;
            tokio::fs::create_dir_all(&self.config.cache_dir).await?;
        }

        let mut status = self.status.write().await;
        *status = ModelStatus::NotLoaded;

        let mut cache = self.cache.write().await;
        *cache = None;

        Ok(())
    }
}

impl Clone for SimplifiedModelCache {
    fn clone(&self) -> Self {
        Self {
            model_path: self.model_path.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

/// Predefined model configurations
pub struct ModelConfigs;

impl ModelConfigs {
    /// Phi-3 Mini configuration
    pub fn phi3_mini() -> ModelConfig {
        ModelConfig {
            repo: "microsoft/Phi-3-mini-4k-instruct".to_string(),
            filename: "model.onnx".to_string(),
            max_size_mb: 500,
            cache_dir: std::env::temp_dir()
                .join("jorm-rs")
                .join("phi3-mini"),
            use_onnx: true,
        }
    }

    /// Gemma 2B configuration
    pub fn gemma_2b() -> ModelConfig {
        ModelConfig {
            repo: "google/gemma-2b-it".to_string(),
            filename: "model.onnx".to_string(),
            max_size_mb: 500,
            cache_dir: std::env::temp_dir()
                .join("jorm-rs")
                .join("gemma-2b"),
            use_onnx: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_model_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = ModelConfig {
            repo: "test/model".to_string(),
            filename: "model.onnx".to_string(),
            max_size_mb: 100,
            cache_dir: temp_dir.path().to_path_buf(),
            use_onnx: true,
        };

        let manager = ModelManager::new(config).unwrap();
        assert!(!manager.is_available().await);
        assert_eq!(manager.get_status().await, ModelStatus::NotLoaded);
    }

    #[tokio::test]
    async fn test_model_loading() {
        let temp_dir = TempDir::new().unwrap();
        let config = ModelConfig {
            repo: "test/model".to_string(),
            filename: "model.onnx".to_string(),
            max_size_mb: 100,
            cache_dir: temp_dir.path().to_path_buf(),
            use_onnx: true,
        };

        let manager = ModelManager::new(config).unwrap();
        
        // Test loading
        manager.load_model().await.unwrap();
        assert!(manager.is_available().await);
        assert_eq!(manager.get_status().await, ModelStatus::Loaded);
    }

    #[tokio::test]
    async fn test_model_configs() {
        let phi3_config = ModelConfigs::phi3_mini();
        assert_eq!(phi3_config.repo, "microsoft/Phi-3-mini-4k-instruct");
        assert!(phi3_config.use_onnx);

        let gemma_config = ModelConfigs::gemma_2b();
        assert_eq!(gemma_config.repo, "google/gemma-2b-it");
        assert!(gemma_config.use_onnx);
    }
}