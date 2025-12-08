//! Context analysis for understanding task relationships
//!
//! This module analyzes the context of natural language input to infer relationships
//! between tasks, including sequential, parallel, and conditional dependencies.

#[cfg(feature = "tier3")]
use crate::core::error::JormError;
#[cfg(feature = "tier3")]
use crate::nlp::semantic_types::{ContextAnalysis, ExecutionOrder, IntentResult, TaskRelationship};
#[cfg(feature = "tier3")]
use regex::Regex;

/// Keywords that indicate temporal/sequential relationships
#[cfg(feature = "tier3")]
const TEMPORAL_KEYWORDS: &[&str] = &["then", "after", "before", "while", "next", "following", "subsequently"];

/// Keywords that indicate logical relationships
#[cfg(feature = "tier3")]
const PARALLEL_KEYWORDS: &[&str] = &["and", "also", "plus", "additionally", "simultaneously"];

/// Keywords that indicate conditional relationships
#[cfg(feature = "tier3")]
const CONDITIONAL_KEYWORDS: &[&str] = &["if", "when", "unless", "provided", "assuming"];

/// Keywords that indicate alternative relationships
#[cfg(feature = "tier3")]
const ALTERNATIVE_KEYWORDS: &[&str] = &["or", "either", "alternatively"];

/// Analyzes context and relationships between tasks
#[cfg(feature = "tier3")]
pub struct ContextAnalyzer {
    temporal_regex: Regex,
    parallel_regex: Regex,
    conditional_regex: Regex,
    alternative_regex: Regex,
}

#[cfg(feature = "tier3")]
impl ContextAnalyzer {
    /// Create a new ContextAnalyzer
    pub fn new() -> Result<Self, JormError> {
        // Build regex patterns for keyword detection
        let temporal_pattern = format!(r"\b({})\b", TEMPORAL_KEYWORDS.join("|"));
        let parallel_pattern = format!(r"\b({})\b", PARALLEL_KEYWORDS.join("|"));
        let conditional_pattern = format!(r"\b({})\b", CONDITIONAL_KEYWORDS.join("|"));
        let alternative_pattern = format!(r"\b({})\b", ALTERNATIVE_KEYWORDS.join("|"));

        Ok(Self {
            temporal_regex: Regex::new(&temporal_pattern)
                .map_err(|e| JormError::NlpError(format!("Failed to compile temporal regex: {}", e)))?,
            parallel_regex: Regex::new(&parallel_pattern)
                .map_err(|e| JormError::NlpError(format!("Failed to compile parallel regex: {}", e)))?,
            conditional_regex: Regex::new(&conditional_pattern)
                .map_err(|e| JormError::NlpError(format!("Failed to compile conditional regex: {}", e)))?,
            alternative_regex: Regex::new(&alternative_pattern)
                .map_err(|e| JormError::NlpError(format!("Failed to compile alternative regex: {}", e)))?,
        })
    }

    /// Analyze the context of the input text
    pub fn analyze(
        &self,
        text: &str,
        intents: &[IntentResult],
    ) -> Result<ContextAnalysis, JormError> {
        let text_lower = text.to_lowercase();
        
        // Detect temporal keywords
        let has_temporal = self.detect_temporal_keywords(&text_lower);
        
        // Detect logical keywords
        let (has_parallel, has_conditional, has_alternative) = self.detect_logical_keywords(&text_lower);
        
        // Determine task relationships based on detected keywords
        let task_relationships = self.infer_task_relationships(
            has_temporal,
            has_parallel,
            has_conditional,
            has_alternative,
            intents.len(),
        );
        
        // Determine execution order
        let execution_order = self.determine_execution_order(
            has_temporal,
            has_parallel,
            has_conditional,
            &task_relationships,
        );
        
        // Infer implicit dependencies
        let implicit_dependencies = self.infer_dependencies(
            &text_lower,
            intents,
            has_temporal,
            has_parallel,
        );
        
        Ok(ContextAnalysis {
            task_relationships,
            execution_order,
            implicit_dependencies,
        })
    }

    /// Detect temporal keywords that indicate sequential relationships
    fn detect_temporal_keywords(&self, text: &str) -> bool {
        self.temporal_regex.is_match(text)
    }

    /// Detect logical keywords (parallel, conditional, alternative)
    fn detect_logical_keywords(&self, text: &str) -> (bool, bool, bool) {
        let has_parallel = self.parallel_regex.is_match(text);
        let has_conditional = self.conditional_regex.is_match(text);
        let has_alternative = self.alternative_regex.is_match(text);
        
        (has_parallel, has_conditional, has_alternative)
    }

