//! Transformer-based NLP engine (Tier 3)
//!
//! This module orchestrates the complete Tier 3 NLP pipeline, coordinating
//! model management, intent classification, entity extraction, semantic matching,
//! and context analysis.

#[cfg(feature = "tier3")]
use std::sync::Arc;

#[cfg(feature = "tier3")]
use crate::core::error::JormError;
#[cfg(feature = "tier3")]
use crate::nlp::config::Config;
#[cfg(feature = "tier3")]
use crate::nlp::context_analyzer::ContextAnalyzer;
#[cfg(feature = "tier3")]
use crate::nlp::entity_extractor::EntityExtractor;
#[cfg(feature = "tier3")]
use crate::nlp::intent_classifier::IntentClassifier;
#[cfg(feature = "tier3")]
use crate::nlp::model_manager::ModelManager;
#[cfg(feature = "tier3")]
use crate::nlp::semantic_matcher::SemanticMatcher;
#[cfg(feature = "tier3")]
use crate::nlp::semantic_types::{SemanticParse, IntentResult};

/// Main orchestrator for Tier 3 transformer-based NLP
#[cfg(feature = "tier3")]
pub struct TransformerNlpEngine {
    model_manager: Arc<ModelManager>,
    intent_classifier: IntentClassifier,
    entity_extractor: EntityExtractor,
    semantic_matcher: SemanticMatcher,
    context_analyzer: ContextAnalyzer,
    config: Config,
}

#[cfg(feature = "tier3")]
impl TransformerNlpEngine {
    /// Create a new TransformerNlpEngine with the specified configuration
    pub fn new(config: Config) -> Result<Self, JormError> {
        let mut model_manager = ModelManager::new(config.clone())?;
        
        // Load the model before creating components that depend on it
        model_manager.load_model()?;
        
        // Wrap model_manager in Arc for sharing across components
        let model_manager = Arc::new(model_manager);
        
        // Create components that depend on the model manager
        let intent_classifier = IntentClassifier::new(Arc::clone(&model_manager), config.intent_threshold)?;
        let entity_extractor = EntityExtractor::new(config.entity_threshold)?;
        let semantic_matcher = SemanticMatcher::new(Arc::clone(&model_manager), config.similarity_threshold)?;
        let context_analyzer = ContextAnalyzer::new()?;

        Ok(Self {
            model_manager,
            intent_classifier,
            entity_extractor,
            semantic_matcher,
            context_analyzer,
            config,
        })
    }

    /// Process natural language input through the Tier 3 pipeline
    /// 
    /// This method coordinates the complete NLP pipeline:
    /// 1. Preprocessing and tokenization
    /// 2. Embedding generation
    /// 3. Intent classification
    /// 4. Entity extraction
    /// 5. Semantic pattern matching
    /// 6. Context analysis
    /// 7. Task generation
    pub fn process(&self, text: &str) -> Result<SemanticParse, JormError> {
        use std::time::Instant;
        
        let start_time = Instant::now();
        
        // Stage 1: Preprocessing - normalize and clean text
        let preprocessing_start = Instant::now();
        let normalized_text = self.preprocess_text(text);
        let preprocessing_time = preprocessing_start.elapsed();
        
        // Stage 2: Generate embeddings
        let embedding_start = Instant::now();
        let tokens = self.model_manager.tokenize(&normalized_text)?;
        let embedding = self.model_manager.generate_embeddings(&tokens)?;
        let embedding_time = embedding_start.elapsed();
        
        // Stage 3: Classify intent
        let intent_start = Instant::now();
        let intent_results = self.intent_classifier.classify_multi(&embedding)?;
        let intent_time = intent_start.elapsed();
        
        // Stage 4: Extract entities
        let entity_start = Instant::now();
        let extracted_entities = self.entity_extractor.extract(&normalized_text)?;
        let entity_time = entity_start.elapsed();
        
        // Stage 5: Match semantic patterns
        let matching_start = Instant::now();
        let pattern_match = self.semantic_matcher.match_pattern(&normalized_text)?;
        let matching_time = matching_start.elapsed();
        
        // Stage 6: Analyze context
        let context_start = Instant::now();
        let context_analysis = self.context_analyzer.analyze(&normalized_text, &intent_results)?;
        let context_time = context_start.elapsed();
        
        // Stage 7: Generate semantic tasks
        let task_gen_start = Instant::now();
        let tasks = self.generate_semantic_tasks(
            &intent_results,
            &extracted_entities,
            &pattern_match,
            &context_analysis,
        )?;
        let task_gen_time = task_gen_start.elapsed();
        
        // Calculate overall confidence
        let confidence = self.calculate_overall_confidence(
            &intent_results,
            &extracted_entities,
            &pattern_match,
        );
        
        let total_time = start_time.elapsed();
        
        // Log performance metrics
        println!("⏱️  Tier 3 Pipeline Performance:");
        println!("   Preprocessing: {:.2}ms", preprocessing_time.as_secs_f64() * 1000.0);
        println!("   Embedding:     {:.2}ms", embedding_time.as_secs_f64() * 1000.0);
        println!("   Intent:        {:.2}ms", intent_time.as_secs_f64() * 1000.0);
        println!("   Entity:        {:.2}ms", entity_time.as_secs_f64() * 1000.0);
        println!("   Matching:      {:.2}ms", matching_time.as_secs_f64() * 1000.0);
        println!("   Context:       {:.2}ms", context_time.as_secs_f64() * 1000.0);
        println!("   Task Gen:      {:.2}ms", task_gen_time.as_secs_f64() * 1000.0);
        println!("   Total:         {:.2}ms", total_time.as_secs_f64() * 1000.0);
        
        // Check if we exceeded the time limit
        if total_time.as_millis() > self.config.max_total_time_ms as u128 {
            return Err(JormError::from(crate::nlp::errors::Tier3Error::InferenceTimeout(
                total_time.as_millis() as u64,
            )));
        }
        
        Ok(SemanticParse {
            original_text: text.to_string(),
            intents: intent_results,
            entities: extracted_entities,
            context: context_analysis,
            tasks,
            confidence,
        })
    }

