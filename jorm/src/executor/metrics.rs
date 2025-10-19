//! Execution metrics collection and performance monitoring
//!
//! This module provides comprehensive metrics tracking for DAG executions,
//! including timing, resource usage, and performance data.

use crate::executor::{ExecutionResult, TaskResult, ExecutionStatus, TaskStatus};
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Comprehensive metrics collector for execution monitoring
pub struct MetricsCollector {
    /// Execution-level metrics
    execution_metrics: Arc<RwLock<HashMap<String, ExecutionMetrics>>>,
    /// Task-level metrics
    task_metrics: Arc<RwLock<HashMap<String, TaskMetrics>>>,
    /// System resource metrics
    resource_metrics: Arc<RwLock<ResourceMetrics>>,
    /// Performance benchmarks
    benchmarks: Arc<RwLock<Vec<PerformanceBenchmark>>>,
}

/// Detailed execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub execution_id: String,
    pub dag_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_duration: Option<Duration>,
    pub status: ExecutionStatus,
    
    // Task statistics
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub skipped_tasks: usize,
    pub retried_tasks: usize,
    
    // Performance metrics
    pub peak_concurrent_tasks: usize,
    pub average_task_duration: Option<Duration>,
    pub median_task_duration: Option<Duration>,
    pub min_task_duration: Option<Duration>,
    pub max_task_duration: Option<Duration>,
    
    // Resource usage
    pub peak_memory_usage: Option<u64>, // bytes
    pub total_cpu_time: Option<Duration>,
    pub average_cpu_usage: Option<f64>, // percentage
    
    // Throughput metrics
    pub tasks_per_second: Option<f64>,
    pub data_processed: Option<u64>, // bytes
    pub data_throughput: Option<f64>, // bytes per second
    
    // Error metrics
    pub error_rate: f64, // percentage
    pub retry_rate: f64, // percentage
    
    // Timeline data
    pub task_timeline: Vec<TaskTimelineEntry>,
    
    // Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Task-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub task_id: String,
    pub execution_id: String,
    pub task_type: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration: Option<Duration>,
    pub status: TaskStatus,
    
    // Performance metrics
    pub cpu_time: Option<Duration>,
    pub memory_usage: Option<u64>, // bytes
    pub peak_memory: Option<u64>, // bytes
    pub io_read_bytes: Option<u64>,
    pub io_write_bytes: Option<u64>,
    
    // Retry metrics
    pub retry_count: u32,
    pub retry_duration: Option<Duration>,
    
    // Output metrics
    pub output_size: Option<u64>, // bytes
    pub stdout_lines: Option<usize>,
    pub stderr_lines: Option<usize>,
    
    // Custom metrics
    pub custom_metrics: HashMap<String, f64>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// System resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub timestamp: DateTime<Utc>,
    
    // CPU metrics
    pub cpu_usage_percent: f64,
    pub cpu_cores: usize,
    pub load_average: Option<f64>,
    
    // Memory metrics
    pub memory_total: u64, // bytes
    pub memory_used: u64, // bytes
    pub memory_available: u64, // bytes
    pub memory_usage_percent: f64,
    
    // Disk metrics
    pub disk_usage_percent: f64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub disk_read_ops: u64,
    pub disk_write_ops: u64,
    
    // Network metrics
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub network_rx_packets: u64,
    pub network_tx_packets: u64,
    
    // Process metrics
    pub active_processes: usize,
    pub thread_count: usize,
    pub file_descriptors: usize,
}

/// Performance benchmark data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBenchmark {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub duration: Duration,
    pub operations_per_second: f64,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Task timeline entry for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTimelineEntry {
    pub task_id: String,
    pub task_type: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: TaskStatus,
    pub duration: Option<Duration>,
    pub dependencies: Vec<String>,
}

/// Metrics export format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsExport {
    pub export_timestamp: DateTime<Utc>,
    pub execution_metrics: Vec<ExecutionMetrics>,
    pub task_metrics: Vec<TaskMetrics>,
    pub resource_metrics: Vec<ResourceMetrics>,
    pub benchmarks: Vec<PerformanceBenchmark>,
    pub summary: MetricsSummary,
}