    /// Infer task relationships based on detected keywords
    fn infer_task_relationships(
        &self,
        has_temporal: bool,
        has_parallel: bool,
        has_conditional: bool,
        _has_alternative: bool,
        intent_count: usize,
    ) -> Vec<TaskRelationship> {
        let mut relationships = Vec::new();
        
        // If we have multiple intents, infer relationships
        if intent_count > 1 {
            // Conditional takes precedence
            if has_conditional {
                relationships.push(TaskRelationship::Conditional);
            }
            // Then temporal/sequential
            else if has_temporal {
                relationships.push(TaskRelationship::Sequential);
            }
            // Then parallel
            else if has_parallel {
                relationships.push(TaskRelationship::Parallel);
            }
            // Default to sequential for multiple tasks
            else {
                relationships.push(TaskRelationship::Sequential);
            }
        }
        
        relationships
    }

    /// Determine the overall execution order
    fn determine_execution_order(
        &self,
        has_temporal: bool,
        has_parallel: bool,
        has_conditional: bool,
        relationships: &[TaskRelationship],
    ) -> ExecutionOrder {
        // If we have both sequential and parallel indicators, it's mixed
        if (has_temporal || has_conditional) && has_parallel {
            return ExecutionOrder::Mixed;
        }
        
        // Check relationships
        if relationships.is_empty() {
            return ExecutionOrder::Sequential;
        }
        
        let has_sequential = relationships.iter().any(|r| matches!(r, TaskRelationship::Sequential | TaskRelationship::Conditional));
        let has_parallel_rel = relationships.iter().any(|r| matches!(r, TaskRelationship::Parallel));
        
        if has_sequential && has_parallel_rel {
            ExecutionOrder::Mixed
        } else if has_parallel_rel {
            ExecutionOrder::Parallel
        } else {
            ExecutionOrder::Sequential
        }
    }

    /// Infer implicit dependencies between tasks
    fn infer_dependencies(
        &self,
        text: &str,
        intents: &[IntentResult],
        has_temporal: bool,
        has_parallel: bool,
    ) -> Vec<(String, String)> {
        let mut dependencies = Vec::new();
        
        // If we have temporal keywords and multiple intents, create sequential dependencies
        if has_temporal && intents.len() > 1 {
            for i in 0..intents.len() - 1 {
                let task_name = format!("task_{}", i);
                let dependent_task = format!("task_{}", i + 1);
                dependencies.push((dependent_task, task_name));
            }
        }
        // If no parallel keywords and multiple intents, assume sequential by default
        else if !has_parallel && intents.len() > 1 {
            // Check for specific patterns that suggest dependencies
            if self.has_dependency_pattern(text) {
                for i in 0..intents.len() - 1 {
                    let task_name = format!("task_{}", i);
                    let dependent_task = format!("task_{}", i + 1);
                    dependencies.push((dependent_task, task_name));
                }
            }
        }
        
        dependencies
    }

    /// Check if text contains patterns that suggest dependencies
    fn has_dependency_pattern(&self, text: &str) -> bool {
        // Patterns that suggest one task depends on another
        let dependency_patterns = [
            "them",      // "create files and copy them"
            "it",        // "create file and execute it"
            "result",    // "run command and use result"
            "output",    // "generate output and process output"
            "to",        // "create file to backup"
        ];
        
        dependency_patterns.iter().any(|pattern| text.contains(pattern))
    }
}

#[cfg(feature = "tier3")]
impl Default for ContextAnalyzer {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(all(test, feature = "tier3"))]
mod tests {
    use super::*;
    use crate::nlp::semantic_types::Intent;

    fn create_test_intent(intent: Intent, confidence: f32) -> IntentResult {
        IntentResult {
            intent,
            confidence,
            secondary_intents: Vec::new(),
        }
    }

    #[test]
    fn test_temporal_keyword_detection() {
        let analyzer = ContextAnalyzer::new().unwrap();
        
        // Test temporal keywords
        assert!(analyzer.detect_temporal_keywords("create file then copy it"));
        assert!(analyzer.detect_temporal_keywords("after creating, delete it"));
        assert!(analyzer.detect_temporal_keywords("before running, compile"));
        assert!(analyzer.detect_temporal_keywords("while processing, log"));
        
        // Test non-temporal
        assert!(!analyzer.detect_temporal_keywords("create and copy files"));
    }

