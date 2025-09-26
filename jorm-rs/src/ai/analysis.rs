//! DAG analysis engine for intelligent optimization

use crate::ai::{
    ComplexityMetrics, DAGAnalysis, EffortLevel, ImpactLevel, IssueType, OptimizationSuggestion,
    OptimizationType, PotentialIssue, ResourceUsageEstimate, Severity,
};
use crate::parser::Dag;
use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;

/// DAG analysis engine
pub struct DAGAnalysisEngine {
    model_provider: std::sync::Arc<dyn crate::ai::LanguageModelProvider>,
}

impl DAGAnalysisEngine {
    /// Create a new DAG analysis engine
    pub fn new(model_provider: std::sync::Arc<dyn crate::ai::LanguageModelProvider>) -> Self {
        Self { model_provider }
    }

    /// Analyze a DAG and provide comprehensive insights
    pub async fn analyze(&self, dag: &Dag) -> Result<DAGAnalysis> {
        let analysis_timestamp = chrono::Utc::now();

        // Calculate complexity metrics
        let complexity_metrics = self.calculate_complexity_metrics(dag);

        // Calculate performance score
        let performance_score = self.calculate_performance_score(dag, &complexity_metrics);

        // Generate optimization suggestions
        let optimization_suggestions = self
            .generate_optimization_suggestions(dag, &complexity_metrics)
            .await?;

        // Identify potential issues
        let potential_issues = self.identify_potential_issues(dag, &complexity_metrics);

        // Estimate resource usage
        let resource_usage_estimate = self.estimate_resource_usage(dag, &complexity_metrics);

        Ok(DAGAnalysis {
            dag_id: dag.name.clone(),
            analysis_timestamp,
            performance_score,
            optimization_suggestions,
            potential_issues,
            resource_usage_estimate,
            complexity_metrics,
        })
    }

    /// Calculate complexity metrics for a DAG
    fn calculate_complexity_metrics(&self, dag: &Dag) -> ComplexityMetrics {
        let task_count = dag.tasks.len();
        let dependency_count = dag.dependencies.len();

        // Calculate max depth using BFS
        let max_depth = self.calculate_max_depth(dag);

        // Calculate average fan-out (dependencies per task)
        let average_fan_out = if task_count > 0 {
            dependency_count as f64 / task_count as f64
        } else {
            0.0
        };

        // Calculate cyclomatic complexity
        let cyclomatic_complexity = self.calculate_cyclomatic_complexity(dag);

        // Calculate maintainability index (simplified)
        let maintainability_index = self.calculate_maintainability_index(
            task_count,
            dependency_count,
            max_depth,
            cyclomatic_complexity,
        );

        ComplexityMetrics {
            task_count,
            dependency_count,
            max_depth,
            average_fan_out,
            cyclomatic_complexity,
            maintainability_index,
        }
    }

    /// Calculate maximum depth of the DAG
    fn calculate_max_depth(&self, dag: &Dag) -> usize {
        use std::collections::{HashMap, VecDeque};

        // Build adjacency list
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for task_name in dag.tasks.keys() {
            graph.insert(task_name.clone(), Vec::new());
        }

        for dep in &dag.dependencies {
            graph
                .get_mut(&dep.depends_on)
                .unwrap()
                .push(dep.task.clone());
        }

        // Find root nodes (tasks with no dependencies)
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for task_name in dag.tasks.keys() {
            in_degree.insert(task_name.clone(), 0);
        }

        for dep in &dag.dependencies {
            *in_degree.get_mut(&dep.task).unwrap() += 1;
        }

        let root_nodes: Vec<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(task, _)| task.clone())
            .collect();

        if root_nodes.is_empty() {
            return 0;
        }

        // BFS to find maximum depth
        let mut max_depth = 0;
        let mut queue: VecDeque<(String, usize)> =
            root_nodes.into_iter().map(|task| (task, 0)).collect();

        while let Some((current_task, depth)) = queue.pop_front() {
            max_depth = max_depth.max(depth);

            if let Some(neighbors) = graph.get(&current_task) {
                for neighbor in neighbors {
                    queue.push_back((neighbor.clone(), depth + 1));
                }
            }
        }

