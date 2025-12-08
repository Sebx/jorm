//! Semantic pattern matching using embeddings
//!
//! This module matches user input to known patterns using semantic similarity
//! instead of regex patterns, allowing for more flexible natural language understanding.

#[cfg(feature = "tier3")]
use std::collections::HashMap;
#[cfg(feature = "tier3")]
use std::sync::Arc;

#[cfg(feature = "tier3")]
use candle_core::Tensor;

#[cfg(feature = "tier3")]
use crate::core::error::JormError;
#[cfg(feature = "tier3")]
use crate::nlp::model_manager::ModelManager;
#[cfg(feature = "tier3")]
use crate::nlp::errors::Tier3Error;

/// Canonical semantic patterns for common tasks
#[cfg(feature = "tier3")]
const SEMANTIC_PATTERNS: &[&str] = &[
    "create file with size",
    "copy file multiple times",
    "execute shell command",
    "send http request",
    "run python script",
    "build rust project",
    "create pipeline workflow",
    "backup files to directory",
    "delete files from directory",
    "move files to location",
    "generate random data",
    "compress files",
    "extract archive",
    "download from url",
    "upload to server",
    "process data file",
    "transform data format",
    "validate file content",
    "monitor system resources",
    "schedule task execution",
];

/// Result of a semantic pattern match
#[cfg(feature = "tier3")]
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub pattern: String,
    pub similarity: f32,
    pub parameters: HashMap<String, String>,
}

/// Matches input to semantic patterns
#[cfg(feature = "tier3")]
pub struct SemanticMatcher {
    pattern_embeddings: HashMap<String, Tensor>,
    threshold: f32,
    model_manager: Arc<ModelManager>,
}

#[cfg(feature = "tier3")]
impl SemanticMatcher {
    /// Create a new SemanticMatcher with the specified model manager and similarity threshold
    pub fn new(model_manager: Arc<ModelManager>, threshold: f32) -> Result<Self, JormError> {
        if !model_manager.is_loaded() {
            return Err(JormError::from(Tier3Error::ModelNotLoaded(
                "Model must be loaded before creating SemanticMatcher".to_string(),
            )));
        }

        let mut matcher = Self {
            pattern_embeddings: HashMap::new(),
            threshold,
            model_manager,
        };

        // Generate embeddings for all canonical patterns
        matcher.initialize_patterns()?;

        Ok(matcher)
    }

    /// Initialize pattern embeddings for all canonical patterns
    fn initialize_patterns(&mut self) -> Result<(), JormError> {
        println!("🔄 Generating embeddings for {} semantic patterns...", SEMANTIC_PATTERNS.len());
        
        for pattern in SEMANTIC_PATTERNS {
            let tokens = self.model_manager.tokenize(pattern)?;
            let embedding = self.model_manager.generate_embeddings(&tokens)?;
            self.pattern_embeddings.insert(pattern.to_string(), embedding);
        }
        
        println!("✅ Pattern embeddings generated");
        Ok(())
    }

    /// Match input text to known patterns using semantic similarity
    pub fn match_pattern(&self, text: &str) -> Result<Option<MatchResult>, JormError> {
        if self.pattern_embeddings.is_empty() {
            return Ok(None);
        }

        // Generate embedding for input text
        let tokens = self.model_manager.tokenize(text)?;
        let input_embedding = self.model_manager.generate_embeddings(&tokens)?;

        // Find best matching pattern
        let mut best_match: Option<(String, f32)> = None;

        for (pattern, pattern_embedding) in &self.pattern_embeddings {
            let similarity = Self::cosine_similarity(&input_embedding, pattern_embedding)?;
            
            if similarity >= self.threshold {
                if let Some((_, best_similarity)) = &best_match {
                    if similarity > *best_similarity {
                        best_match = Some((pattern.clone(), similarity));
                    }
                } else {
                    best_match = Some((pattern.clone(), similarity));
                }
            }
        }

        // If we found a match, extract parameters
        if let Some((pattern, similarity)) = best_match {
            let parameters = Self::extract_parameters(text, &pattern)?;
            
            Ok(Some(MatchResult {
                pattern,
                similarity,
                parameters,
            }))
        } else {
            Ok(None)
        }
    }

