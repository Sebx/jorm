use super::{ScheduledJob, SchedulerEvent, JobResult, OverlapPolicy};
use anyhow::{Result, Context};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time::{Duration, Instant};
use uuid::Uuid;
use crate::executor::{NativeExecutor, ExecutorConfig};
use crate::parser::parse_dag_file;

pub struct CronScheduler {
    jobs: Arc<RwLock<HashMap<Uuid, ScheduledJob>>>,
    running_jobs: Arc<RwLock<HashMap<Uuid, tokio::task::JoinHandle<()>>>>,
    event_sender: mpsc::UnboundedSender<SchedulerEvent>,
    event_receiver: Arc<Mutex<mpsc::UnboundedReceiver<SchedulerEvent>>>,
    executor: Arc<NativeExecutor>,
    shutdown_signal: Arc<tokio::sync::Notify>,
}

impl CronScheduler {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let executor_config = ExecutorConfig::default();
        let executor = Arc::new(NativeExecutor::new(executor_config));

        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            running_jobs: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            executor,
            shutdown_signal: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub async fn add_job(&self, job: ScheduledJob) -> Result<Uuid> {
        let job_id = job.id;
        let mut jobs = self.jobs.write().await;
        
        // Calculate initial next run time
        let mut job = job;
        job.next_run = job.calculate_next_run(Utc::now());
        
        jobs.insert(job_id, job);
        
        self.event_sender.send(SchedulerEvent::JobScheduled(job_id))
            .context("Failed to send job scheduled event")?;
        
        Ok(job_id)
    }

    pub async fn remove_job(&self, job_id: Uuid) -> Result<bool> {
        let mut jobs = self.jobs.write().await;
        let removed = jobs.remove(&job_id).is_some();
        
        // Cancel running job if it exists
        if removed {
            let mut running_jobs = self.running_jobs.write().await;
            if let Some(handle) = running_jobs.remove(&job_id) {
                handle.abort();
                self.event_sender.send(SchedulerEvent::JobCancelled(job_id))
                    .context("Failed to send job cancelled event")?;
            }
        }
        
        Ok(removed)
    }

