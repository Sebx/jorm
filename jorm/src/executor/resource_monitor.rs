//! Resource monitoring and limits enforcement for task execution
//!
//! This module provides comprehensive CPU and memory usage tracking with configurable
//! thresholds and resource-based task throttling and queuing capabilities.

use crate::executor::error::ExecutorError;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;

/// Resource usage thresholds and limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU usage percentage (0.0 - 100.0)
    pub max_cpu_percent: f64,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Maximum memory usage percentage (0.0 - 100.0)
    pub max_memory_percent: f64,
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: usize,
    /// Monitoring interval for resource checks
    pub monitoring_interval: Duration,
    /// Grace period before enforcing limits after threshold breach
    pub grace_period: Duration,
    /// Whether to enable adaptive throttling based on resource usage
    pub adaptive_throttling: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_percent: 80.0,
            max_memory_bytes: 8 * 1024 * 1024 * 1024, // 8GB
            max_memory_percent: 85.0,
            max_concurrent_tasks: num_cpus::get(),
            monitoring_interval: Duration::from_secs(1),
            grace_period: Duration::from_secs(5),
            adaptive_throttling: true,
        }
    }
}

/// Current system resource usage snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Timestamp of measurement
    pub timestamp: DateTime<Utc>,
    /// CPU usage percentage (0.0 - 100.0)
    pub cpu_percent: f64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Memory usage percentage (0.0 - 100.0)
    pub memory_percent: f64,
    /// Total system memory in bytes
    pub total_memory_bytes: u64,
    /// Number of CPU cores
    pub cpu_cores: usize,
    /// Load average (1 minute)
    pub load_average: Option<f64>,
    /// Currently running tasks count
    pub active_tasks: usize,
}

/// Resource monitoring and enforcement engine
pub struct ResourceMonitor {
    /// Resource limits configuration
    limits: ResourceLimits,
    /// Current resource usage
    current_usage: Arc<RwLock<ResourceUsage>>,
    /// Resource usage history for trend analysis
    usage_history: Arc<Mutex<VecDeque<ResourceUsage>>>,
    /// Task queue for throttling
    task_queue: Arc<Mutex<VecDeque<QueuedTask>>>,
    /// Currently running tasks
    running_tasks: Arc<RwLock<usize>>,
    /// Monitoring task handle
    monitoring_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Whether monitoring is active
    is_monitoring: Arc<RwLock<bool>>,
    /// Threshold breach tracking
    threshold_breaches: Arc<Mutex<Vec<ThresholdBreach>>>,
}

/// Queued task waiting for resource availability
#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task_id: String,
    pub queued_at: DateTime<Utc>,
    pub priority: TaskPriority,
    pub estimated_resources: EstimatedResources,
}

/// Task priority for queue ordering
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Estimated resource requirements for a task
#[derive(Debug, Clone)]
pub struct EstimatedResources {
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub duration_estimate: Option<Duration>,
}

/// Threshold breach event for tracking violations
#[derive(Debug, Clone)]
pub struct ThresholdBreach {
    pub timestamp: DateTime<Utc>,
    pub resource_type: ResourceType,
    pub threshold_value: f64,
    pub actual_value: f64,
    pub duration: Duration,
}

/// Type of resource that breached threshold
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    Cpu,
    Memory,
    ConcurrentTasks,
}

/// Resource availability check result
#[derive(Debug, Clone)]
pub struct ResourceAvailability {
    pub can_start_task: bool,
    pub reason: Option<String>,
    pub estimated_wait_time: Option<Duration>,
    pub current_usage: ResourceUsage,
}