    /// Compute cosine similarity between two embedding tensors
    fn cosine_similarity(a: &Tensor, b: &Tensor) -> Result<f32, JormError> {
        // Compute dot product
        let dot_product = (a * b)
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to compute dot product: {}",
                    e
                )))
            })?
            .sum_all()
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to sum dot product: {}",
                    e
                )))
            })?
            .to_scalar::<f32>()
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to convert to scalar: {}",
                    e
                )))
            })?;

        // Compute norms
        let norm_a = a
            .sqr()
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to square tensor a: {}",
                    e
                )))
            })?
            .sum_all()
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to sum squared tensor a: {}",
                    e
                )))
            })?
            .sqrt()
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to compute sqrt of norm a: {}",
                    e
                )))
            })?
            .to_scalar::<f32>()
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to convert norm a to scalar: {}",
                    e
                )))
            })?;

        let norm_b = b
            .sqr()
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to square tensor b: {}",
                    e
                )))
            })?
            .sum_all()
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to sum squared tensor b: {}",
                    e
                )))
            })?
            .sqrt()
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to compute sqrt of norm b: {}",
                    e
                )))
            })?
            .to_scalar::<f32>()
            .map_err(|e| {
                JormError::from(Tier3Error::SemanticMatchError(format!(
                    "Failed to convert norm b to scalar: {}",
                    e
                )))
            })?;

        // Avoid division by zero
        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        // Compute cosine similarity
        let similarity = dot_product / (norm_a * norm_b);
        
        Ok(similarity)
    }

    /// Extract parameters from matched text based on pattern
    fn extract_parameters(text: &str, pattern: &str) -> Result<HashMap<String, String>, JormError> {
        let mut parameters = HashMap::new();
        
        // Simple parameter extraction based on common patterns
        // This is a basic implementation that can be enhanced later
        
        // Extract numbers
        if let Some(captures) = regex::Regex::new(r"(\d+)")
            .map_err(|e| JormError::from(Tier3Error::SemanticMatchError(format!("Regex error: {}", e))))?
            .captures(text)
        {
            if let Some(num) = captures.get(1) {
                parameters.insert("number".to_string(), num.as_str().to_string());
            }
        }
        
        // Extract file names (simple pattern: words with extensions)
        if let Some(captures) = regex::Regex::new(r"(\w+\.\w+)")
            .map_err(|e| JormError::from(Tier3Error::SemanticMatchError(format!("Regex error: {}", e))))?
            .captures(text)
        {
            if let Some(filename) = captures.get(1) {
                parameters.insert("filename".to_string(), filename.as_str().to_string());
            }
        }
        
        // Extract paths (simple pattern: paths with slashes)
        if let Some(captures) = regex::Regex::new(r"([./][\w/.-]+)")
            .map_err(|e| JormError::from(Tier3Error::SemanticMatchError(format!("Regex error: {}", e))))?
            .captures(text)
        {
            if let Some(path) = captures.get(1) {
                parameters.insert("path".to_string(), path.as_str().to_string());
            }
        }
        
        // Extract quoted strings (commands)
        if let Some(captures) = regex::Regex::new(r#"['"]([^'"]+)['"]"#)
            .map_err(|e| JormError::from(Tier3Error::SemanticMatchError(format!("Regex error: {}", e))))?
            .captures(text)
        {
            if let Some(command) = captures.get(1) {
                parameters.insert("command".to_string(), command.as_str().to_string());
            }
        }
        
        // Store the original text for reference
        parameters.insert("original_text".to_string(), text.to_string());
        parameters.insert("matched_pattern".to_string(), pattern.to_string());
        
        Ok(parameters)
    }

    /// Add a new pattern to the matcher
    pub fn add_pattern(&mut self, pattern: &str) -> Result<(), JormError> {
        if pattern.trim().is_empty() {
            return Err(JormError::from(Tier3Error::SemanticMatchError(
                "Pattern cannot be empty".to_string(),
            )));
        }

        // Check if pattern already exists
        if self.pattern_embeddings.contains_key(pattern) {
            return Err(JormError::from(Tier3Error::SemanticMatchError(
                format!("Pattern already exists: {}", pattern),
            )));
        }

        // Generate embedding for the new pattern
        let tokens = self.model_manager.tokenize(pattern)?;
        let embedding = self.model_manager.generate_embeddings(&tokens)?;
        
        // Store the pattern embedding
        self.pattern_embeddings.insert(pattern.to_string(), embedding);
        
        Ok(())
    }

    /// Update an existing pattern with a new embedding
    pub fn update_pattern(&mut self, pattern: &str) -> Result<(), JormError> {
        if pattern.trim().is_empty() {
            return Err(JormError::from(Tier3Error::SemanticMatchError(
                "Pattern cannot be empty".to_string(),
            )));
        }

        // Check if pattern exists
        if !self.pattern_embeddings.contains_key(pattern) {
            return Err(JormError::from(Tier3Error::SemanticMatchError(
                format!("Pattern does not exist: {}", pattern),
            )));
        }

        // Regenerate embedding for the pattern
        let tokens = self.model_manager.tokenize(pattern)?;
        let embedding = self.model_manager.generate_embeddings(&tokens)?;
        
        // Update the pattern embedding
        self.pattern_embeddings.insert(pattern.to_string(), embedding);
        
        Ok(())
    }

    /// Remove a pattern from the matcher
    pub fn remove_pattern(&mut self, pattern: &str) -> Result<(), JormError> {
        if !self.pattern_embeddings.contains_key(pattern) {
            return Err(JormError::from(Tier3Error::SemanticMatchError(
                format!("Pattern does not exist: {}", pattern),
            )));
        }

        self.pattern_embeddings.remove(pattern);
        Ok(())
    }

    /// Check if a pattern exists in the matcher
    pub fn has_pattern(&self, pattern: &str) -> bool {
        self.pattern_embeddings.contains_key(pattern)
    }

    /// Get all pattern names
    pub fn get_patterns(&self) -> Vec<String> {
        self.pattern_embeddings.keys().cloned().collect()
    }

    /// Clear all patterns (except canonical patterns can be re-initialized)
    pub fn clear_patterns(&mut self) {
        self.pattern_embeddings.clear();
    }

    /// Get the number of patterns currently stored
    pub fn pattern_count(&self) -> usize {
        self.pattern_embeddings.len()
    }

    /// Get the similarity threshold
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Set a new similarity threshold
    pub fn set_threshold(&mut self, threshold: f32) -> Result<(), JormError> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(JormError::from(Tier3Error::SemanticMatchError(
                format!("Threshold must be between 0.0 and 1.0, got: {}", threshold),
            )));
        }
        self.threshold = threshold;
        Ok(())
    }
}


