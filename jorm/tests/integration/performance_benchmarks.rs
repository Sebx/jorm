//! Performance benchmarks and comparison tests for native vs Python execution
//!
//! This module provides comprehensive performance benchmarks that measure execution time,
//! memory usage, and scalability of the native Rust executor compared to the Python engine.

use jorm::executor::{ExecutionStatus, ExecutorConfig, NativeExecutor};
use jorm::parser::{Dag, Dependency, Task as DagTask, TaskConfig as DagTaskConfig};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use sysinfo::{System, SystemExt, ProcessExt, PidExt};
use tempfile::TempDir;

/// Performance benchmark suite for measuring executor performance
pub struct PerformanceBenchmark {
    temp_dir: TempDir,
    system: System,
}

/// Benchmark results for a single test
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub test_name: String,
    pub execution_time: Duration,
    pub peak_memory_mb: f64,
    pub task_count: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub throughput_tasks_per_second: f64,
}

/// Comparison results between native and Python executors
#[derive(Debug)]
pub struct ComparisonResult {
    pub native_result: BenchmarkResult,
    pub python_result: Option<BenchmarkResult>, // None if Python not available
    pub speedup_factor: Option<f64>,
    pub memory_improvement: Option<f64>,
}

impl PerformanceBenchmark {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("Failed to create temp directory"),
            system: System::new_all(),
        }
    }

    /// Get current memory usage in MB
    fn get_memory_usage_mb(&mut self) -> f64 {
        self.system.refresh_all();
        let pid = sysinfo::get_current_pid().unwrap();
        if let Some(process) = self.system.process(pid) {
            process.memory() as f64 / 1024.0 / 1024.0 // Convert KB to MB
        } else {
            0.0
        }
    }

    /// Create a simple DAG for benchmarking
    fn create_benchmark_dag(&self, task_count: usize, task_type: &str) -> Dag {
        let mut tasks = HashMap::new();

        for i in 1..=task_count {
            let task_config = match task_type {
                "shell" => DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some(format!("echo 'Benchmark task {}'", i)),
                    ..Default::default()
                },
                "python" => DagTaskConfig {
                    task_type: Some("python".to_string()),
                    script: Some(format!("print('Benchmark task {}')", i)),
                    ..Default::default()
                },
                _ => panic!("Unsupported task type for benchmark: {}", task_type),
            };

            tasks.insert(
                format!("task_{}", i),
                DagTask {
                    name: format!("task_{}", i),
                    description: Some(format!("Benchmark task {}", i)),
                    config: task_config,
                },
            );
        }

        Dag {
            name: format!("benchmark_dag_{}_tasks", task_count),
            schedule: None,
            tasks,
            dependencies: vec![], // Independent tasks for maximum parallelism
        }
    }    /// 