impl ResourceMonitor {
    /// Create a new resource monitor with the given limits
    pub fn new(limits: ResourceLimits) -> Self {
        let initial_usage = ResourceUsage {
            timestamp: Utc::now(),
            cpu_percent: 0.0,
            memory_bytes: 0,
            memory_percent: 0.0,
            total_memory_bytes: Self::get_total_memory(),
            cpu_cores: num_cpus::get(),
            load_average: None,
            active_tasks: 0,
        };

        Self {
            limits,
            current_usage: Arc::new(RwLock::new(initial_usage)),
            usage_history: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            running_tasks: Arc::new(RwLock::new(0)),
            monitoring_handle: Arc::new(Mutex::new(None)),
            is_monitoring: Arc::new(RwLock::new(false)),
            threshold_breaches: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Start resource monitoring
    pub async fn start_monitoring(&self) -> Result<()> {
        let mut is_monitoring = self.is_monitoring.write().await;
        if *is_monitoring {
            return Ok(()); // Already monitoring
        }
        *is_monitoring = true;
        drop(is_monitoring);

        let current_usage = self.current_usage.clone();
        let usage_history = self.usage_history.clone();
        let threshold_breaches = self.threshold_breaches.clone();
        let limits = self.limits.clone();
        let is_monitoring_flag = self.is_monitoring.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(limits.monitoring_interval);
            
            while *is_monitoring_flag.read().await {
                interval.tick().await;
                
                // Collect current resource usage
                let usage = Self::collect_resource_usage().await;
                
                // Update current usage
                {
                    let mut current = current_usage.write().await;
                    *current = usage.clone();
                }
                
                // Add to history
                {
                    let mut history = usage_history.lock().await;
                    history.push_back(usage.clone());
                    
                    // Keep only recent history (last 1000 measurements)
                    if history.len() > 1000 {
                        history.pop_front();
                    }
                }
                
                // Check for threshold breaches
                Self::check_threshold_breaches(&usage, &limits, &threshold_breaches).await;
            }
        });

        let mut monitoring_handle = self.monitoring_handle.lock().await;
        *monitoring_handle = Some(handle);

        Ok(())
    }

    /// Stop resource monitoring
    pub async fn stop_monitoring(&self) {
        let mut is_monitoring = self.is_monitoring.write().await;
        *is_monitoring = false;
        drop(is_monitoring);

        let mut handle = self.monitoring_handle.lock().await;
        if let Some(h) = handle.take() {
            h.abort();
        }
    }

    /// Check if resources are available to start a new task
    pub async fn check_resource_availability(
        &self,
        estimated_resources: &EstimatedResources,
    ) -> ResourceAvailability {
        let current_usage = self.current_usage.read().await.clone();
        let running_tasks = *self.running_tasks.read().await;

        // Check concurrent task limit
        if running_tasks >= self.limits.max_concurrent_tasks {
            return ResourceAvailability {
                can_start_task: false,
                reason: Some(format!(
                    "Maximum concurrent tasks limit reached ({}/{})",
                    running_tasks, self.limits.max_concurrent_tasks
                )),
                estimated_wait_time: Some(Duration::from_secs(30)), // Estimate
                current_usage,
            };
        }

        // Check CPU availability
        let projected_cpu = current_usage.cpu_percent + estimated_resources.cpu_percent;
        if projected_cpu > self.limits.max_cpu_percent {
            return ResourceAvailability {
                can_start_task: false,
                reason: Some(format!(
                    "CPU usage would exceed limit ({:.1}% + {:.1}% > {:.1}%)",
                    current_usage.cpu_percent,
                    estimated_resources.cpu_percent,
                    self.limits.max_cpu_percent
                )),
                estimated_wait_time: self.estimate_wait_time_for_cpu().await,
                current_usage,
            };
        }

        // Check memory availability
        let projected_memory = current_usage.memory_bytes + estimated_resources.memory_bytes;
        let projected_memory_percent = (projected_memory as f64 / current_usage.total_memory_bytes as f64) * 100.0;
        
        if projected_memory > self.limits.max_memory_bytes || projected_memory_percent > self.limits.max_memory_percent {
            return ResourceAvailability {
                can_start_task: false,
                reason: Some(format!(
                    "Memory usage would exceed limit ({:.1}MB + {:.1}MB > {:.1}MB or {:.1}% > {:.1}%)",
                    current_usage.memory_bytes as f64 / 1024.0 / 1024.0,
                    estimated_resources.memory_bytes as f64 / 1024.0 / 1024.0,
                    self.limits.max_memory_bytes as f64 / 1024.0 / 1024.0,
                    projected_memory_percent,
                    self.limits.max_memory_percent
                )),
                estimated_wait_time: self.estimate_wait_time_for_memory().await,
                current_usage,
            };
        }

        ResourceAvailability {
            can_start_task: true,
            reason: None,
            estimated_wait_time: None,
            current_usage,
        }
    }

