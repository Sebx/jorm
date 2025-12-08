//! Entity extraction from natural language
//!
//! This module extracts entities (file names, numbers, paths, commands, etc.)
//! from natural language using a hybrid approach of pattern matching and semantic understanding.

#[cfg(feature = "tier3")]
use std::collections::HashMap;
#[cfg(feature = "tier3")]
use std::sync::Arc;

#[cfg(feature = "tier3")]
use regex::Regex;

#[cfg(feature = "tier3")]
use crate::core::error::JormError;
#[cfg(feature = "tier3")]
use crate::nlp::semantic_types::{Entity, ExtractedEntities};
#[cfg(feature = "tier3")]
use crate::nlp::model_manager::ModelManager;

/// Extracts entities from natural language
#[cfg(feature = "tier3")]
pub struct EntityExtractor {
    threshold: f32,
    model_manager: Option<Arc<ModelManager>>,
}

#[cfg(feature = "tier3")]
impl EntityExtractor {
    /// Create a new EntityExtractor with the specified confidence threshold
    pub fn new(threshold: f32) -> Result<Self, JormError> {
        Ok(Self { 
            threshold,
            model_manager: None,
        })
    }

    /// Create a new EntityExtractor with model manager for semantic extraction
    pub fn with_model_manager(threshold: f32, model_manager: Arc<ModelManager>) -> Result<Self, JormError> {
        Ok(Self {
            threshold,
            model_manager: Some(model_manager),
        })
    }

    /// Extract entities from the input text
    pub fn extract(&self, text: &str) -> Result<ExtractedEntities, JormError> {
        let mut entities = Vec::new();
        let mut confidence = HashMap::new();
        let mut entity_index = 0;

        // Pattern-based extraction
        self.extract_numbers_with_units(text, &mut entities, &mut confidence, &mut entity_index);
        self.extract_urls(text, &mut entities, &mut confidence, &mut entity_index);
        self.extract_commands(text, &mut entities, &mut confidence, &mut entity_index);
        self.extract_file_paths(text, &mut entities, &mut confidence, &mut entity_index);

        // Semantic extraction (if model manager is available)
        if let Some(_model_manager) = &self.model_manager {
            self.extract_semantic_entities(text, &mut entities, &mut confidence, &mut entity_index)?;
        }

        // Combine confidence scores for duplicate entities
        self.combine_duplicate_confidences(&mut entities, &mut confidence);

        // Filter entities below threshold
        let filtered_results = self.filter_by_confidence(entities, confidence);

        Ok(filtered_results)
    }

    /// Combine confidence scores for duplicate entities
    fn combine_duplicate_confidences(
        &self,
        entities: &mut Vec<Entity>,
        confidence: &mut HashMap<usize, f32>,
    ) {
        let mut seen: HashMap<String, (usize, f32)> = HashMap::new();
        let mut to_remove = Vec::new();

        for (idx, entity) in entities.iter().enumerate() {
            let entity_key = self.entity_to_key(entity);
            let current_confidence = confidence.get(&idx).copied().unwrap_or(0.5);

            if let Some((existing_idx, existing_confidence)) = seen.get(&entity_key) {
                // Combine confidences: take the maximum or average
                let combined = self.combine_confidence_scores(*existing_confidence, current_confidence);
                confidence.insert(*existing_idx, combined);
                to_remove.push(idx);
            } else {
                seen.insert(entity_key, (idx, current_confidence));
            }
        }

        // Remove duplicates (in reverse order to maintain indices)
        for idx in to_remove.iter().rev() {
            entities.remove(*idx);
            confidence.remove(idx);
        }

        // Rebuild confidence map with correct indices
        let mut new_confidence = HashMap::new();
        for (new_idx, entity) in entities.iter().enumerate() {
            let entity_key = self.entity_to_key(entity);
            if let Some((old_idx, _)) = seen.get(&entity_key) {
                if let Some(&conf) = confidence.get(old_idx) {
                    new_confidence.insert(new_idx, conf);
                }
            }
        }
        *confidence = new_confidence;
    }