        max_depth
    }

    /// Calculate cyclomatic complexity
    fn calculate_cyclomatic_complexity(&self, dag: &Dag) -> usize {
        // Cyclomatic complexity = E - N + 2P
        // Where E = number of edges, N = number of nodes, P = number of connected components
        let edges = dag.dependencies.len();
        let nodes = dag.tasks.len();
        let components = 1; // Assuming single connected component for DAGs

        if nodes == 0 {
            0
        } else {
            edges.saturating_sub(nodes) + 2 * components
        }
    }

    /// Calculate maintainability index
    fn calculate_maintainability_index(
        &self,
        task_count: usize,
        dependency_count: usize,
        max_depth: usize,
        cyclomatic_complexity: usize,
    ) -> f64 {
        // Simplified maintainability index calculation
        // Higher values indicate better maintainability
        let complexity_penalty = (cyclomatic_complexity as f64 * 0.1).min(10.0);
        let depth_penalty = (max_depth as f64 * 0.05).min(5.0);
        let size_penalty = ((task_count + dependency_count) as f64 * 0.01).min(3.0);

        let base_score = 100.0;
        let total_penalty = complexity_penalty + depth_penalty + size_penalty;

        (base_score - total_penalty).max(0.0)
    }

    /// Calculate performance score
    fn calculate_performance_score(&self, _dag: &Dag, metrics: &ComplexityMetrics) -> f64 {
        let mut score: f64 = 1.0;

        // Penalize high complexity
        if metrics.cyclomatic_complexity > 10 {
            score -= 0.2;
        }

        // Penalize high depth
        if metrics.max_depth > 5 {
            score -= 0.15;
        }

        // Penalize high fan-out
        if metrics.average_fan_out > 3.0 {
            score -= 0.1;
        }

        // Reward good maintainability
        if metrics.maintainability_index > 80.0 {
            score += 0.1;
        }

        score.max(0.0).min(1.0)
    }

    /// Generate optimization suggestions
    async fn generate_optimization_suggestions(
        &self,
        dag: &Dag,
        metrics: &ComplexityMetrics,
    ) -> Result<Vec<OptimizationSuggestion>> {
        let mut suggestions = Vec::new();

        // Check for parallelization opportunities
        if self.has_parallelization_opportunities(dag) {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: OptimizationType::Parallelization,
                description: "Some tasks can be executed in parallel to improve performance"
                    .to_string(),
                impact: ImpactLevel::High,
                implementation_effort: EffortLevel::Medium,
                suggested_changes: Some(serde_json::json!({
                    "parallel_tasks": self.identify_parallel_tasks(dag)
                })),
            });
        }

        // Check for dependency optimization
        if metrics.average_fan_out > 2.0 {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: OptimizationType::DependencyOptimization,
                description: "Consider reducing task dependencies to simplify the DAG".to_string(),
                impact: ImpactLevel::Medium,
                implementation_effort: EffortLevel::High,
                suggested_changes: None,
            });
        }

        // Check for resource optimization
        if dag.tasks.len() > 10 {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: OptimizationType::ResourceOptimization,
                description:
                    "Consider breaking down large DAGs into smaller, more manageable pieces"
                        .to_string(),
                impact: ImpactLevel::Medium,
                implementation_effort: EffortLevel::High,
                suggested_changes: None,
            });
        }

        // Check for error handling
        suggestions.push(OptimizationSuggestion {
            suggestion_type: OptimizationType::ErrorHandling,
            description: "Consider adding retry mechanisms and error handling for critical tasks"
                .to_string(),
            impact: ImpactLevel::High,
            implementation_effort: EffortLevel::Low,
            suggested_changes: None,
        });

        Ok(suggestions)
    }

    /// Check if there are parallelization opportunities
    fn has_parallelization_opportunities(&self, dag: &Dag) -> bool {
        // Simple heuristic: if there are tasks with no dependencies, they can run in parallel
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for task_name in dag.tasks.keys() {
            in_degree.insert(task_name.clone(), 0);
        }

        for dep in &dag.dependencies {
            *in_degree.get_mut(&dep.task).unwrap() += 1;
        }

        let independent_tasks = in_degree.values().filter(|&&degree| degree == 0).count();
        independent_tasks > 1
    }

    /// Identify tasks that can run in parallel
    fn identify_parallel_tasks(&self, dag: &Dag) -> Vec<Vec<String>> {
        use std::collections::HashMap;

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for task_name in dag.tasks.keys() {
            in_degree.insert(task_name.clone(), 0);
        }

        for dep in &dag.dependencies {
            *in_degree.get_mut(&dep.task).unwrap() += 1;
        }

        let mut parallel_groups = Vec::new();
        let mut remaining_tasks: Vec<String> = dag.tasks.keys().cloned().collect();

        while !remaining_tasks.is_empty() {
            // Find tasks with no dependencies
            let current_level: Vec<String> = remaining_tasks
                .iter()
                .filter(|task| in_degree.get(*task).unwrap_or(&0) == &0)
                .cloned()
                .collect();

            if current_level.is_empty() {
                break; // Circular dependency or error
            }

            parallel_groups.push(current_level.clone());

            // Remove completed tasks and update dependencies
            for task in &current_level {
                remaining_tasks.retain(|t| t != task);

                // Update in-degrees for dependent tasks
                for dep in &dag.dependencies {
                    if dep.depends_on == *task {
                        if let Some(degree) = in_degree.get_mut(&dep.task) {
                            *degree = degree.saturating_sub(1);
                        }
                    }
                }
            }
        }

        parallel_groups
    }

    /// Identify potential issues in the DAG
    fn identify_potential_issues(
        &self,
        dag: &Dag,
        metrics: &ComplexityMetrics,
    ) -> Vec<PotentialIssue> {
        let mut issues = Vec::new();

        // Check for high complexity
        if metrics.cyclomatic_complexity > 15 {
            issues.push(PotentialIssue {
                issue_type: IssueType::PerformanceIssue,
                description:
                    "High cyclomatic complexity may impact performance and maintainability"
                        .to_string(),
                severity: Severity::Warning,
                affected_tasks: dag.tasks.keys().cloned().collect(),
                suggested_fix: Some(
                    "Consider breaking down complex tasks into smaller, simpler ones".to_string(),
                ),
            });
        }

        // Check for deep dependency chains
        if metrics.max_depth > 8 {
            issues.push(PotentialIssue {
                issue_type: IssueType::PerformanceIssue,
                description: "Deep dependency chains may cause delays and make debugging difficult"
                    .to_string(),
                severity: Severity::Warning,
                affected_tasks: dag.tasks.keys().cloned().collect(),
                suggested_fix: Some("Consider flattening the dependency structure".to_string()),
            });
        }

        // Check for tasks with many dependencies
        let mut task_dependency_count: HashMap<String, usize> = HashMap::new();
        for dep in &dag.dependencies {
            *task_dependency_count.entry(dep.task.clone()).or_insert(0) += 1;
        }

        for (task, count) in task_dependency_count {
            if count > 5 {
                issues.push(PotentialIssue {
                    issue_type: IssueType::ResourceBottleneck,
                    description: format!(
                        "Task '{}' has many dependencies ({}), which may create bottlenecks",
                        task, count
                    ),
                    severity: Severity::Warning,
                    affected_tasks: vec![task],
                    suggested_fix: Some(
                        "Consider breaking down this task or reducing its dependencies".to_string(),
                    ),
                });
            }
        }

        issues
    }

    /// Estimate resource usage for the DAG
    fn estimate_resource_usage(
        &self,
        _dag: &Dag,
        metrics: &ComplexityMetrics,
    ) -> ResourceUsageEstimate {
        // Simple estimation based on task count and complexity
        let base_duration_per_task = Duration::from_secs(30); // 30 seconds per task
        let _complexity_multiplier = 1.0 + (metrics.cyclomatic_complexity as f64 * 0.1);
        let estimated_duration = base_duration_per_task * metrics.task_count as u32;

        // Estimate memory usage (simplified)
        let base_memory_per_task = 50 * 1024 * 1024; // 50MB per task
        let memory_usage = base_memory_per_task * metrics.task_count as u64;

        // Estimate CPU usage
        let cpu_usage = (metrics.task_count as f64 * 0.1).min(1.0);

        ResourceUsageEstimate {
            estimated_duration,
            memory_usage,
            cpu_usage,
            network_usage: None,
            storage_usage: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Dag, Task};

    fn create_test_dag() -> Dag {
        let mut dag = Dag::new("test_dag".to_string());

        // Add tasks
        dag.add_task(Task::new("task1".to_string()));
        dag.add_task(Task::new("task2".to_string()));
        dag.add_task(Task::new("task3".to_string()));
        dag.add_task(Task::new("task4".to_string()));

        // Add dependencies
        dag.add_dependency("task2".to_string(), "task1".to_string());
        dag.add_dependency("task3".to_string(), "task1".to_string());
        dag.add_dependency("task4".to_string(), "task2".to_string());
        dag.add_dependency("task4".to_string(), "task3".to_string());

        dag
    }

    #[tokio::test]
    async fn test_complexity_metrics_calculation() {
        let dag = create_test_dag();
        let engine = DAGAnalysisEngine::new(std::sync::Arc::new(
            crate::ai::RemoteAPIProvider::new().unwrap(),
        ));

        let metrics = engine.calculate_complexity_metrics(&dag);

        assert_eq!(metrics.task_count, 4);
        assert_eq!(metrics.dependency_count, 4);
        assert!(metrics.max_depth > 0);
        assert!(metrics.average_fan_out > 0.0);
        assert!(metrics.cyclomatic_complexity >= 0);
        assert!(metrics.maintainability_index >= 0.0);
    }

    #[tokio::test]
    async fn test_parallelization_opportunities() {
        let dag = create_test_dag();
        let engine = DAGAnalysisEngine::new(std::sync::Arc::new(
            crate::ai::RemoteAPIProvider::new().unwrap(),
        ));

        let has_opportunities = engine.has_parallelization_opportunities(&dag);
        assert!(has_opportunities); // task1 has no dependencies, so it can run in parallel with others

        let parallel_tasks = engine.identify_parallel_tasks(&dag);
        assert!(!parallel_tasks.is_empty());
        assert!(parallel_tasks[0].contains(&"task1".to_string()));
    }

    #[tokio::test]
    async fn test_performance_score_calculation() {
        let dag = create_test_dag();
        let engine = DAGAnalysisEngine::new(std::sync::Arc::new(
            crate::ai::RemoteAPIProvider::new().unwrap(),
        ));

        let metrics = engine.calculate_complexity_metrics(&dag);
        let score = engine.calculate_performance_score(&dag, &metrics);

        assert!(score >= 0.0);
        assert!(score <= 1.0);
    }
}