    /// Queue a task for execution when resources become available
    pub async fn queue_task(&self, task: QueuedTask) {
        let mut queue = self.task_queue.lock().await;
        
        // Insert task in priority order
        let insert_pos = queue
            .iter()
            .position(|queued| queued.priority < task.priority)
            .unwrap_or(queue.len());
        
        queue.insert(insert_pos, task);
    }

    /// Get the next task from the queue that can be started
    pub async fn get_next_available_task(&self) -> Option<QueuedTask> {
        let mut queue = self.task_queue.lock().await;
        
        // Find the first task that can be started
        for (index, task) in queue.iter().enumerate() {
            let availability = self.check_resource_availability(&task.estimated_resources).await;
            if availability.can_start_task {
                return queue.remove(index);
            }
        }
        
        None
    }

    /// Register that a task has started (increment running count)
    pub async fn register_task_start(&self, _task_id: &str) -> Result<()> {
        let mut running_tasks = self.running_tasks.write().await;
        *running_tasks += 1;
        
        // Update current usage active tasks count
        let mut current_usage = self.current_usage.write().await;
        current_usage.active_tasks = *running_tasks;
        
        Ok(())
    }

    /// Register that a task has completed (decrement running count)
    pub async fn register_task_completion(&self, _task_id: &str) -> Result<()> {
        let mut running_tasks = self.running_tasks.write().await;
        *running_tasks = running_tasks.saturating_sub(1);
        
        // Update current usage active tasks count
        let mut current_usage = self.current_usage.write().await;
        current_usage.active_tasks = *running_tasks;
        
        Ok(())
    }

    /// Get current resource usage
    pub async fn get_current_usage(&self) -> ResourceUsage {
        self.current_usage.read().await.clone()
    }