    /// Convert entity to a unique key for deduplication
    fn entity_to_key(&self, entity: &Entity) -> String {
        match entity {
            Entity::FileName(name) => format!("file:{}", name),
            Entity::FilePath(path) => format!("path:{}", path),
            Entity::Number { value, unit } => {
                format!("num:{}:{}", value, unit.as_ref().unwrap_or(&String::new()))
            }
            Entity::Command(cmd) => format!("cmd:{}", cmd),
            Entity::Url(url) => format!("url:{}", url),
            Entity::Variable(var) => format!("var:{}", var),
        }
    }

    /// Combine two confidence scores (using weighted average favoring higher confidence)
    fn combine_confidence_scores(&self, conf1: f32, conf2: f32) -> f32 {
        // Use maximum confidence when combining pattern and semantic matches
        conf1.max(conf2)
    }

    /// Filter entities by confidence threshold
    fn filter_by_confidence(
        &self,
        entities: Vec<Entity>,
        confidence: HashMap<usize, f32>,
    ) -> ExtractedEntities {
        let mut filtered_entities = Vec::new();
        let mut filtered_confidence = HashMap::new();

        for (idx, entity) in entities.into_iter().enumerate() {
            let conf = confidence.get(&idx).copied().unwrap_or(0.5);
            
            if conf >= self.threshold {
                let new_idx = filtered_entities.len();
                filtered_entities.push(entity);
                filtered_confidence.insert(new_idx, conf);
            }
        }

        ExtractedEntities {
            entities: filtered_entities,
            confidence: filtered_confidence,
        }
    }

    /// Extract numbers with units (e.g., "5 files", "10MB")
    fn extract_numbers_with_units(
        &self,
        text: &str,
        entities: &mut Vec<Entity>,
        confidence: &mut HashMap<usize, f32>,
        entity_index: &mut usize,
    ) {
        let pattern = Regex::new(r"(?i)(\d+)\s*(kb|mb|gb|tb|bytes?|times?|count|files?|copies|items?)").unwrap();
        
        for cap in pattern.captures_iter(text) {
            if let Some(number_str) = cap.get(1) {
                if let Ok(value) = number_str.as_str().parse::<u32>() {
                    let unit = cap.get(2).map(|m| m.as_str().to_lowercase());
                    entities.push(Entity::Number { value, unit });
                    confidence.insert(*entity_index, 0.95); // High confidence for pattern match
                    *entity_index += 1;
                }
            }
        }
    }

    /// Extract URLs from text
    fn extract_urls(
        &self,
        text: &str,
        entities: &mut Vec<Entity>,
        confidence: &mut HashMap<usize, f32>,
        entity_index: &mut usize,
    ) {
        let pattern = Regex::new(r"https?://[^\s]+").unwrap();
        
        for cap in pattern.captures_iter(text) {
            if let Some(url_match) = cap.get(0) {
                entities.push(Entity::Url(url_match.as_str().to_string()));
                confidence.insert(*entity_index, 0.98); // Very high confidence for URL pattern
                *entity_index += 1;
            }
        }
    }