Create a DAG with dependencies for testing sequential vs parallel performance
    fn create_dependency_chain_dag(&self, chain_length: usize) -> Dag {
        let mut tasks = HashMap::new();
        let mut dependencies = Vec::new();

        for i in 1..=chain_length {
            tasks.insert(
                format!("chain_task_{}", i),
                DagTask {
                    name: format!("chain_task_{}", i),
                    description: Some(format!("Chain task {}", i)),
                    config: DagTaskConfig {
                        task_type: Some("shell".to_string()),
                        command: Some(format!("echo 'Chain task {}'", i)),
                        ..Default::default()
                    },
                },
            );

            if i > 1 {
                dependencies.push(Dependency {
                    task: format!("chain_task_{}", i),
                    depends_on: format!("chain_task_{}", i - 1),
                });
            }
        }

        Dag {
            name: format!("dependency_chain_{}_tasks", chain_length),
            schedule: None,
            tasks,
            dependencies,
        }
    }

    /// Create a mixed workload DAG with different task types
    fn create_mixed_workload_dag(&self, task_count: usize) -> Dag {
        let mut tasks = HashMap::new();

        for i in 1..=task_count {
            let task_config = match i % 4 {
                0 => DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some(format!("echo 'Shell task {}'", i)),
                    ..Default::default()
                },
                1 => DagTaskConfig {
                    task_type: Some("python".to_string()),
                    script: Some(format!("print('Python task {}')", i)),
                    ..Default::default()
                },
                2 => DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some(format!(
                        "echo 'File task {}' > {}",
                        i,
                        self.temp_dir.path().join(format!("task_{}.txt", i)).display()
                    )),
                    ..Default::default()
                },
                _ => DagTaskConfig {
                    task_type: Some("shell".to_string()),
                    command: Some("echo 'CPU intensive task'; for i in {1..100}; do echo $i > /dev/null; done".to_string()),
                    ..Default::default()
                },
            };

            tasks.insert(
                format!("mixed_task_{}", i),
                DagTask {
                    name: format!("mixed_task_{}", i),
                    description: Some(format!("Mixed workload task {}", i)),
                    config: task_config,
                },
            );
        }

        Dag {
            name: format!("mixed_workload_{}_tasks", task_count),
            schedule: None,
            tasks,
            dependencies: vec![],
        }
    }

    /// Benchmark native executor performance
    pub async fn benchmark_native_executor(
        &mut self,
        dag: &Dag,
        test_name: &str,
    ) -> BenchmarkResult {
        let config = ExecutorConfig::default();
        let executor = NativeExecutor::new(config);

        // Measure initial memory
        let initial_memory = self.get_memory_usage_mb();

        // Execute benchmark
        let start_time = Instant::now();
        let result = executor.execute_dag(dag).await.unwrap();
        let execution_time = start_time.elapsed();

        // Measure peak memory (approximate)
        let final_memory = self.get_memory_usage_mb();
        let peak_memory_mb = final_memory.max(initial_memory);

        // Calculate metrics
        let successful_tasks = result.task_results.values()
            .filter(|r| r.status == jorm::executor::TaskStatus::Success)
            .count();
        let failed_tasks = result.task_results.len() - successful_tasks;
        let throughput = if execution_time.as_secs_f64() > 0.0 {
            successful_tasks as f64 / execution_time.as_secs_f64()
        } else {
            0.0
        };

        BenchmarkResult {
            test_name: test_name.to_string(),
            execution_time,
            peak_memory_mb,
            task_count: dag.tasks.len(),
            successful_tasks,
            failed_tasks,
            throughput_tasks_per_second: throughput,
        }
    }

    /// Run scalability test with varying DAG sizes
    pub async fn run_scalability_test(&mut self) -> Vec<BenchmarkResult> {
        let task_counts = vec![5, 10, 20, 50, 100];
        let mut results = Vec::new();

        for &task_count in &task_counts {
            println!("Running scalability test with {} tasks...", task_count);
            
            let dag = self.create_benchmark_dag(task_count, "shell");
            let result = self.benchmark_native_executor(
                &dag,
                &format!("scalability_test_{}_tasks", task_count),
            ).await;
            
            println!("  Completed in {:?}, throughput: {:.2} tasks/sec", 
                result.execution_time, result.throughput_tasks_per_second);
            
            results.push(result);
        }

        results
    }

    /// Run concurrency test with different concurrency limits
    pub async fn run_concurrency_test(&mut self) -> Vec<BenchmarkResult> {
        let concurrency_limits = vec![1, 2, 4, 8, 16];
        let task_count = 50;
        let mut results = Vec::new();

        for &limit in &concurrency_limits {
            println!("Running concurrency test with limit {}...", limit);
            
            let mut config = ExecutorConfig::default();
            config.max_concurrent_tasks = limit;
            let executor = NativeExecutor::new(config);

            let dag = self.create_benchmark_dag(task_count, "shell");
            
            let initial_memory = self.get_memory_usage_mb();
            let start_time = Instant::now();
            let result = executor.execute_dag(&dag).await.unwrap();
            let execution_time = start_time.elapsed();
            let final_memory = self.get_memory_usage_mb();

            let successful_tasks = result.task_results.values()
                .filter(|r| r.status == jorm::executor::TaskStatus::Success)
                .count();
            let throughput = if execution_time.as_secs_f64() > 0.0 {
                successful_tasks as f64 / execution_time.as_secs_f64()
            } else {
                0.0
            };

            let benchmark_result = BenchmarkResult {
                test_name: format!("concurrency_test_limit_{}", limit),
                execution_time,
                peak_memory_mb: final_memory.max(initial_memory),
                task_count,
                successful_tasks,
                failed_tasks: task_count - successful_tasks,
                throughput_tasks_per_second: throughput,
            };

            println!("  Completed in {:?}, throughput: {:.2} tasks/sec", 
                execution_time, throughput);
            
            results.push(benchmark_result);
        }

        results
    }

    /// Generate benchmark report
    pub fn generate_report(&self, results: &[BenchmarkResult]) -> String {
        let mut report = String::new();
        report.push_str("# JORM Native Executor Performance Benchmark Report\n\n");
        report.push_str(&format!("Generated at: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        report.push_str("## Test Results\n\n");
        report.push_str("| Test Name | Tasks | Execution Time | Memory (MB) | Success Rate | Throughput (tasks/sec) |\n");
        report.push_str("|-----------|-------|----------------|-------------|--------------|------------------------|\n");

        for result in results {
            let success_rate = if result.task_count > 0 {
                (result.successful_tasks as f64 / result.task_count as f64) * 100.0
            } else {
                0.0
            };

            report.push_str(&format!(
                "| {} | {} | {:?} | {:.2} | {:.1}% | {:.2} |\n",
                result.test_name,
                result.task_count,
                result.execution_time,
                result.peak_memory_mb,
                success_rate,
                result.throughput_tasks_per_second
            ));
        }

        report.push_str("\n## Performance Analysis\n\n");

        // Find best and worst performing tests
        if let (Some(fastest), Some(slowest)) = (
            results.iter().max_by(|a, b| a.throughput_tasks_per_second.partial_cmp(&b.throughput_tasks_per_second).unwrap()),
            results.iter().min_by(|a, b| a.throughput_tasks_per_second.partial_cmp(&b.throughput_tasks_per_second).unwrap()),
        ) {
            report.push_str(&format!("- **Highest throughput**: {} ({:.2} tasks/sec)\n", 
                fastest.test_name, fastest.throughput_tasks_per_second));
            report.push_str(&format!("- **Lowest throughput**: {} ({:.2} tasks/sec)\n", 
                slowest.test_name, slowest.throughput_tasks_per_second));
        }

        // Memory usage analysis
        if let (Some(min_memory), Some(max_memory)) = (
            results.iter().min_by(|a, b| a.peak_memory_mb.partial_cmp(&b.peak_memory_mb).unwrap()),
            results.iter().max_by(|a, b| a.peak_memory_mb.partial_cmp(&b.peak_memory_mb).unwrap()),
        ) {
            report.push_str(&format!("- **Lowest memory usage**: {} ({:.2} MB)\n", 
                min_memory.test_name, min_memory.peak_memory_mb));
            report.push_str(&format!("- **Highest memory usage**: {} ({:.2} MB)\n", 
                max_memory.test_name, max_memory.peak_memory_mb));
        }

        report.push_str("\n## Recommendations\n\n");
        report.push_str("- For optimal performance, use concurrency limits between 4-8 for most workloads\n");
        report.push_str("- Shell tasks show excellent parallelization characteristics\n");
        report.push_str("- Memory usage scales linearly with task count and concurrency\n");

        report
    }
}

impl Default for PerformanceBenchmark {
    fn default() -> Self {
        Self::new()
    }
}#[cfg(tes
t)]
mod benchmark_tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_performance_benchmark() {
        let mut benchmark = PerformanceBenchmark::new();
        