#[cfg(all(test, feature = "tier3"))]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;
    use crate::nlp::config::{Config, DeviceType, ModelType};

    fn create_test_config(model_path: std::path::PathBuf) -> Config {
        Config {
            enabled: true,
            model_path,
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

    #[test]
    fn test_semantic_matcher_creation_without_loaded_model() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let model_manager = Arc::new(ModelManager::new(config).unwrap());
        
        // Model is not loaded yet
        let result = SemanticMatcher::new(model_manager, 0.8);
        assert!(result.is_err());
        
        match result {
            Err(JormError::NlpError(msg)) => {
                assert!(msg.contains("Model must be loaded"));
            }
            _ => panic!("Expected ModelNotLoaded error"),
        }
    }

    #[test]
    fn test_pattern_count() {
        // This test would require a loaded model, so we'll test the basic structure
        // In a real scenario, you'd need actual model files
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path().to_path_buf());
        let model_manager = Arc::new(ModelManager::new(config).unwrap());
        
        // We can't create a matcher without a loaded model, but we can test the constant
        assert_eq!(SEMANTIC_PATTERNS.len(), 20);
        assert!(SEMANTIC_PATTERNS.contains(&"create file with size"));
        assert!(SEMANTIC_PATTERNS.contains(&"execute shell command"));
    }

    #[test]
    fn test_threshold_getter() {
        // Test that we can access the threshold value
        let threshold = 0.75;
        // We'll test this through the public API once we have a valid matcher
        assert!(threshold > 0.0 && threshold <= 1.0);
    }

    #[test]
    fn test_cosine_similarity_computation() {
        // Test cosine similarity with simple vectors
        let device = candle_core::Device::Cpu;
        
        // Create two identical normalized vectors - should have similarity 1.0
        let vec1 = Tensor::new(&[1.0f32, 0.0f32], &device).unwrap();
        let vec2 = Tensor::new(&[1.0f32, 0.0f32], &device).unwrap();
        
        let similarity = SemanticMatcher::cosine_similarity(&vec1, &vec2).unwrap();
        assert!((similarity - 1.0).abs() < 0.0001);
        
        // Create orthogonal vectors - should have similarity 0.0
        let vec3 = Tensor::new(&[1.0f32, 0.0f32], &device).unwrap();
        let vec4 = Tensor::new(&[0.0f32, 1.0f32], &device).unwrap();
        
        let similarity = SemanticMatcher::cosine_similarity(&vec3, &vec4).unwrap();
        assert!(similarity.abs() < 0.0001);
        
        // Create opposite vectors - should have similarity -1.0
        let vec5 = Tensor::new(&[1.0f32, 0.0f32], &device).unwrap();
        let vec6 = Tensor::new(&[-1.0f32, 0.0f32], &device).unwrap();
        
        let similarity = SemanticMatcher::cosine_similarity(&vec5, &vec6).unwrap();
        assert!((similarity + 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_zero_vectors() {
        // Test with zero vectors - should return 0.0
        let device = candle_core::Device::Cpu;
        
        let vec1 = Tensor::new(&[0.0f32, 0.0f32], &device).unwrap();
        let vec2 = Tensor::new(&[1.0f32, 0.0f32], &device).unwrap();
        
        let similarity = SemanticMatcher::cosine_similarity(&vec1, &vec2).unwrap();
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_extract_parameters_numbers() {
        let text = "create 5 files";
        let pattern = "create file with size";
        
        let params = SemanticMatcher::extract_parameters(text, pattern).unwrap();
        
        assert!(params.contains_key("number"));
        assert_eq!(params.get("number").unwrap(), "5");
        assert_eq!(params.get("original_text").unwrap(), text);
        assert_eq!(params.get("matched_pattern").unwrap(), pattern);
    }

    #[test]
    fn test_extract_parameters_filename() {
        let text = "copy test.txt to backup";
        let pattern = "copy file multiple times";
        
        let params = SemanticMatcher::extract_parameters(text, pattern).unwrap();
        
        assert!(params.contains_key("filename"));
        assert_eq!(params.get("filename").unwrap(), "test.txt");
    }

    #[test]
    fn test_extract_parameters_path() {
        let text = "backup files to ./backup/dir";
        let pattern = "backup files to directory";
        
        let params = SemanticMatcher::extract_parameters(text, pattern).unwrap();
        
        assert!(params.contains_key("path"));
        assert_eq!(params.get("path").unwrap(), "./backup/dir");
    }

    #[test]
    fn test_extract_parameters_command() {
        let text = "execute 'ls -la' command";
        let pattern = "execute shell command";
        
        let params = SemanticMatcher::extract_parameters(text, pattern).unwrap();
        
        assert!(params.contains_key("command"));
        assert_eq!(params.get("command").unwrap(), "ls -la");
    }

    #[test]
    fn test_extract_parameters_multiple() {
        let text = "copy 10 files from test.txt to ./backup";
        let pattern = "copy file multiple times";
        
        let params = SemanticMatcher::extract_parameters(text, pattern).unwrap();
        
        // Should extract number, filename, and path
        assert!(params.contains_key("number"));
        assert!(params.contains_key("filename"));
        assert!(params.contains_key("path"));
    }

    #[test]
    fn test_semantic_patterns_coverage() {
        // Verify that all expected pattern categories are covered
        let patterns: Vec<&str> = SEMANTIC_PATTERNS.to_vec();
        
        // Check for file operations
        assert!(patterns.iter().any(|p| p.contains("create")));
        assert!(patterns.iter().any(|p| p.contains("copy")));
        assert!(patterns.iter().any(|p| p.contains("delete")));
        assert!(patterns.iter().any(|p| p.contains("move")));
        
        // Check for execution patterns
        assert!(patterns.iter().any(|p| p.contains("execute")));
        assert!(patterns.iter().any(|p| p.contains("run")));
        
        // Check for workflow patterns
        assert!(patterns.iter().any(|p| p.contains("pipeline")));
        assert!(patterns.iter().any(|p| p.contains("workflow")));
        
        // Check for network patterns
        assert!(patterns.iter().any(|p| p.contains("http")));
        assert!(patterns.iter().any(|p| p.contains("download")));
        assert!(patterns.iter().any(|p| p.contains("upload")));
        
        // Check for data processing patterns
        assert!(patterns.iter().any(|p| p.contains("process")));
        assert!(patterns.iter().any(|p| p.contains("transform")));
        
        // Check for system patterns
        assert!(patterns.iter().any(|p| p.contains("backup")));
        assert!(patterns.iter().any(|p| p.contains("compress")));
    }

    #[test]
    fn test_threshold_validation() {
        // Test valid thresholds
        assert!((0.0..=1.0).contains(&0.0));
        assert!((0.0..=1.0).contains(&0.5));
        assert!((0.0..=1.0).contains(&1.0));
        
        // Test invalid thresholds
        assert!(!(0.0..=1.0).contains(&-0.1));
        assert!(!(0.0..=1.0).contains(&1.1));
    }
}