    /// Preprocess text by normalizing whitespace and cleaning
    fn preprocess_text(&self, text: &str) -> String {
        // Normalize whitespace
        let normalized = text.trim().split_whitespace().collect::<Vec<_>>().join(" ");
        
        // Convert to lowercase for consistency (embeddings are case-insensitive)
        normalized.to_lowercase()
    }

    /// Generate semantic tasks from the analysis results
    fn generate_semantic_tasks(
        &self,
        intent_results: &[IntentResult],
        extracted_entities: &crate::nlp::semantic_types::ExtractedEntities,
        pattern_match: &Option<crate::nlp::semantic_matcher::MatchResult>,
        context_analysis: &crate::nlp::semantic_types::ContextAnalysis,
    ) -> Result<Vec<crate::nlp::semantic_types::SemanticTask>, JormError> {
        use crate::nlp::semantic_types::SemanticTask;
        
        let mut tasks = Vec::new();
        
        // If we have no intents, we can't generate tasks
        if intent_results.is_empty() {
            return Ok(tasks);
        }
        
        // Generate a task for each intent
        for (idx, intent_result) in intent_results.iter().enumerate() {
            let task_name = format!("task_{}", idx);
            
            // Collect entities relevant to this task
            let task_entities = extracted_entities.entities.clone();
            
            // Determine dependencies based on context analysis
            let dependencies = self.determine_task_dependencies(
                idx,
                intent_results.len(),
                context_analysis,
            );
            
            tasks.push(SemanticTask {
                name: task_name,
                intent: intent_result.intent.clone(),
                entities: task_entities,
                dependencies,
                confidence: intent_result.confidence,
            });
        }
        
        // If we have a pattern match, use it to refine the tasks
        if let Some(match_result) = pattern_match {
            self.refine_tasks_with_pattern(&mut tasks, match_result)?;
        }
        
        Ok(tasks)
    }

    /// Determine dependencies for a task based on context analysis
    fn determine_task_dependencies(
        &self,
        task_index: usize,
        _total_tasks: usize,
        context_analysis: &crate::nlp::semantic_types::ContextAnalysis,
    ) -> Vec<String> {
        use crate::nlp::semantic_types::ExecutionOrder;
        
        let mut dependencies = Vec::new();
        
        // Check implicit dependencies from context analysis
        let task_name = format!("task_{}", task_index);
        for (dependent, dependency) in &context_analysis.implicit_dependencies {
            if dependent == &task_name {
                dependencies.push(dependency.clone());
            }
        }
        
        // If no explicit dependencies and execution is sequential, add previous task
        if dependencies.is_empty() 
            && task_index > 0 
            && matches!(context_analysis.execution_order, ExecutionOrder::Sequential | ExecutionOrder::Mixed)
        {
            dependencies.push(format!("task_{}", task_index - 1));
        }
        
        dependencies
    }

    /// Refine tasks using pattern match information
    fn refine_tasks_with_pattern(
        &self,
        tasks: &mut [crate::nlp::semantic_types::SemanticTask],
        pattern_match: &crate::nlp::semantic_matcher::MatchResult,
    ) -> Result<(), JormError> {
        // Use pattern match parameters to enhance task information
        // This is a basic implementation that can be enhanced
        
        for task in tasks.iter_mut() {
            // If pattern has specific parameters, we could use them to refine entities
            // For now, we just log that we have a pattern match
            if pattern_match.similarity > 0.9 {
                // High confidence pattern match - could boost task confidence
                task.confidence = task.confidence.max(pattern_match.similarity);
            }
        }
        
        Ok(())
    }

    /// Calculate overall confidence from all analysis components
    fn calculate_overall_confidence(
        &self,
        intent_results: &[IntentResult],
        extracted_entities: &crate::nlp::semantic_types::ExtractedEntities,
        pattern_match: &Option<crate::nlp::semantic_matcher::MatchResult>,
    ) -> f32 {
        let mut confidence_scores = Vec::new();
        
        // Intent confidence (average of all intents)
        if !intent_results.is_empty() {
            let intent_avg = intent_results.iter()
                .map(|r| r.confidence)
                .sum::<f32>() / intent_results.len() as f32;
            confidence_scores.push(intent_avg);
        }
        
        // Entity confidence (average of all entities)
        if !extracted_entities.entities.is_empty() {
            let entity_avg = extracted_entities.confidence.values()
                .sum::<f32>() / extracted_entities.confidence.len() as f32;
            confidence_scores.push(entity_avg);
        }
        
        // Pattern match confidence
        if let Some(match_result) = pattern_match {
            confidence_scores.push(match_result.similarity);
        }
        
        // Calculate weighted average (or just average if no weights)
        if confidence_scores.is_empty() {
            0.0
        } else {
            confidence_scores.iter().sum::<f32>() / confidence_scores.len() as f32
        }
    }

    /// Check if the engine is available and ready to use
    pub fn is_available(&self) -> bool {
        self.config.enabled && self.model_manager.is_loaded()
    }
}