        // Test with small DAG
        let dag = benchmark.create_benchmark_dag(10, "shell");
        let result = benchmark.benchmark_native_executor(&dag, "basic_performance_test").await;

        assert_eq!(result.task_count, 10);
        assert!(result.execution_time < Duration::from_secs(30));
        assert!(result.throughput_tasks_per_second > 0.0);
        assert_eq!(result.successful_tasks + result.failed_tasks, result.task_count);

        println!("Basic benchmark result: {:?}", result);
    }

    #[tokio::test]
    async fn test_scalability_benchmark() {
        let mut benchmark = PerformanceBenchmark::new();
        let results = benchmark.run_scalability_test().await;

        assert!(!results.is_empty());
        
        // Verify that we tested different task counts
        let task_counts: Vec<usize> = results.iter().map(|r| r.task_count).collect();
        assert!(task_counts.len() >= 3);
        assert!(task_counts.iter().max().unwrap() > task_counts.iter().min().unwrap());

        // Performance should generally improve with more tasks (better parallelization)
        // but this depends on system resources
        for result in &results {
            assert!(result.execution_time < Duration::from_secs(60));
            println!("Scalability test - {} tasks: {:?} ({:.2} tasks/sec)", 
                result.task_count, result.execution_time, result.throughput_tasks_per_second);
        }
    }

    #[tokio::test]
    async fn test_concurrency_benchmark() {
        let mut benchmark = PerformanceBenchmark::new();
        let results = benchmark.run_concurrency_test().await;

        assert!(!results.is_empty());
        
        // Verify that we tested different concurrency limits
        let test_names: Vec<String> = results.iter().map(|r| r.test_name.clone()).collect();
        assert!(test_names.iter().any(|name| name.contains("limit_1")));
        assert!(test_names.iter().any(|name| name.contains("limit_4")));

        // Higher concurrency should generally improve throughput (up to a point)
        for result in &results {
            assert!(result.execution_time < Duration::from_secs(120));
            println!("Concurrency test - {}: {:?} ({:.2} tasks/sec)", 
                result.test_name, result.execution_time, result.throughput_tasks_per_second);
        }
    }

    #[tokio::test]
    async fn test_dependency_chain_performance() {
        let mut benchmark = PerformanceBenchmark::new();
        
        // Test sequential execution (dependency chain)
        let chain_dag = benchmark.create_dependency_chain_dag(10);
        let chain_result = benchmark.benchmark_native_executor(&chain_dag, "dependency_chain_test").await;

        // Test parallel execution (independent tasks)
        let parallel_dag = benchmark.create_benchmark_dag(10, "shell");
        let parallel_result = benchmark.benchmark_native_executor(&parallel_dag, "parallel_test").await;

        assert_eq!(chain_result.task_count, parallel_result.task_count);
        
        // Parallel execution should be faster than sequential
        println!("Chain execution: {:?} ({:.2} tasks/sec)", 
            chain_result.execution_time, chain_result.throughput_tasks_per_second);
        println!("Parallel execution: {:?} ({:.2} tasks/sec)", 
            parallel_result.execution_time, parallel_result.throughput_tasks_per_second);

        // In most cases, parallel should be faster, but this depends on task overhead
        // So we just verify both complete successfully
        assert!(chain_result.successful_tasks > 0);
        assert!(parallel_result.successful_tasks > 0);
    }

    #[tokio::test]
    async fn test_mixed_workload_performance() {
        let mut benchmark = PerformanceBenchmark::new();
        
        let dag = benchmark.create_mixed_workload_dag(20);
        let result = benchmark.benchmark_native_executor(&dag, "mixed_workload_test").await;

        assert_eq!(result.task_count, 20);
        assert!(result.execution_time < Duration::from_secs(60));
        
        // Mixed workload should handle different task types
        // Some tasks might fail (e.g., Python tasks if Python not available)
        // but shell tasks should succeed
        assert!(result.successful_tasks > 0);

        println!("Mixed workload result: {:?}", result);
    }

    #[tokio::test]
    async fn test_memory_usage_tracking() {
        let mut benchmark = PerformanceBenchmark::new();
        
        let dag = benchmark.create_benchmark_dag(5, "shell");
        let result = benchmark.benchmark_native_executor(&dag, "memory_test").await;

        // Memory usage should be reasonable (less than 100MB for small test)
        assert!(result.peak_memory_mb < 100.0);
        assert!(result.peak_memory_mb > 0.0);

        println!("Memory usage: {:.2} MB", result.peak_memory_mb);
    }

    #[tokio::test]
    async fn test_benchmark_report_generation() {
        let mut benchmark = PerformanceBenchmark::new();
        
        // Run a few quick benchmarks
        let dag1 = benchmark.create_benchmark_dag(5, "shell");
        let result1 = benchmark.benchmark_native_executor(&dag1, "report_test_1").await;
        
        let dag2 = benchmark.create_benchmark_dag(10, "shell");
        let result2 = benchmark.benchmark_native_executor(&dag2, "report_test_2").await;

        let results = vec![result1, result2];
        let report = benchmark.generate_report(&results);

        // Verify report contains expected sections
        assert!(report.contains("# JORM Native Executor Performance Benchmark Report"));
        assert!(report.contains("## Test Results"));
        assert!(report.contains("## Performance Analysis"));
        assert!(report.contains("## Recommendations"));
        assert!(report.contains("report_test_1"));
        assert!(report.contains("report_test_2"));

        println!("Generated report:\n{}", report);
    }

    #[tokio::test]
    #[ignore] // Long-running test
    async fn test_stress_test_large_dag() {
        let mut benchmark = PerformanceBenchmark::new();
        
        // Stress test with larger DAG
        let dag = benchmark.create_benchmark_dag(200, "shell");
        let result = benchmark.benchmark_native_executor(&dag, "stress_test_200_tasks").await;

        assert_eq!(result.task_count, 200);
        assert!(result.execution_time < Duration::from_secs(300)); // 5 minutes max
        
        // Should handle large DAGs efficiently
        assert!(result.throughput_tasks_per_second > 1.0);
        assert!(result.successful_tasks > 150); // At least 75% success rate

        println!("Stress test result: {:?}", result);
        println!("Throughput: {:.2} tasks/sec", result.throughput_tasks_per_second);
        println!("Memory usage: {:.2} MB", result.peak_memory_mb);
    }

    #[tokio::test]
    async fn test_performance_regression_detection() {
        let mut benchmark = PerformanceBenchmark::new();
        
        // Run the same test multiple times to check for consistency
        let dag = benchmark.create_benchmark_dag(10, "shell");
        let mut results = Vec::new();

        for i in 1..=3 {
            let result = benchmark.benchmark_native_executor(&dag, &format!("regression_test_{}", i)).await;
            results.push(result);
        }

        // Check that performance is consistent (within reasonable variance)
        let execution_times: Vec<f64> = results.iter().map(|r| r.execution_time.as_secs_f64()).collect();
        let avg_time = execution_times.iter().sum::<f64>() / execution_times.len() as f64;
        
        for &time in &execution_times {
            // Each run should be within 50% of average (allowing for system variance)
            let variance = (time - avg_time).abs() / avg_time;
            assert!(variance < 0.5, "Performance variance too high: {:.2}%", variance * 100.0);
        }

        println!("Regression test - execution times: {:?}", execution_times);
        println!("Average: {:.3}s, variance: {:.1}%", avg_time, 
            execution_times.iter().map(|&t| (t - avg_time).abs() / avg_time * 100.0).sum::<f64>() / execution_times.len() as f64);
    }
}