    /// Get resource usage history
    pub async fn get_usage_history(&self, limit: Option<usize>) -> Vec<ResourceUsage> {
        let history = self.usage_history.lock().await;
        let limit = limit.unwrap_or(history.len());
        
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Get threshold breach history
    pub async fn get_threshold_breaches(&self) -> Vec<ThresholdBreach> {
        self.threshold_breaches.lock().await.clone()
    }

    /// Get queue status
    pub async fn get_queue_status(&self) -> QueueStatus {
        let queue = self.task_queue.lock().await;
        let running_tasks = *self.running_tasks.read().await;
        
        QueueStatus {
            queued_tasks: queue.len(),
            running_tasks,
            max_concurrent: self.limits.max_concurrent_tasks,
            queue_by_priority: {
                let mut counts = std::collections::HashMap::new();
                for task in queue.iter() {
                    *counts.entry(task.priority.clone()).or_insert(0) += 1;
                }
                counts
            },
        }
    }

    /// Update resource limits configuration
    pub async fn update_limits(&mut self, new_limits: ResourceLimits) {
        self.limits = new_limits;
    }

    /// Collect current system resource usage
    async fn collect_resource_usage() -> ResourceUsage {
        // In a real implementation, this would use system APIs
        // For now, we'll use a simplified approach with sysinfo or similar
        
        #[cfg(feature = "sysinfo")]
        {
            use sysinfo::{System, SystemExt, CpuExt};
            
            let mut system = System::new_all();
            system.refresh_all();
            
            let cpu_percent = system.global_cpu_info().cpu_usage() as f64;
            let memory_bytes = system.used_memory() * 1024; // sysinfo returns KB
            let total_memory_bytes = system.total_memory() * 1024;
            let memory_percent = (memory_bytes as f64 / total_memory_bytes as f64) * 100.0;
            
            ResourceUsage {
                timestamp: Utc::now(),
                cpu_percent,
                memory_bytes,
                memory_percent,
                total_memory_bytes,
                cpu_cores: system.cpus().len(),
                load_average: None, // Would need platform-specific implementation
                active_tasks: 0, // Will be updated by task registration
            }
        }
        
        #[cfg(not(feature = "sysinfo"))]
        {
            // Fallback implementation with mock data
            ResourceUsage {
                timestamp: Utc::now(),
                cpu_percent: 45.0, // Mock CPU usage
                memory_bytes: 4 * 1024 * 1024 * 1024, // Mock 4GB usage
                memory_percent: 50.0,
                total_memory_bytes: Self::get_total_memory(),
                cpu_cores: num_cpus::get(),
                load_average: None,
                active_tasks: 0,
            }
        }
    }

    /// Get total system memory
    fn get_total_memory() -> u64 {
        #[cfg(feature = "sysinfo")]
        {
            use sysinfo::{System, SystemExt};
            let mut system = System::new();
            system.refresh_memory();
            system.total_memory() * 1024 // Convert KB to bytes
        }
        
        #[cfg(not(feature = "sysinfo"))]
        {
            8 * 1024 * 1024 * 1024 // Default to 8GB
        }
    }

    /// Check for threshold breaches and record them
    async fn check_threshold_breaches(
        usage: &ResourceUsage,
        limits: &ResourceLimits,
        breaches: &Arc<Mutex<Vec<ThresholdBreach>>>,
    ) {
        let mut breach_list = breaches.lock().await;
        
        // Check CPU threshold
        if usage.cpu_percent > limits.max_cpu_percent {
            breach_list.push(ThresholdBreach {
                timestamp: usage.timestamp,
                resource_type: ResourceType::Cpu,
                threshold_value: limits.max_cpu_percent,
                actual_value: usage.cpu_percent,
                duration: Duration::from_secs(0), // Would track duration in real implementation
            });
        }
        
        // Check memory threshold
        if usage.memory_percent > limits.max_memory_percent {
            breach_list.push(ThresholdBreach {
                timestamp: usage.timestamp,
                resource_type: ResourceType::Memory,
                threshold_value: limits.max_memory_percent,
                actual_value: usage.memory_percent,
                duration: Duration::from_secs(0),
            });
        }
        
        // Keep only recent breaches (last 100)
        if breach_list.len() > 100 {
            let excess = breach_list.len() - 100;
            breach_list.drain(0..excess);
        }
    }

    /// Estimate wait time for CPU availability
    async fn estimate_wait_time_for_cpu(&self) -> Option<Duration> {
        let history = self.usage_history.lock().await;
        
        if history.len() < 10 {
            return Some(Duration::from_secs(30)); // Default estimate
        }
        
        // Simple trend analysis - check if CPU usage is decreasing
        let recent: Vec<_> = history.iter().rev().take(10).collect();
        let avg_recent = recent.iter().map(|u| u.cpu_percent).sum::<f64>() / recent.len() as f64;
        
        if avg_recent < self.limits.max_cpu_percent * 0.9 {
            Some(Duration::from_secs(10)) // Should be available soon
        } else {
            Some(Duration::from_secs(60)) // Might take longer
        }
    }

    /// Estimate wait time for memory availability
    async fn estimate_wait_time_for_memory(&self) -> Option<Duration> {
        // Similar to CPU estimation but for memory
        Some(Duration::from_secs(45))
    }
}

/// Queue status information
#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub queued_tasks: usize,
    pub running_tasks: usize,
    pub max_concurrent: usize,
    pub queue_by_priority: std::collections::HashMap<TaskPriority, usize>,
}

impl Default for EstimatedResources {
    fn default() -> Self {
        Self {
            cpu_percent: 10.0, // Default 10% CPU
            memory_bytes: 256 * 1024 * 1024, // Default 256MB
            duration_estimate: None,
        }
    }
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_resource_monitor_creation() {
        let limits = ResourceLimits::default();
        let monitor = ResourceMonitor::new(limits);
        
        let usage = monitor.get_current_usage().await;
        assert_eq!(usage.cpu_cores, num_cpus::get());
    }

    #[tokio::test]
    async fn test_resource_availability_check() {
        let limits = ResourceLimits {
            max_cpu_percent: 80.0,
            max_memory_percent: 85.0,
            max_concurrent_tasks: 2,
            ..Default::default()
        };
        let monitor = ResourceMonitor::new(limits);
        
        let resources = EstimatedResources {
            cpu_percent: 10.0,
            memory_bytes: 100 * 1024 * 1024, // 100MB
            duration_estimate: Some(Duration::from_secs(30)),
        };
        
        let availability = monitor.check_resource_availability(&resources).await;
        assert!(availability.can_start_task);
        assert!(availability.reason.is_none());
    }