    pub async fn enable_job(&self, job_id: Uuid) -> Result<bool> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.enabled = true;
            job.next_run = job.calculate_next_run(Utc::now());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn disable_job(&self, job_id: Uuid) -> Result<bool> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.enabled = false;
            job.next_run = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn list_jobs(&self) -> Vec<ScheduledJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    pub async fn get_job(&self, job_id: Uuid) -> Option<ScheduledJob> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id).cloned()
    }

    pub async fn trigger_job(&self, job_id: Uuid) -> Result<bool> {
        let job = {
            let jobs = self.jobs.read().await;
            jobs.get(&job_id).cloned()
        };

        if let Some(job) = job {
            self.execute_job(job).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn start(&self) -> Result<()> {
        self.event_sender.send(SchedulerEvent::SchedulerStarted)
            .context("Failed to send scheduler started event")?;

        let jobs = Arc::clone(&self.jobs);
        let running_jobs = Arc::clone(&self.running_jobs);
        let event_sender = self.event_sender.clone();
        let executor = Arc::clone(&self.executor);
        let shutdown_signal = Arc::clone(&self.shutdown_signal);

        // Main scheduler loop
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let now = Utc::now();
                        let jobs_to_run = {
                            let mut jobs_guard = jobs.write().await;
                            let mut jobs_to_run = Vec::new();
                            
                            for job in jobs_guard.values_mut() {
                                if job.enabled {
                                    if let Some(next_run) = job.next_run {
                                        if next_run <= now {
                                            // Check overlap policy
                                            let running_jobs_guard = running_jobs.read().await;
                                            let is_running = running_jobs_guard.contains_key(&job.id);
                                            
                                            let should_run = match job.config.overlap_policy {
                                                OverlapPolicy::Allow => true,
                                                OverlapPolicy::Skip => !is_running,
                                                OverlapPolicy::Queue => !is_running, // TODO: Implement proper queuing
                                                OverlapPolicy::Cancel => {
                                                    if is_running {
                                                        // Cancel the running job
                                                        if let Some(handle) = running_jobs_guard.get(&job.id) {
                                                            handle.abort();
                                                        }
                                                    }
                                                    true
                                                }
                                            };
                                            
                                            if should_run {
                                                jobs_to_run.push(job.clone());
                                                // Calculate next run time
                                                job.next_run = job.calculate_next_run(now);
                                            }
                                        }
                                    }
                                }
                            }
                            
                            jobs_to_run
                        };
                        
                        // Execute jobs
                        for job in jobs_to_run {
                            let job_id = job.id; // Extract job_id before moving
                            let job_executor = Arc::clone(&executor);
                            let job_event_sender = event_sender.clone();
                            let job_running_jobs = Arc::clone(&running_jobs);
                            let job_jobs = Arc::clone(&jobs);
                            
                            let handle = tokio::spawn(async move {
                                // Send job started event
                                let _ = job_event_sender.send(SchedulerEvent::JobStarted(job_id));
                                
                                let start_time = Instant::now();
                                let result = Self::execute_job_internal(job.clone(), job_executor).await;
                                let duration = start_time.elapsed();
                                
                                // Update job statistics
                                {
                                    let mut jobs_guard = job_jobs.write().await;
                                    if let Some(job_ref) = jobs_guard.get_mut(&job_id) {
                                        job_ref.last_run = Some(Utc::now());
                                        job_ref.run_count += 1;
                                        if result.is_err() {
                                            job_ref.failure_count += 1;
                                        }
                                    }
                                }
                                
                                // Remove from running jobs
                                {
                                    let mut running_jobs_guard = job_running_jobs.write().await;
                                    running_jobs_guard.remove(&job_id);
                                }
                                
                                // Send completion event
                                match result {
                                    Ok(output) => {
                                        let job_result = JobResult {
                                            success: true,
                                            duration,
                                            output,
                                            error: None,
                                        };
                                        let _ = job_event_sender.send(SchedulerEvent::JobCompleted(job_id, job_result));
                                    }
                                    Err(e) => {
                                        let _ = job_event_sender.send(SchedulerEvent::JobFailed(job_id, e.to_string()));
                                    }
                                }
                            });
                            
                            // Add to running jobs
                            {
                                let mut running_jobs_guard = running_jobs.write().await;
                                running_jobs_guard.insert(job_id, handle);
                            }
                        }
                    }
                    _ = shutdown_signal.notified() => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        // Signal shutdown
        self.shutdown_signal.notify_waiters();
        
        // Cancel all running jobs
        let running_jobs = {
            let mut running_jobs_guard = self.running_jobs.write().await;
            let jobs: Vec<_> = running_jobs_guard.drain().collect();
            jobs
        };
        
        for (job_id, handle) in running_jobs {
            handle.abort();
            self.event_sender.send(SchedulerEvent::JobCancelled(job_id))
                .context("Failed to send job cancelled event")?;
        }
        
        self.event_sender.send(SchedulerEvent::SchedulerStopped)
            .context("Failed to send scheduler stopped event")?;
        
        Ok(())
    }

    async fn execute_job(&self, job: ScheduledJob) -> Result<()> {
        let executor = Arc::clone(&self.executor);
        let output = Self::execute_job_internal(job, executor).await?;
        println!("Job execution output: {}", output);
        Ok(())
    }

    async fn execute_job_internal(job: ScheduledJob, executor: Arc<NativeExecutor>) -> Result<String> {
        // Parse the DAG file
        let dag = parse_dag_file(&job.dag_file).await
            .context("Failed to parse DAG file")?;
        
        // Execute the DAG
        let result = executor.execute_dag(&dag).await
            .context("Failed to execute DAG")?;
        
        // Format the result
        let output = format!(
            "DAG '{}' executed successfully. {} tasks completed, {} failed.",
            dag.name,
            result.task_results.iter().filter(|(_, r)| r.status == crate::executor::TaskStatus::Success).count(),
            result.task_results.iter().filter(|(_, r)| r.status == crate::executor::TaskStatus::Failed).count()
        );
        
        Ok(output)
    }

    pub fn event_receiver(&self) -> Arc<Mutex<mpsc::UnboundedReceiver<SchedulerEvent>>> {
        Arc::clone(&self.event_receiver)
    }
}

impl Default for CronScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let scheduler = CronScheduler::new();
        assert_eq!(scheduler.list_jobs().await.len(), 0);
    }

    #[tokio::test]
    async fn test_add_remove_job() {
        let scheduler = CronScheduler::new();
        
        let job = ScheduledJob::new(
            "test_job".to_string(),
            "test.yaml".to_string(),
            crate::scheduler::Schedule::Cron("0 0 * * *".to_string()),
        );
        
        let job_id = scheduler.add_job(job).await.unwrap();
        assert_eq!(scheduler.list_jobs().await.len(), 1);
        
        let removed = scheduler.remove_job(job_id).await.unwrap();
        assert!(removed);
        assert_eq!(scheduler.list_jobs().await.len(), 0);
    }

    #[tokio::test]
    async fn test_enable_disable_job() {
        let scheduler = CronScheduler::new();
        
        let job = ScheduledJob::new(
            "test_job".to_string(),
            "test.yaml".to_string(),
            crate::scheduler::Schedule::Cron("0 0 * * *".to_string()),
        );
        
        let job_id = scheduler.add_job(job).await.unwrap();
        
        let disabled = scheduler.disable_job(job_id).await.unwrap();
        assert!(disabled);
        
        let job = scheduler.get_job(job_id).await.unwrap();
        assert!(!job.enabled);
        
        let enabled = scheduler.enable_job(job_id).await.unwrap();
        assert!(enabled);
        
        let job = scheduler.get_job(job_id).await.unwrap();
        assert!(job.enabled);
    }
}