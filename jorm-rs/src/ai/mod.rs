//! AI-powered features for Jorm
//! 
//! This module provides intelligent DAG analysis, natural language processing,
//! and automated optimization suggestions using local language models.

pub mod models;
pub mod analysis;
pub mod generation;
pub mod chat;
pub mod model_manager;
pub mod interactive;

pub use models::*;
pub use analysis::*;
pub use generation::*;
pub use chat::*;

use anyhow::Result;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// AI service for intelligent DAG operations
pub struct AIService {
    /// Language model provider
    model_provider: Arc<dyn LanguageModelProvider>,
    
    /// Analysis engine for DAG optimization
    analysis_engine: DAGAnalysisEngine,
    
    /// Natural language to DAG generator
    nl_generator: NaturalLanguageGenerator,
    
    /// Chat interface for interactive DAG management
    chat_interface: ChatInterface,
}

impl AIService {
    /// Create a new AI service with default configuration
    pub async fn new() -> Result<Self> {
        let model_provider = Self::create_default_model_provider().await?;
        let analysis_engine = DAGAnalysisEngine::new(model_provider.clone());
        let nl_generator = NaturalLanguageGenerator::new(model_provider.clone());
        let chat_interface = ChatInterface::new(model_provider.clone());
        
        Ok(Self {
            model_provider,
            analysis_engine,
            nl_generator,
            chat_interface,
        })
    }
    
    /// Create a new AI service with custom model provider
    pub fn with_model_provider(
        model_provider: Arc<dyn LanguageModelProvider>,
    ) -> Self {
        let analysis_engine = DAGAnalysisEngine::new(model_provider.clone());
        let nl_generator = NaturalLanguageGenerator::new(model_provider.clone());
        let chat_interface = ChatInterface::new(model_provider.clone());
        
        Self {
            model_provider,
            analysis_engine,
            nl_generator,
            chat_interface,
        }
    }
    
    /// Create default model provider (Phi-3 Mini)
    async fn create_default_model_provider() -> Result<Arc<dyn LanguageModelProvider>> {
        // Try to create Phi-3 Mini provider first
        match Phi3Provider::new().await {
            Ok(provider) => Ok(Arc::new(provider)),
            Err(_) => {
                // Fallback to Gemma if Phi-3 is not available
                match GemmaProvider::new().await {
                    Ok(provider) => Ok(Arc::new(provider)),
                    Err(_) => {
                        // Final fallback to remote API
                        Ok(Arc::new(RemoteAPIProvider::new()?))
                    }
                }
            }
        }
    }
    
    /// Analyze a DAG and provide optimization suggestions
    pub async fn analyze_dag(&self, dag: &crate::parser::Dag) -> Result<DAGAnalysis> {
        self.analysis_engine.analyze(dag).await
    }
    
    /// Generate a DAG from natural language description
    pub async fn generate_dag_from_natural_language(
        &self,
        description: &str,
    ) -> Result<crate::parser::Dag> {
        self.nl_generator.generate_dag(description).await
    }
    
    /// Start interactive chat interface
    pub async fn start_chat(&self) -> Result<()> {
        self.chat_interface.start().await
    }
    
    /// Get model information
    pub fn model_info(&self) -> ModelInfo {
        self.model_provider.model_info()
    }
}

/// Information about the loaded language model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub parameters: u64,
    pub memory_usage: u64,
    pub capabilities: Vec<String>,
}

/// Analysis result for a DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGAnalysis {
    pub dag_id: String,
    pub analysis_timestamp: chrono::DateTime<chrono::Utc>,
    pub performance_score: f64,
    pub optimization_suggestions: Vec<OptimizationSuggestion>,
    pub potential_issues: Vec<PotentialIssue>,
    pub resource_usage_estimate: ResourceUsageEstimate,
    pub complexity_metrics: ComplexityMetrics,
}

/// Optimization suggestion for a DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub suggestion_type: OptimizationType,
    pub description: String,
    pub impact: ImpactLevel,
    pub implementation_effort: EffortLevel,
    pub suggested_changes: Option<serde_json::Value>,
}

/// Types of optimizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    /// Parallel execution optimization
    Parallelization,
    /// Dependency optimization
    DependencyOptimization,
    /// Resource usage optimization
    ResourceOptimization,
    /// Error handling improvement
    ErrorHandling,
    /// Performance tuning
    PerformanceTuning,
}

/// Impact levels for suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Effort levels for implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Potential issues in a DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialIssue {
    pub issue_type: IssueType,
    pub description: String,
    pub severity: Severity,
    pub affected_tasks: Vec<String>,
    pub suggested_fix: Option<String>,
}

/// Types of issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueType {
    /// Circular dependencies
    CircularDependency,
    /// Resource bottlenecks
    ResourceBottleneck,
    /// Error handling gaps
    ErrorHandlingGap,
    /// Performance issues
    PerformanceIssue,
    /// Security concerns
    SecurityIssue,
}

/// Severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Resource usage estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageEstimate {
    pub estimated_duration: std::time::Duration,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub network_usage: Option<u64>,
    pub storage_usage: Option<u64>,
}

/// Complexity metrics for a DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub task_count: usize,
    pub dependency_count: usize,
    pub max_depth: usize,
    pub average_fan_out: f64,
    pub cyclomatic_complexity: usize,
    pub maintainability_index: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ai_service_creation() {
        // This test might fail if no models are available
        match AIService::new().await {
            Ok(service) => {
                let model_info = service.model_info();
                assert!(!model_info.name.is_empty());
            }
            Err(_) => {
                // Skip test if no AI models are available
                println!("AI models not available, skipping test");
            }
        }
    }
}