    #[test]
    fn test_logical_keyword_detection() {
        let analyzer = ContextAnalyzer::new().unwrap();
        
        // Test parallel keywords
        let (has_parallel, has_conditional, _) = analyzer.detect_logical_keywords("create and copy files");
        assert!(has_parallel);
        assert!(!has_conditional);
        
        // Test conditional keywords
        let (has_parallel, has_conditional, _) = analyzer.detect_logical_keywords("if file exists, delete it");
        assert!(!has_parallel);
        assert!(has_conditional);
        
        // Test both
        let (has_parallel, has_conditional, _) = analyzer.detect_logical_keywords("if ready, create and copy");
        assert!(has_parallel);
        assert!(has_conditional);
    }

    #[test]
    fn test_sequential_task_detection() {
        let analyzer = ContextAnalyzer::new().unwrap();
        let intents = vec![
            create_test_intent(Intent::Create, 0.9),
            create_test_intent(Intent::Copy, 0.85),
        ];
        
        let result = analyzer.analyze("create file then copy it", &intents).unwrap();
        
        assert_eq!(result.execution_order, ExecutionOrder::Sequential);
        assert!(!result.task_relationships.is_empty());
        assert!(matches!(result.task_relationships[0], TaskRelationship::Sequential));
        assert!(!result.implicit_dependencies.is_empty());
    }

    #[test]
    fn test_parallel_task_detection() {
        let analyzer = ContextAnalyzer::new().unwrap();
        let intents = vec![
            create_test_intent(Intent::Create, 0.9),
            create_test_intent(Intent::Copy, 0.85),
        ];
        
        let result = analyzer.analyze("create and copy files", &intents).unwrap();
        
        assert_eq!(result.execution_order, ExecutionOrder::Parallel);
        assert!(!result.task_relationships.is_empty());
        assert!(matches!(result.task_relationships[0], TaskRelationship::Parallel));
    }

    #[test]
    fn test_conditional_task_detection() {
        let analyzer = ContextAnalyzer::new().unwrap();
        let intents = vec![
            create_test_intent(Intent::Create, 0.9),
            create_test_intent(Intent::Delete, 0.85),
        ];
        
        let result = analyzer.analyze("if file exists, delete it", &intents).unwrap();
        
        assert_eq!(result.execution_order, ExecutionOrder::Sequential);
        assert!(!result.task_relationships.is_empty());
        assert!(matches!(result.task_relationships[0], TaskRelationship::Conditional));
    }

    #[test]
    fn test_dependency_inference() {
        let analyzer = ContextAnalyzer::new().unwrap();
        let intents = vec![
            create_test_intent(Intent::Create, 0.9),
            create_test_intent(Intent::Copy, 0.85),
            create_test_intent(Intent::Delete, 0.8),
        ];
        
        let result = analyzer.analyze("create files then copy them and delete them", &intents).unwrap();
        
        // Should have dependencies between tasks
        assert!(!result.implicit_dependencies.is_empty());
        assert_eq!(result.implicit_dependencies.len(), 2); // task_1 depends on task_0, task_2 depends on task_1
    }

    #[test]
    fn test_mixed_execution_order() {
        let analyzer = ContextAnalyzer::new().unwrap();
        let intents = vec![
            create_test_intent(Intent::Create, 0.9),
            create_test_intent(Intent::Copy, 0.85),
            create_test_intent(Intent::Delete, 0.8),
        ];
        
        let result = analyzer.analyze("create file then copy and delete simultaneously", &intents).unwrap();
        
        assert_eq!(result.execution_order, ExecutionOrder::Mixed);
    }

    #[test]
    fn test_single_intent_no_relationships() {
        let analyzer = ContextAnalyzer::new().unwrap();
        let intents = vec![
            create_test_intent(Intent::Create, 0.9),
        ];
        
        let result = analyzer.analyze("create a file", &intents).unwrap();
        
        // Single intent should have no relationships
        assert!(result.task_relationships.is_empty());
        assert_eq!(result.execution_order, ExecutionOrder::Sequential);
        assert!(result.implicit_dependencies.is_empty());
    }

    #[test]
    fn test_dependency_pattern_detection() {
        let analyzer = ContextAnalyzer::new().unwrap();
        
        assert!(analyzer.has_dependency_pattern("create files and copy them"));
        assert!(analyzer.has_dependency_pattern("generate output and process it"));
        assert!(analyzer.has_dependency_pattern("run command and use result"));
        assert!(!analyzer.has_dependency_pattern("create and copy files"));
    }
}
