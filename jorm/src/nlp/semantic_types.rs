//! Semantic types for Tier 3 transformer-based NLP
//!
//! This module defines the core data structures used throughout the Tier 3 NLP pipeline,
//! including intents, entities, and semantic parse results.

#[cfg(feature = "tier3")]
use std::collections::HashMap;

#[cfg(feature = "tier3")]
use serde::{Deserialize, Serialize};

/// Represents the user's intent extracted from natural language
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Intent {
    Create,
    Copy,
    Move,
    Delete,
    Execute,
    Pipeline,
    Workflow,
    Http,
    Unknown,
}

/// Result of intent classification with confidence scoring
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    pub intent: Intent,
    pub confidence: f32,
    pub secondary_intents: Vec<(Intent, f32)>,
}

/// Represents an extracted entity from natural language
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Entity {
    FileName(String),
    FilePath(String),
    Number { value: u32, unit: Option<String> },
    Command(String),
    Url(String),
    Variable(String),
}

/// Collection of extracted entities with confidence scores
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntities {
    pub entities: Vec<Entity>,
    pub confidence: HashMap<usize, f32>,
}

/// Relationship between tasks
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskRelationship {
    Sequential,
    Parallel,
    Conditional,
}

/// Execution order for tasks
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionOrder {
    Sequential,
    Parallel,
    Mixed,
}

/// Context analysis result
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAnalysis {
    pub task_relationships: Vec<TaskRelationship>,
    pub execution_order: ExecutionOrder,
    pub implicit_dependencies: Vec<(String, String)>,
}

/// A semantic task extracted from natural language
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTask {
    pub name: String,
    pub intent: Intent,
    pub entities: Vec<Entity>,
    pub dependencies: Vec<String>,
    pub confidence: f32,
}

/// Complete semantic parse result
#[cfg(feature = "tier3")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticParse {
    pub original_text: String,
    pub intents: Vec<IntentResult>,
    pub entities: ExtractedEntities,
    pub context: ContextAnalysis,
    pub tasks: Vec<SemanticTask>,
    pub confidence: f32,
}

#[cfg(feature = "tier3")]
impl SemanticParse {
    /// Check if the parse result has sufficient confidence
    pub fn is_confident(&self) -> bool {
        self.confidence >= 0.7
    }
}