    /// Extract commands from quoted strings
    fn extract_commands(
        &self,
        text: &str,
        entities: &mut Vec<Entity>,
        confidence: &mut HashMap<usize, f32>,
        entity_index: &mut usize,
    ) {
        let pattern = Regex::new(r#"['"]([^'"]+)['"]|`([^`]+)`"#).unwrap();
        
        for cap in pattern.captures_iter(text) {
            // Try both capture groups (single/double quotes and backticks)
            let command = cap.get(1).or_else(|| cap.get(2));
            if let Some(cmd_match) = command {
                entities.push(Entity::Command(cmd_match.as_str().to_string()));
                confidence.insert(*entity_index, 0.90); // High confidence for quoted commands
                *entity_index += 1;
            }
        }
    }

    /// Extract file paths from text
    fn extract_file_paths(
        &self,
        text: &str,
        entities: &mut Vec<Entity>,
        confidence: &mut HashMap<usize, f32>,
        entity_index: &mut usize,
    ) {
        let pattern = Regex::new(r"(?:[./\\][\w/\\.\\-]+|[\w.-]+\.\w+|[A-Za-z]:[/\\][\w/\\.\\-]+)").unwrap();
        
        for cap in pattern.captures_iter(text) {
            if let Some(path_match) = cap.get(0) {
                let path_str = path_match.as_str();
                
                // Determine if it's a file name or full path
                if path_str.contains('/') || path_str.contains('\\') || path_str.starts_with('.') {
                    entities.push(Entity::FilePath(path_str.to_string()));
                    confidence.insert(*entity_index, 0.85); // Good confidence for path patterns
                } else if path_str.contains('.') {
                    // Likely a file name with extension
                    entities.push(Entity::FileName(path_str.to_string()));
                    confidence.insert(*entity_index, 0.80); // Moderate confidence for file names
                } else {
                    // Could be a file name without extension
                    entities.push(Entity::FileName(path_str.to_string()));
                    confidence.insert(*entity_index, 0.70); // Lower confidence without extension
                }
                *entity_index += 1;
            }
        }
    }

    /// Extract entities using semantic understanding (context-dependent)
    fn extract_semantic_entities(
        &self,
        text: &str,
        entities: &mut Vec<Entity>,
        confidence: &mut HashMap<usize, f32>,
        entity_index: &mut usize,
    ) -> Result<(), JormError> {
        let model_manager = match &self.model_manager {
            Some(mm) => mm,
            None => return Ok(()), // No model manager, skip semantic extraction
        };

        // Only proceed if model is loaded
        if !model_manager.is_loaded() {
            return Ok(());
        }

        // Extract file names from context using semantic patterns
        self.extract_semantic_file_names(text, model_manager, entities, confidence, entity_index)?;
        
        // Extract variable names from context
        self.extract_semantic_variables(text, entities, confidence, entity_index)?;

        Ok(())
    }

    /// Extract file names using semantic context
    fn extract_semantic_file_names(
        &self,
        text: &str,
        model_manager: &Arc<ModelManager>,
        entities: &mut Vec<Entity>,
        confidence: &mut HashMap<usize, f32>,
        entity_index: &mut usize,
    ) -> Result<(), JormError> {
        // Look for words that appear in file-related contexts
        // Common patterns: "create <filename>", "copy <filename>", "file called <filename>"
        let file_context_patterns = [
            r"(?i)create\s+(?:a\s+)?(?:file\s+)?(?:named\s+)?(\w+(?:\.\w+)?)",
            r"(?i)copy\s+(?:the\s+)?(?:file\s+)?(\w+(?:\.\w+)?)",
            r"(?i)file\s+(?:named|called)\s+(\w+(?:\.\w+)?)",
            r"(?i)(?:to|into)\s+(?:a\s+)?(?:file\s+)?(\w+(?:\.\w+)?)",
        ];

        for pattern_str in &file_context_patterns {
            let pattern = Regex::new(pattern_str).map_err(|e| {
                JormError::NlpError(format!("Invalid regex pattern: {}", e))
            })?;

            for cap in pattern.captures_iter(text) {
                if let Some(filename_match) = cap.get(1) {
                    let filename = filename_match.as_str().to_string();
                    
                    // Skip if already extracted by pattern matching
                    if entities.iter().any(|e| matches!(e, Entity::FileName(f) if f == &filename)) {
                        continue;
                    }

                    // Use embeddings to verify this is actually a file name in context
                    let context_confidence = self.compute_semantic_confidence(
                        text,
                        &filename,
                        "file name",
                        model_manager,
                    )?;

                    if context_confidence >= self.threshold {
                        entities.push(Entity::FileName(filename));
                        confidence.insert(*entity_index, context_confidence);
                        *entity_index += 1;
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract variable names using semantic context
    fn extract_semantic_variables(
        &self,
        text: &str,
        entities: &mut Vec<Entity>,
        confidence: &mut HashMap<usize, f32>,
        entity_index: &mut usize,
    ) -> Result<(), JormError> {
        // Look for variable-like patterns in programming contexts
        let variable_patterns = [
            r"(?i)variable\s+(?:named\s+)?(\w+)",
            r"(?i)set\s+(\w+)\s+to",
            r"(?i)(\w+)\s*=\s*",
        ];

        for pattern_str in &variable_patterns {
            let pattern = Regex::new(pattern_str).map_err(|e| {
                JormError::NlpError(format!("Invalid regex pattern: {}", e))
            })?;

            for cap in pattern.captures_iter(text) {
                if let Some(var_match) = cap.get(1) {
                    let var_name = var_match.as_str().to_string();
                    
                    // Skip common words that aren't variables
                    if Self::is_common_word(&var_name) {
                        continue;
                    }

                    // Skip if already extracted
                    if entities.iter().any(|e| matches!(e, Entity::Variable(v) if v == &var_name)) {
                        continue;
                    }

                    entities.push(Entity::Variable(var_name));
                    confidence.insert(*entity_index, 0.75); // Moderate confidence for variables
                    *entity_index += 1;
                }
            }
        }

        Ok(())
    }

    /// Compute semantic confidence using embeddings
    fn compute_semantic_confidence(
        &self,
        text: &str,
        entity: &str,
        entity_type: &str,
        model_manager: &Arc<ModelManager>,
    ) -> Result<f32, JormError> {
        // Create context strings for comparison
        let context_with_entity = format!("{} is a {}", entity, entity_type);
        
        // Tokenize and generate embeddings
        let text_tokens = model_manager.tokenize(text)?;
        let context_tokens = model_manager.tokenize(&context_with_entity)?;
        
        let text_embedding = model_manager.generate_embeddings(&text_tokens)?;
        let context_embedding = model_manager.generate_embeddings(&context_tokens)?;
        
        // Compute cosine similarity
        let similarity = Self::cosine_similarity(&text_embedding, &context_embedding)?;
        
        // Convert similarity to confidence (clamp to [0, 1])
        Ok(similarity.max(0.0).min(1.0))
    }

    /// Compute cosine similarity between two embeddings
    fn cosine_similarity(
        a: &candle_core::Tensor,
        b: &candle_core::Tensor,
    ) -> Result<f32, JormError> {
        
        // Compute dot product
        let dot_product = (a * b)
            .map_err(|e| JormError::NlpError(format!("Failed to multiply tensors: {}", e)))?
            .sum_all()
            .map_err(|e| JormError::NlpError(format!("Failed to sum tensor: {}", e)))?
            .to_scalar::<f32>()
            .map_err(|e| JormError::NlpError(format!("Failed to convert to scalar: {}", e)))?;
        
        // Compute norms
        let norm_a = a
            .sqr()
            .map_err(|e| JormError::NlpError(format!("Failed to square tensor: {}", e)))?
            .sum_all()
            .map_err(|e| JormError::NlpError(format!("Failed to sum tensor: {}", e)))?
            .sqrt()
            .map_err(|e| JormError::NlpError(format!("Failed to sqrt tensor: {}", e)))?
            .to_scalar::<f32>()
            .map_err(|e| JormError::NlpError(format!("Failed to convert to scalar: {}", e)))?;
        
        let norm_b = b
            .sqr()
            .map_err(|e| JormError::NlpError(format!("Failed to square tensor: {}", e)))?
            .sum_all()
            .map_err(|e| JormError::NlpError(format!("Failed to sum tensor: {}", e)))?
            .sqrt()
            .map_err(|e| JormError::NlpError(format!("Failed to sqrt tensor: {}", e)))?
            .to_scalar::<f32>()
            .map_err(|e| JormError::NlpError(format!("Failed to convert to scalar: {}", e)))?;
        
        // Avoid division by zero
        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }
        
        Ok(dot_product / (norm_a * norm_b))
    }

    /// Check if a word is a common word (not likely a variable name)
    fn is_common_word(word: &str) -> bool {
        let common_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "from", "as", "is", "was", "are", "were", "be",
            "been", "being", "have", "has", "had", "do", "does", "did", "will",
            "would", "should", "could", "may", "might", "must", "can", "this",
            "that", "these", "those", "it", "its", "they", "them", "their",
        ];
        
        common_words.contains(&word.to_lowercase().as_str())
    }
}

#[cfg(all(test, feature = "tier3"))]
mod tests {
    use super::*;

    #[test]
    fn test_entity_extractor_creation() {
        let extractor = EntityExtractor::new(0.6);
        assert!(extractor.is_ok());
        
        let extractor = extractor.unwrap();
        assert_eq!(extractor.threshold, 0.6);
    }

    #[test]
    fn test_extract_numbers_with_units() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "Create 5 files and copy them 3 times";
        let result = extractor.extract(text).unwrap();
        
        // Should extract "5 files" and "3 times"
        assert_eq!(result.entities.len(), 2);
        
        // Check first entity
        match &result.entities[0] {
            Entity::Number { value, unit } => {
                assert_eq!(*value, 5);
                assert_eq!(unit.as_ref().unwrap(), "files");
            }
            _ => panic!("Expected Number entity"),
        }
        
        // Check second entity
        match &result.entities[1] {
            Entity::Number { value, unit } => {
                assert_eq!(*value, 3);
                assert_eq!(unit.as_ref().unwrap(), "times");
            }
            _ => panic!("Expected Number entity"),
        }
    }

    #[test]
    fn test_extract_numbers_with_byte_units() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "Create a file of 10MB and another of 512KB";
        let result = extractor.extract(text).unwrap();
        
        assert_eq!(result.entities.len(), 2);
        
        match &result.entities[0] {
            Entity::Number { value, unit } => {
                assert_eq!(*value, 10);
                assert_eq!(unit.as_ref().unwrap(), "mb");
            }
            _ => panic!("Expected Number entity"),
        }
        
        match &result.entities[1] {
            Entity::Number { value, unit } => {
                assert_eq!(*value, 512);
                assert_eq!(unit.as_ref().unwrap(), "kb");
            }
            _ => panic!("Expected Number entity"),
        }
    }

    #[test]
    fn test_extract_file_names() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "Copy test.txt to backup.log";
        let result = extractor.extract(text).unwrap();
        
        // Should extract both file names
        assert!(result.entities.len() >= 2);
        
        let file_names: Vec<String> = result
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::FileName(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        
        assert!(file_names.contains(&"test.txt".to_string()));
        assert!(file_names.contains(&"backup.log".to_string()));
    }

    #[test]
    fn test_extract_file_paths() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "Copy files to /home/user/backup and ./local/dir";
        let result = extractor.extract(text).unwrap();
        
        let paths: Vec<String> = result
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::FilePath(path) => Some(path.clone()),
                _ => None,
            })
            .collect();
        
