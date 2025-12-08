//! Intent classification for natural language understanding
//!
//! This module classifies user intent from natural language using semantic similarity
//! between input embeddings and canonical intent phrase embeddings.

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
use crate::nlp::semantic_types::{Intent, IntentResult};
#[cfg(feature = "tier3")]
use crate::nlp::errors::Tier3Error;

/// Canonical phrases for each intent type
/// These phrases are used to generate embeddings that represent each intent
#[cfg(feature = "tier3")]
const INTENT_PHRASES: &[(Intent, &[&str])] = &[
    (Intent::Create, &[
        "create a new file",
        "make a file",
        "generate files",
        "create new document",
        "build a file",
        "produce files",
    ]),
    (Intent::Copy, &[
        "copy files",
        "duplicate data",
        "copy file to location",
        "replicate files",
        "make a copy",
        "clone files",
    ]),
    (Intent::Move, &[
        "move files",
        "relocate data",
        "transfer files",
        "move file to directory",
        "shift files",
    ]),
    (Intent::Delete, &[
        "delete files",
        "remove data",
        "erase files",
        "delete file from disk",
        "remove files",
        "discard files",
    ]),
    (Intent::Execute, &[
        "execute command",
        "run script",
        "run command",
        "execute program",
        "launch application",
        "start process",
    ]),
    (Intent::Pipeline, &[
        "create pipeline",
        "build data pipeline",
        "setup pipeline workflow",
        "construct pipeline",
        "make pipeline",
    ]),
    (Intent::Workflow, &[
        "build workflow",
        "create workflow",
        "setup workflow",
        "construct workflow",
        "design workflow",
    ]),
    (Intent::Http, &[
        "send http request",
        "make api call",
        "send web request",
        "call http endpoint",
        "make http request",
        "fetch from url",
    ]),
];

/// Classifies user intent from natural language
#[cfg(feature = "tier3")]
pub struct IntentClassifier {
    /// Embeddings for each intent, averaged from canonical phrases
    intent_embeddings: HashMap<Intent, Tensor>,
    /// Confidence threshold for classification
    threshold: f32,
}

#[cfg(feature = "tier3")]
impl IntentClassifier {
    /// Create a new IntentClassifier with the specified confidence threshold
    /// 
    /// This will generate embeddings for all canonical intent phrases using the provided model manager.
    pub fn new(model_manager: Arc<ModelManager>, threshold: f32) -> Result<Self, JormError> {
        if !model_manager.is_loaded() {
            return Err(JormError::from(Tier3Error::ModelNotLoaded(
                "Model must be loaded before creating IntentClassifier".to_string(),
            )));
        }
        
        // Generate embeddings for all intent phrases
        let mut intent_embeddings = HashMap::new();
        
        for (intent, phrases) in INTENT_PHRASES.iter() {
            // Generate embeddings for all phrases for this intent
            let mut phrase_embeddings = Vec::new();
            
            for phrase in *phrases {
                let tokens = model_manager.tokenize(phrase)?;
                let embedding = model_manager.generate_embeddings(&tokens)?;
                phrase_embeddings.push(embedding);
            }
            
            // Average the embeddings to get a single representative embedding for this intent
            let avg_embedding = Self::average_embeddings(&phrase_embeddings)?;
            intent_embeddings.insert(intent.clone(), avg_embedding);
        }
        
        println!("✅ IntentClassifier initialized with {} intent types", intent_embeddings.len());
        
        Ok(Self {
            intent_embeddings,
            threshold,
        })
    }
    