/// Summary statistics across all executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub average_execution_duration: Option<Duration>,
    pub total_tasks_executed: usize,
    pub average_tasks_per_execution: f64,
    pub overall_success_rate: f64,
    pub peak_concurrent_executions: usize,
    pub resource_efficiency: f64,
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            cpu_usage_percent: 0.0,
            cpu_cores: 1,
            load_average: None,
            memory_total: 0,
            memory_used: 0,
            memory_available: 0,
            memory_usage_percent: 0.0,
            disk_usage_percent: 0.0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            disk_read_ops: 0,
            disk_write_ops: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            network_rx_packets: 0,
            network_tx_packets: 0,
            active_processes: 0,
            thread_count: 0,
            file_descriptors: 0,
        }
    }
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            execution_metrics: Arc::new(RwLock::new(HashMap::new())),
            task_metrics: Arc::new(RwLock::new(HashMap::new())),
            resource_metrics: Arc::new(RwLock::new(ResourceMetrics::default())),
            benchmarks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start tracking an execution
    pub async fn start_execution(&self, execution_id: &str, dag_id: &str, total_tasks: usize) {
        let metrics = ExecutionMetrics {
            execution_id: execution_id.to_string(),
            dag_id: dag_id.to_string(),
            started_at: Utc::now(),
            completed_at: None,
            total_duration: None,
            status: ExecutionStatus::Running,
            total_tasks,
            successful_tasks: 0,
            failed_tasks: 0,
            skipped_tasks: 0,
            retried_tasks: 0,
            peak_concurrent_tasks: 0,
            average_task_duration: None,
            median_task_duration: None,
            min_task_duration: None,
            max_task_duration: None,
            peak_memory_usage: None,
            total_cpu_time: None,
            average_cpu_usage: None,
            tasks_per_second: None,
            data_processed: None,
            data_throughput: None,
            error_rate: 0.0,
            retry_rate: 0.0,
            task_timeline: Vec::new(),
            metadata: HashMap::new(),
        };

        let mut execution_metrics = self.execution_metrics.write().await;
        execution_metrics.insert(execution_id.to_string(), metrics);
    }

    /// Complete execution tracking
    pub async fn complete_execution(&self, execution_result: &ExecutionResult) {
        let mut execution_metrics = self.execution_metrics.write().await;
        
        if let Some(metrics) = execution_metrics.get_mut(&execution_result.execution_id) {
            metrics.completed_at = execution_result.completed_at;
            metrics.total_duration = execution_result.total_duration;
            metrics.status = execution_result.status.clone();
            metrics.successful_tasks = execution_result.metrics.successful_tasks;
            metrics.failed_tasks = execution_result.metrics.failed_tasks;
            metrics.skipped_tasks = execution_result.metrics.skipped_tasks;
            metrics.peak_concurrent_tasks = execution_result.metrics.peak_concurrent_tasks;
            metrics.average_task_duration = execution_result.metrics.average_task_duration;
            
            // Calculate derived metrics
            self.calculate_derived_metrics(metrics, execution_result).await;
        }
    }

    /// Track task execution
    pub async fn track_task(&self, execution_id: &str, task_result: &TaskResult) {
        let task_metrics = TaskMetrics {
            task_id: task_result.task_id.clone(),
            execution_id: execution_id.to_string(),
            task_type: "unknown".to_string(), // Would be filled from task definition
            started_at: task_result.started_at,
            completed_at: Some(task_result.completed_at),
            duration: Some(task_result.duration),
            status: task_result.status.clone(),
            cpu_time: None,
            memory_usage: None,
            peak_memory: None,
            io_read_bytes: None,
            io_write_bytes: None,
            retry_count: task_result.retry_count,
            retry_duration: None,
            output_size: Some(task_result.stdout.len() as u64 + task_result.stderr.len() as u64),
            stdout_lines: Some(task_result.stdout.lines().count()),
            stderr_lines: Some(task_result.stderr.lines().count()),
            custom_metrics: HashMap::new(),
            metadata: task_result.metadata.clone(),
        };

        let mut task_metrics_map = self.task_metrics.write().await;
        task_metrics_map.insert(task_result.task_id.clone(), task_metrics);

        // Update execution timeline
        self.update_execution_timeline(execution_id, task_result).await;
    }

    /// Update resource metrics
    pub async fn update_resource_metrics(&self, metrics: ResourceMetrics) {
        let mut resource_metrics = self.resource_metrics.write().await;
        *resource_metrics = metrics;
    }

    /// Add performance benchmark
    pub async fn add_benchmark(&self, benchmark: PerformanceBenchmark) {
        let mut benchmarks = self.benchmarks.write().await;
        benchmarks.push(benchmark);
    }

    /// Get execution metrics
    pub async fn get_execution_metrics(&self, execution_id: &str) -> Option<ExecutionMetrics> {
        let execution_metrics = self.execution_metrics.read().await;
        execution_metrics.get(execution_id).cloned()
    }

    /// Get task metrics
    pub async fn get_task_metrics(&self, task_id: &str) -> Option<TaskMetrics> {
        let task_metrics = self.task_metrics.read().await;
        task_metrics.get(task_id).cloned()
    }

    /// Get current resource metrics
    pub async fn get_resource_metrics(&self) -> ResourceMetrics {
        let resource_metrics = self.resource_metrics.read().await;
        resource_metrics.clone()
    }

    /// Export all metrics
    pub async fn export_metrics(&self) -> MetricsExport {
        let execution_metrics = self.execution_metrics.read().await;
        let task_metrics = self.task_metrics.read().await;
        let resource_metrics = self.resource_metrics.read().await;
        let benchmarks = self.benchmarks.read().await;

        let summary = self.calculate_summary(&execution_metrics).await;

        MetricsExport {
            export_timestamp: Utc::now(),
            execution_metrics: execution_metrics.values().cloned().collect(),
            task_metrics: task_metrics.values().cloned().collect(),
            resource_metrics: vec![resource_metrics.clone()],
            benchmarks: benchmarks.clone(),
            summary,
        }
    }

    /// Export metrics to JSON
    pub async fn export_to_json(&self) -> Result<String, serde_json::Error> {
        let metrics = self.export_metrics().await;
        serde_json::to_string_pretty(&metrics)
    }

    /// Clear old metrics (keep only recent data)
    pub async fn cleanup_old_metrics(&self, older_than: DateTime<Utc>) {
        let mut execution_metrics = self.execution_metrics.write().await;
        execution_metrics.retain(|_, metrics| metrics.started_at > older_than);

        let mut task_metrics = self.task_metrics.write().await;
        task_metrics.retain(|_, metrics| metrics.started_at > older_than);

        let mut benchmarks = self.benchmarks.write().await;
        benchmarks.retain(|benchmark| benchmark.timestamp > older_than);
    }

    /// Calculate derived metrics
    async fn calculate_derived_metrics(&self, metrics: &mut ExecutionMetrics, execution_result: &ExecutionResult) {
        // Calculate error rate
        if metrics.total_tasks > 0 {
            metrics.error_rate = (metrics.failed_tasks as f64 / metrics.total_tasks as f64) * 100.0;
        }

        // Calculate retry rate
        let total_retries: u32 = execution_result.task_results.values()
            .map(|task| task.retry_count)
            .sum();
        metrics.retried_tasks = execution_result.task_results.values()
            .filter(|task| task.retry_count > 0)
            .count();
        
        if metrics.total_tasks > 0 {
            metrics.retry_rate = (metrics.retried_tasks as f64 / metrics.total_tasks as f64) * 100.0;
        }

        // Calculate tasks per second
        if let Some(duration) = metrics.total_duration {
            let duration_secs = duration.as_secs_f64();
            if duration_secs > 0.0 {
                metrics.tasks_per_second = Some(metrics.total_tasks as f64 / duration_secs);
            }
        }

        // Calculate task duration statistics
        let durations: Vec<Duration> = execution_result.task_results.values()
            .map(|task| task.duration)
            .collect();

        if !durations.is_empty() {
            metrics.min_task_duration = durations.iter().min().cloned();
            metrics.max_task_duration = durations.iter().max().cloned();
            
            // Calculate median
            let mut sorted_durations = durations.clone();
            sorted_durations.sort();
            let mid = sorted_durations.len() / 2;
            metrics.median_task_duration = if sorted_durations.len() % 2 == 0 {
                Some(Duration::from_nanos(
                    ((sorted_durations[mid - 1].as_nanos() + sorted_durations[mid].as_nanos()) / 2) as u64
                ))
            } else {
                Some(sorted_durations[mid])
            };
        }
    }

    /// Update execution timeline
    async fn update_execution_timeline(&self, execution_id: &str, task_result: &TaskResult) {
        let mut execution_metrics = self.execution_metrics.write().await;
        
        if let Some(metrics) = execution_metrics.get_mut(execution_id) {
            let timeline_entry = TaskTimelineEntry {
                task_id: task_result.task_id.clone(),
                task_type: "unknown".to_string(), // Would be filled from task definition
                started_at: task_result.started_at,
                completed_at: Some(task_result.completed_at),
                status: task_result.status.clone(),
                duration: Some(task_result.duration),
                dependencies: Vec::new(), // Would be filled from task definition
            };
            
            metrics.task_timeline.push(timeline_entry);
        }
    }

    /// Calculate summary statistics
    async fn calculate_summary(&self, execution_metrics: &HashMap<String, ExecutionMetrics>) -> MetricsSummary {
        let total_executions = execution_metrics.len();
        let successful_executions = execution_metrics.values()
            .filter(|m| m.status == ExecutionStatus::Success)
            .count();
        let failed_executions = execution_metrics.values()
            .filter(|m| m.status == ExecutionStatus::Failed)
            .count();

        let total_tasks_executed: usize = execution_metrics.values()
            .map(|m| m.total_tasks)
            .sum();

        let average_tasks_per_execution = if total_executions > 0 {
            total_tasks_executed as f64 / total_executions as f64
        } else {
            0.0
        };

        let overall_success_rate = if total_executions > 0 {
            (successful_executions as f64 / total_executions as f64) * 100.0
        } else {
            0.0
        };

        let durations: Vec<Duration> = execution_metrics.values()
            .filter_map(|m| m.total_duration)
            .collect();

        let average_execution_duration = if !durations.is_empty() {
            let total_nanos: u128 = durations.iter().map(|d| d.as_nanos()).sum();
            Some(Duration::from_nanos((total_nanos / durations.len() as u128) as u64))
        } else {
            None
        };

        MetricsSummary {
            total_executions,
            successful_executions,
            failed_executions,
            average_execution_duration,
            total_tasks_executed,
            average_tasks_per_execution,
            overall_success_rate,
            peak_concurrent_executions: 1, // Would need to track this separately
            resource_efficiency: 85.0, // Placeholder - would calculate based on resource usage
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// System resource monitor
pub struct ResourceMonitor {
    collector: Arc<MetricsCollector>,
    monitoring_interval: Duration,
    running: Arc<RwLock<bool>>,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(collector: Arc<MetricsCollector>, monitoring_interval: Duration) -> Self {
        Self {
            collector,
            monitoring_interval,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start monitoring system resources
    pub async fn start_monitoring(&self) {
        let mut running = self.running.write().await;
        *running = true;
        drop(running);

        let collector = self.collector.clone();
        let interval = self.monitoring_interval;
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.read().await {
                let metrics = Self::collect_system_metrics().await;
                collector.update_resource_metrics(metrics).await;
                tokio::time::sleep(interval).await;
            }
        });
    }

    /// Stop monitoring
    pub async fn stop_monitoring(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    /// Collect current system metrics
    async fn collect_system_metrics() -> ResourceMetrics {
        // This is a simplified implementation
        // In a real system, you would use system APIs or libraries like `sysinfo`
        ResourceMetrics {
            timestamp: Utc::now(),
            cpu_usage_percent: 50.0, // Placeholder
            cpu_cores: num_cpus::get(),
            load_average: None,
            memory_total: 8_000_000_000, // 8GB placeholder
            memory_used: 4_000_000_000,  // 4GB placeholder
            memory_available: 4_000_000_000,
            memory_usage_percent: 50.0,
            disk_usage_percent: 60.0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            disk_read_ops: 0,
            disk_write_ops: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            network_rx_packets: 0,
            network_tx_packets: 0,
            active_processes: 100,
            thread_count: 500,
            file_descriptors: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{ExecutionMetrics as ExecMetrics, TaskStatus};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        
        // Test that we can start tracking an execution
        collector.start_execution("test-exec", "test-dag", 5).await;
        
        let metrics = collector.get_execution_metrics("test-exec").await;
        assert!(metrics.is_some());
        
        let metrics = metrics.unwrap();
        assert_eq!(metrics.execution_id, "test-exec");
        assert_eq!(metrics.dag_id, "test-dag");
        assert_eq!(metrics.total_tasks, 5);
        assert_eq!(metrics.status, ExecutionStatus::Running);
    }

    #[tokio::test]
    async fn test_task_tracking() {
        let collector = MetricsCollector::new();
        
        collector.start_execution("test-exec", "test-dag", 1).await;
        
        let task_result = TaskResult {
            task_id: "test-task".to_string(),
            status: TaskStatus::Success,
            started_at: Utc::now(),
            completed_at: Utc::now(),
            duration: Duration::from_secs(5),
            stdout: "Task output".to_string(),
            stderr: "".to_string(),
            exit_code: Some(0),
            retry_count: 0,
            output_data: None,
            error_message: None,
            metadata: HashMap::new(),
        };
        
        collector.track_task("test-exec", &task_result).await;
        
        let task_metrics = collector.get_task_metrics("test-task").await;
        assert!(task_metrics.is_some());
        
        let task_metrics = task_metrics.unwrap();
        assert_eq!(task_metrics.task_id, "test-task");
        assert_eq!(task_metrics.execution_id, "test-exec");
        assert_eq!(task_metrics.status, TaskStatus::Success);
        assert_eq!(task_metrics.retry_count, 0);
    }

    #[tokio::test]
    async fn test_metrics_export() {
        let collector = MetricsCollector::new();
        
        collector.start_execution("test-exec", "test-dag", 1).await;
        
        let export = collector.export_metrics().await;
        assert_eq!(export.execution_metrics.len(), 1);
        assert_eq!(export.execution_metrics[0].execution_id, "test-exec");
        
        let json = collector.export_to_json().await.unwrap();
        assert!(json.contains("test-exec"));
        assert!(json.contains("test-dag"));
    }

    #[tokio::test]
    async fn test_resource_monitor() {
        let collector = Arc::new(MetricsCollector::new());
        let monitor = ResourceMonitor::new(collector.clone(), Duration::from_millis(100));
        
        monitor.start_monitoring().await;
        
        // Wait a bit for monitoring to collect data
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        let metrics = collector.get_resource_metrics().await;
        assert!(metrics.cpu_cores > 0);
        
        monitor.stop_monitoring().await;
    }

    #[tokio::test]
    async fn test_metrics_cleanup() {
        let collector = MetricsCollector::new();
        
        collector.start_execution("old-exec", "test-dag", 1).await;
        collector.start_execution("new-exec", "test-dag", 1).await;
        
        // Both should exist initially
        assert!(collector.get_execution_metrics("old-exec").await.is_some());
        assert!(collector.get_execution_metrics("new-exec").await.is_some());
        
        // Clean up metrics older than a past time (should not remove anything)
        let cutoff = Utc::now() - ChronoDuration::seconds(10);
        collector.cleanup_old_metrics(cutoff).await;
        
        // Both should still exist since they were created after the cutoff
        assert!(collector.get_execution_metrics("old-exec").await.is_some());
        assert!(collector.get_execution_metrics("new-exec").await.is_some());
    }
}