    #[tokio::test]
    async fn test_resource_limits_enforcement() {
        let limits = ResourceLimits {
            max_cpu_percent: 80.0, // Higher limit to avoid CPU blocking in test
            max_memory_percent: 80.0,
            max_concurrent_tasks: 1,
            ..Default::default()
        };
        let monitor = ResourceMonitor::new(limits);
        
        // Start monitoring
        monitor.start_monitoring().await.unwrap();
        sleep(Duration::from_millis(100)).await;
        
        // Register a task start to reach concurrent limit
        monitor.register_task_start("task1").await.unwrap();
        
        // Use smaller resource requirements for testing
        let resources = EstimatedResources {
            cpu_percent: 5.0, // Small CPU requirement
            memory_bytes: 100 * 1024 * 1024, // 100MB
            duration_estimate: Some(Duration::from_secs(30)),
        };
        let availability = monitor.check_resource_availability(&resources).await;
        
        // Should be blocked due to concurrent task limit
        assert!(!availability.can_start_task);
        assert!(availability.reason.is_some());
        assert!(availability.reason.unwrap().contains("Maximum concurrent tasks limit reached"));
        
        // Complete the task
        monitor.register_task_completion("task1").await.unwrap();
        
        let availability = monitor.check_resource_availability(&resources).await;
        assert!(availability.can_start_task);
        
        monitor.stop_monitoring().await;
    }

    #[tokio::test]
    async fn test_task_queuing() {
        let monitor = ResourceMonitor::new(ResourceLimits::default());
        
        let task = QueuedTask {
            task_id: "test-task".to_string(),
            queued_at: Utc::now(),
            priority: TaskPriority::High,
            estimated_resources: EstimatedResources::default(),
        };
        
        monitor.queue_task(task).await;
        
        let status = monitor.get_queue_status().await;
        assert_eq!(status.queued_tasks, 1);
        assert_eq!(status.running_tasks, 0);
        
        let next_task = monitor.get_next_available_task().await;
        assert!(next_task.is_some());
        assert_eq!(next_task.unwrap().task_id, "test-task");
        
        let status = monitor.get_queue_status().await;
        assert_eq!(status.queued_tasks, 0);
    }

    #[tokio::test]
    async fn test_priority_queue_ordering() {
        let monitor = ResourceMonitor::new(ResourceLimits::default());
        
        // Add tasks with different priorities
        let low_task = QueuedTask {
            task_id: "low-priority".to_string(),
            queued_at: Utc::now(),
            priority: TaskPriority::Low,
            estimated_resources: EstimatedResources::default(),
        };
        
        let high_task = QueuedTask {
            task_id: "high-priority".to_string(),
            queued_at: Utc::now(),
            priority: TaskPriority::High,
            estimated_resources: EstimatedResources::default(),
        };
        
        // Queue low priority first, then high priority
        monitor.queue_task(low_task).await;
        monitor.queue_task(high_task).await;
        
        // High priority should come out first
        let next_task = monitor.get_next_available_task().await;
        assert!(next_task.is_some());
        assert_eq!(next_task.unwrap().task_id, "high-priority");
        
        // Low priority should come out second
        let next_task = monitor.get_next_available_task().await;
        assert!(next_task.is_some());
        assert_eq!(next_task.unwrap().task_id, "low-priority");
    }

    #[tokio::test]
    async fn test_usage_history_tracking() {
        let monitor = ResourceMonitor::new(ResourceLimits::default());
        
        monitor.start_monitoring().await.unwrap();
        
        // Wait for some measurements
        sleep(Duration::from_millis(200)).await;
        
        let history = monitor.get_usage_history(Some(5)).await;
        assert!(!history.is_empty());
        
        // Check that timestamps are in order
        for window in history.windows(2) {
            assert!(window[1].timestamp >= window[0].timestamp);
        }
        
        monitor.stop_monitoring().await;
    }

    #[tokio::test]
    async fn test_limits_update() {
        let mut monitor = ResourceMonitor::new(ResourceLimits::default());
        
        let new_limits = ResourceLimits {
            max_cpu_percent: 90.0,
            max_memory_percent: 95.0,
            max_concurrent_tasks: 10,
            ..Default::default()
        };
        
        monitor.update_limits(new_limits.clone()).await;
        
        assert_eq!(monitor.limits.max_cpu_percent, 90.0);
        assert_eq!(monitor.limits.max_memory_percent, 95.0);
        assert_eq!(monitor.limits.max_concurrent_tasks, 10);
    }
}