    /// Average multiple embeddings into a single embedding
    fn average_embeddings(embeddings: &[Tensor]) -> Result<Tensor, JormError> {
        if embeddings.is_empty() {
            return Err(JormError::from(Tier3Error::InferenceError(
                "Cannot average empty list of embeddings".to_string(),
            )));
        }
        
        // Sum all embeddings
        let mut sum = embeddings[0].clone();
        for embedding in &embeddings[1..] {
            sum = (sum + embedding).map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to add embeddings: {}",
                    e
                )))
            })?;
        }
        
        // Divide by count to get average
        let count = embeddings.len() as f64;
        let avg = (sum / count).map_err(|e| {
            JormError::from(Tier3Error::InferenceError(format!(
                "Failed to average embeddings: {}",
                e
            )))
        })?;
        
        Ok(avg)
    }

    /// Compute cosine similarity between two embeddings
    fn cosine_similarity(a: &Tensor, b: &Tensor) -> Result<f32, JormError> {
        // Compute dot product
        let dot_product = (a * b)
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to compute dot product: {}",
                    e
                )))
            })?
            .sum_all()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to sum dot product: {}",
                    e
                )))
            })?
            .to_scalar::<f32>()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to convert to scalar: {}",
                    e
                )))
            })?;
        
        // Compute norms
        let norm_a = a
            .sqr()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to square tensor a: {}",
                    e
                )))
            })?
            .sum_all()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to sum squared tensor a: {}",
                    e
                )))
            })?
            .sqrt()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to compute sqrt of norm a: {}",
                    e
                )))
            })?
            .to_scalar::<f32>()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to convert norm a to scalar: {}",
                    e
                )))
            })?;
        
        let norm_b = b
            .sqr()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to square tensor b: {}",
                    e
                )))
            })?
            .sum_all()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to sum squared tensor b: {}",
                    e
                )))
            })?
            .sqrt()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
                    "Failed to compute sqrt of norm b: {}",
                    e
                )))
            })?
            .to_scalar::<f32>()
            .map_err(|e| {
                JormError::from(Tier3Error::InferenceError(format!(
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

    /// Classify a single intent from the input embedding
    /// 
    /// Returns the intent with the highest similarity score above the threshold.
    /// If no intent meets the threshold, returns Intent::Unknown.
    pub fn classify(&self, embedding: &Tensor) -> Result<IntentResult, JormError> {
        let mut best_intent = Intent::Unknown;
        let mut best_confidence = 0.0f32;
        let mut secondary_intents = Vec::new();
        
        // Compare input embedding with each intent embedding
        for (intent, intent_embedding) in &self.intent_embeddings {
            let similarity = Self::cosine_similarity(embedding, intent_embedding)?;
            
            // Track all intents with their similarities for secondary intents
            if similarity > self.threshold && similarity < best_confidence {
                secondary_intents.push((intent.clone(), similarity));
            }
            
            // Update best intent if this one is better
            if similarity > best_confidence {
                // Move previous best to secondary if it was above threshold
                if best_confidence > self.threshold {
                    secondary_intents.push((best_intent.clone(), best_confidence));
                }
                best_intent = intent.clone();
                best_confidence = similarity;
            }
        }
        
        // Sort secondary intents by confidence (descending)
        secondary_intents.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // If best confidence is below threshold, return Unknown
        if best_confidence < self.threshold {
            return Ok(IntentResult {
                intent: Intent::Unknown,
                confidence: best_confidence,
                secondary_intents,
            });
        }
        
        Ok(IntentResult {
            intent: best_intent,
            confidence: best_confidence,
            secondary_intents,
        })
    }

    /// Classify multiple intents from the input embedding
    /// 
    /// Returns all intents that have a similarity score above the threshold,
    /// ranked by confidence (highest first).
    pub fn classify_multi(&self, embedding: &Tensor) -> Result<Vec<IntentResult>, JormError> {
        let mut intent_results = Vec::new();
        
        // Compute similarity for all intents
        for (intent, intent_embedding) in &self.intent_embeddings {
            let similarity = Self::cosine_similarity(embedding, intent_embedding)?;
            
            // Only include intents above threshold
            if similarity >= self.threshold {
                intent_results.push(IntentResult {
                    intent: intent.clone(),
                    confidence: similarity,
                    secondary_intents: Vec::new(), // Not used in multi-intent mode
                });
            }
        }
        
        // Sort by confidence (descending)
        intent_results.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        Ok(intent_results)
    }
}


#[cfg(all(test, feature = "tier3"))]
mod tests {
    use super::*;
    use candle_core::Device;
    
    /// Create a mock embedding tensor for testing
    fn create_test_embedding(values: &[f32]) -> Tensor {
        let device = Device::Cpu;
        Tensor::new(values, &device).unwrap()
    }
    
    /// Create a simple mock intent embeddings map for testing
    fn create_mock_intent_embeddings() -> HashMap<Intent, Tensor> {
        let mut embeddings = HashMap::new();
        
        // Create distinct embeddings for each intent
        embeddings.insert(Intent::Create, create_test_embedding(&[1.0, 0.0, 0.0, 0.0]));
        embeddings.insert(Intent::Copy, create_test_embedding(&[0.0, 1.0, 0.0, 0.0]));
        embeddings.insert(Intent::Delete, create_test_embedding(&[0.0, 0.0, 1.0, 0.0]));
        embeddings.insert(Intent::Execute, create_test_embedding(&[0.0, 0.0, 0.0, 1.0]));
        
        embeddings
    }
    
    #[test]
    fn test_cosine_similarity_identical() {
        let embedding1 = create_test_embedding(&[1.0, 0.0, 0.0]);
        let embedding2 = create_test_embedding(&[1.0, 0.0, 0.0]);
        
        let similarity = IntentClassifier::cosine_similarity(&embedding1, &embedding2).unwrap();
        
        // Identical embeddings should have similarity of 1.0
        assert!((similarity - 1.0).abs() < 0.0001);
    }
    
    #[test]
    fn test_cosine_similarity_orthogonal() {
        let embedding1 = create_test_embedding(&[1.0, 0.0, 0.0]);
        let embedding2 = create_test_embedding(&[0.0, 1.0, 0.0]);
        
        let similarity = IntentClassifier::cosine_similarity(&embedding1, &embedding2).unwrap();
        
        // Orthogonal embeddings should have similarity of 0.0
        assert!(similarity.abs() < 0.0001);
    }
    
    #[test]
    fn test_cosine_similarity_opposite() {
        let embedding1 = create_test_embedding(&[1.0, 0.0, 0.0]);
        let embedding2 = create_test_embedding(&[-1.0, 0.0, 0.0]);
        
        let similarity = IntentClassifier::cosine_similarity(&embedding1, &embedding2).unwrap();
        
        // Opposite embeddings should have similarity of -1.0
        assert!((similarity + 1.0).abs() < 0.0001);
    }
    
    #[test]
    fn test_cosine_similarity_zero_vector() {
        let embedding1 = create_test_embedding(&[1.0, 0.0, 0.0]);
        let embedding2 = create_test_embedding(&[0.0, 0.0, 0.0]);
        
        let similarity = IntentClassifier::cosine_similarity(&embedding1, &embedding2).unwrap();
        
        // Zero vector should return 0.0 similarity
        assert_eq!(similarity, 0.0);
    }
    
    #[test]
    fn test_average_embeddings() {
        let embedding1 = create_test_embedding(&[1.0, 0.0, 0.0]);
        let embedding2 = create_test_embedding(&[0.0, 1.0, 0.0]);
        let embedding3 = create_test_embedding(&[0.0, 0.0, 1.0]);
        
        let embeddings = vec![embedding1, embedding2, embedding3];
        let avg = IntentClassifier::average_embeddings(&embeddings).unwrap();
        
        // Average should be approximately [0.33, 0.33, 0.33]
        let values = avg.to_vec1::<f32>().unwrap();
        assert_eq!(values.len(), 3);
        assert!((values[0] - 0.333).abs() < 0.01);
        assert!((values[1] - 0.333).abs() < 0.01);
        assert!((values[2] - 0.333).abs() < 0.01);
    }
    
    #[test]
    fn test_average_embeddings_empty() {
        let embeddings: Vec<Tensor> = vec![];
        let result = IntentClassifier::average_embeddings(&embeddings);
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_classify_single_intent_exact_match() {
        let classifier = IntentClassifier {
            intent_embeddings: create_mock_intent_embeddings(),
            threshold: 0.7,
        };
        
        // Test with exact Create intent embedding
        let input_embedding = create_test_embedding(&[1.0, 0.0, 0.0, 0.0]);
        let result = classifier.classify(&input_embedding).unwrap();
        
        assert_eq!(result.intent, Intent::Create);
        assert!((result.confidence - 1.0).abs() < 0.0001);
    }
    
    #[test]
    fn test_classify_single_intent_similar() {
        let classifier = IntentClassifier {
            intent_embeddings: create_mock_intent_embeddings(),
            threshold: 0.7,
        };
        
        // Test with embedding similar to Create (but not exact)
        let input_embedding = create_test_embedding(&[0.9, 0.1, 0.0, 0.0]);
        let result = classifier.classify(&input_embedding).unwrap();
        
        assert_eq!(result.intent, Intent::Create);
        assert!(result.confidence > 0.7);
    }
    
    #[test]
    fn test_classify_below_threshold() {
        let classifier = IntentClassifier {
            intent_embeddings: create_mock_intent_embeddings(),
            threshold: 0.9, // High threshold
        };
        
        // Test with embedding that doesn't match well
        let input_embedding = create_test_embedding(&[0.5, 0.5, 0.0, 0.0]);
        let result = classifier.classify(&input_embedding).unwrap();
        
        // Should return Unknown when below threshold
        assert_eq!(result.intent, Intent::Unknown);
    }
    
    #[test]
    fn test_classify_secondary_intents() {
        let classifier = IntentClassifier {
            intent_embeddings: create_mock_intent_embeddings(),
            threshold: 0.5,
        };
        
        // Test with embedding that matches multiple intents
        let input_embedding = create_test_embedding(&[0.7, 0.6, 0.0, 0.0]);
        let result = classifier.classify(&input_embedding).unwrap();
        
        // Should have Create as primary and Copy as secondary
        assert_eq!(result.intent, Intent::Create);
        assert!(!result.secondary_intents.is_empty());
        
        // Secondary intents should be sorted by confidence
        if result.secondary_intents.len() > 1 {
            for i in 0..result.secondary_intents.len() - 1 {
                assert!(result.secondary_intents[i].1 >= result.secondary_intents[i + 1].1);
            }
        }
    }
    
    #[test]
    fn test_classify_multi_all_above_threshold() {
        let classifier = IntentClassifier {
            intent_embeddings: create_mock_intent_embeddings(),
            threshold: 0.5,
        };
        
        // Test with embedding that matches multiple intents
        let input_embedding = create_test_embedding(&[0.8, 0.7, 0.6, 0.0]);
        let results = classifier.classify_multi(&input_embedding).unwrap();
        
        // Should return multiple intents
        assert!(results.len() >= 2);
        
        // Should be sorted by confidence (descending)
        for i in 0..results.len() - 1 {
            assert!(results[i].confidence >= results[i + 1].confidence);
        }
        
        // All should be above threshold
        for result in &results {
            assert!(result.confidence >= 0.5);
        }
    }
    
    #[test]
    fn test_classify_multi_none_above_threshold() {
        let classifier = IntentClassifier {
            intent_embeddings: create_mock_intent_embeddings(),
            threshold: 0.95, // Very high threshold
        };
        
        // Test with embedding that doesn't match well
        let input_embedding = create_test_embedding(&[0.5, 0.5, 0.5, 0.5]);
        let results = classifier.classify_multi(&input_embedding).unwrap();
        
        // Should return empty list when nothing is above threshold
        assert!(results.is_empty());
    }
    
    #[test]
    fn test_classify_multi_single_match() {
        let classifier = IntentClassifier {
            intent_embeddings: create_mock_intent_embeddings(),
            threshold: 0.9,
        };
        
        // Test with embedding that strongly matches only one intent
        let input_embedding = create_test_embedding(&[1.0, 0.0, 0.0, 0.0]);
        let results = classifier.classify_multi(&input_embedding).unwrap();
        
        // Should return exactly one intent
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].intent, Intent::Create);
        assert!((results[0].confidence - 1.0).abs() < 0.0001);
    }
    
    #[test]
    fn test_classify_multi_ranking() {
        let classifier = IntentClassifier {
            intent_embeddings: create_mock_intent_embeddings(),
            threshold: 0.3,
        };
        
        // Test with embedding that has clear ranking
        let input_embedding = create_test_embedding(&[0.9, 0.7, 0.5, 0.3]);
        let results = classifier.classify_multi(&input_embedding).unwrap();
        
        // Should be ranked: Create > Copy > Delete > Execute
        assert!(results.len() >= 3);
        assert_eq!(results[0].intent, Intent::Create);
        assert_eq!(results[1].intent, Intent::Copy);
        
        // Verify descending order
        for i in 0..results.len() - 1 {
            assert!(results[i].confidence >= results[i + 1].confidence);
        }
    }
    
    #[test]
    fn test_classify_ambiguous_intents() {
        let classifier = IntentClassifier {
            intent_embeddings: create_mock_intent_embeddings(),
            threshold: 0.6,
        };
        
        // Test with embedding that's ambiguous between two intents
        let input_embedding = create_test_embedding(&[0.71, 0.70, 0.0, 0.0]);
        let result = classifier.classify(&input_embedding).unwrap();
        
        // Should pick the one with higher confidence
        assert_eq!(result.intent, Intent::Create);
        
        // Should have secondary intent
        assert!(!result.secondary_intents.is_empty());
        assert_eq!(result.secondary_intents[0].0, Intent::Copy);
    }
}