        assert!(paths.contains(&"/home/user/backup".to_string()));
        assert!(paths.contains(&"./local/dir".to_string()));
    }

    #[test]
    fn test_extract_urls() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "Download from https://example.com/file.zip and http://test.org/data";
        let result = extractor.extract(text).unwrap();
        
        let urls: Vec<String> = result
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::Url(url) => Some(url.clone()),
                _ => None,
            })
            .collect();
        
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://example.com/file.zip".to_string()));
        assert!(urls.contains(&"http://test.org/data".to_string()));
    }

    #[test]
    fn test_extract_commands() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "Run 'ls -la' and then execute \"echo hello\"";
        let result = extractor.extract(text).unwrap();
        
        let commands: Vec<String> = result
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::Command(cmd) => Some(cmd.clone()),
                _ => None,
            })
            .collect();
        
        assert_eq!(commands.len(), 2);
        assert!(commands.contains(&"ls -la".to_string()));
        assert!(commands.contains(&"echo hello".to_string()));
    }

    #[test]
    fn test_extract_commands_with_backticks() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "Execute `npm install` to setup";
        let result = extractor.extract(text).unwrap();
        
        let commands: Vec<String> = result
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::Command(cmd) => Some(cmd.clone()),
                _ => None,
            })
            .collect();
        
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], "npm install");
    }

    #[test]
    fn test_confidence_scoring() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "Create 5 files at https://example.com";
        let result = extractor.extract(text).unwrap();
        
        // All extracted entities should have confidence scores
        for idx in 0..result.entities.len() {
            assert!(result.confidence.contains_key(&idx));
            let conf = result.confidence[&idx];
            assert!(conf >= 0.0 && conf <= 1.0);
        }
    }

    #[test]
    fn test_confidence_threshold_filtering() {
        // High threshold should filter out low-confidence entities
        let extractor = EntityExtractor::new(0.95).unwrap();
        
        let text = "Create 5 files"; // Number with unit has 0.95 confidence
        let result = extractor.extract(text).unwrap();
        
        // Should extract the number (high confidence)
        assert!(result.entities.len() >= 1);
        
        // All remaining entities should have confidence >= threshold
        for idx in 0..result.entities.len() {
            let conf = result.confidence[&idx];
            assert!(conf >= 0.95);
        }
    }

    #[test]
    fn test_mixed_entity_extraction() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "Download 10MB from https://example.com/data.zip and run 'extract.sh'";
        let result = extractor.extract(text).unwrap();
        
        // Should extract: number with unit, URL, file name, command
        assert!(result.entities.len() >= 3);
        
        // Check we have different entity types
        let has_number = result.entities.iter().any(|e| matches!(e, Entity::Number { .. }));
        let has_url = result.entities.iter().any(|e| matches!(e, Entity::Url(_)));
        let has_command = result.entities.iter().any(|e| matches!(e, Entity::Command(_)));
        
        assert!(has_number);
        assert!(has_url);
        assert!(has_command);
    }

    #[test]
    fn test_empty_text() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let result = extractor.extract("").unwrap();
        assert_eq!(result.entities.len(), 0);
    }

    #[test]
    fn test_no_entities() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "This is just plain text with no entities";
        let result = extractor.extract(text).unwrap();
        
        // Might extract some file-like words, but should be minimal
        assert!(result.entities.len() <= 2);
    }

    #[test]
    fn test_entity_to_key() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let file_entity = Entity::FileName("test.txt".to_string());
        let key = extractor.entity_to_key(&file_entity);
        assert_eq!(key, "file:test.txt");
        
        let num_entity = Entity::Number {
            value: 5,
            unit: Some("files".to_string()),
        };
        let key = extractor.entity_to_key(&num_entity);
        assert_eq!(key, "num:5:files");
        
        let url_entity = Entity::Url("https://example.com".to_string());
        let key = extractor.entity_to_key(&url_entity);
        assert_eq!(key, "url:https://example.com");
    }

    #[test]
    fn test_combine_confidence_scores() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        // Should take maximum
        let combined = extractor.combine_confidence_scores(0.8, 0.9);
        assert_eq!(combined, 0.9);
        
        let combined = extractor.combine_confidence_scores(0.95, 0.7);
        assert_eq!(combined, 0.95);
    }

    #[test]
    fn test_is_common_word() {
        assert!(EntityExtractor::is_common_word("the"));
        assert!(EntityExtractor::is_common_word("and"));
        assert!(EntityExtractor::is_common_word("is"));
        assert!(EntityExtractor::is_common_word("THE")); // Case insensitive
        
        assert!(!EntityExtractor::is_common_word("myVariable"));
        assert!(!EntityExtractor::is_common_word("counter"));
        assert!(!EntityExtractor::is_common_word("data"));
    }

    #[test]
    fn test_windows_paths() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        let text = "Copy to C:\\Users\\test\\file.txt";
        let result = extractor.extract(text).unwrap();
        
        let paths: Vec<String> = result
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::FilePath(path) => Some(path.clone()),
                Entity::FileName(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        
        assert!(paths.len() > 0);
    }

    #[test]
    fn test_case_insensitive_patterns() {
        let extractor = EntityExtractor::new(0.5).unwrap();
        
        // Test case insensitivity for units
        let text = "Create 5 FILES and 10 MB";
        let result = extractor.extract(text).unwrap();
        
        let numbers: Vec<u32> = result
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::Number { value, .. } => Some(*value),
                _ => None,
            })
            .collect();
        
        assert!(numbers.contains(&5));
        assert!(numbers.contains(&10));
    }
}
