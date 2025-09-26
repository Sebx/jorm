use super::CronScheduler;
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct FileTrigger {
    scheduler: Arc<CronScheduler>,
    watcher: Option<notify::RecommendedWatcher>,
    watched_paths: Vec<PathBuf>,
}

impl FileTrigger {
    pub fn new(scheduler: Arc<CronScheduler>) -> Self {
        Self {
            scheduler,
            watcher: None,
            watched_paths: Vec::new(),
        }
    }

    pub async fn watch_file<P: AsRef<Path>>(&mut self, path: P, job_id: Uuid) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        if self.watcher.is_none() {
            let (tx, mut rx) = mpsc::unbounded_channel();
            let scheduler = Arc::clone(&self.scheduler);

            // Create the file watcher
            let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send((event, job_id));
                }
            })
            .context("Failed to create file watcher")?;

            // Start the event processing task
            tokio::spawn(async move {
                while let Some((event, job_id)) = rx.recv().await {
                    if should_trigger_on_event(&event) {
                        if let Err(e) = scheduler.trigger_job(job_id).await {
                            eprintln!("Failed to trigger job {}: {}", job_id, e);
                        }
                    }
                }
            });

            self.watcher = Some(watcher);
        }

        if let Some(watcher) = &mut self.watcher {
            watcher
                .watch(&path, RecursiveMode::NonRecursive)
                .context("Failed to watch file")?;
            self.watched_paths.push(path);
        }

        Ok(())
    }

    pub async fn watch_directory<P: AsRef<Path>>(
        &mut self,
        path: P,
        job_id: Uuid,
        recursive: bool,
    ) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        if self.watcher.is_none() {
            let (tx, mut rx) = mpsc::unbounded_channel();
            let scheduler = Arc::clone(&self.scheduler);

            // Create the file watcher
            let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send((event, job_id));
                }
            })
            .context("Failed to create file watcher")?;

            // Start the event processing task
            tokio::spawn(async move {
                while let Some((event, job_id)) = rx.recv().await {
                    if should_trigger_on_event(&event) {
                        if let Err(e) = scheduler.trigger_job(job_id).await {
                            eprintln!("Failed to trigger job {}: {}", job_id, e);
                        }
                    }
                }
            });

            self.watcher = Some(watcher);
        }

        if let Some(watcher) = &mut self.watcher {
            let mode = if recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            watcher
                .watch(&path, mode)
                .context("Failed to watch directory")?;
            self.watched_paths.push(path);
        }

        Ok(())
    }

    pub fn stop_watching(&mut self) -> Result<()> {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher);
        }
        self.watched_paths.clear();
        Ok(())
    }
}

pub struct WebhookTrigger {
    scheduler: Arc<CronScheduler>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    port: u16,
}

impl WebhookTrigger {
    pub fn new(scheduler: Arc<CronScheduler>, port: u16) -> Self {
        Self {
            scheduler,
            server_handle: None,
            port,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        // TODO: Implement webhook server with hyper
        println!("Webhook server would start on port {}", self.port);
        println!("Note: Webhook functionality not yet implemented");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
        Ok(())
    }
}

// TODO: Implement webhook request handling when hyper is properly configured

fn should_trigger_on_event(event: &Event) -> bool {
    match &event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => true,
        _ => false,
    }
}

pub struct ManualTrigger {
    scheduler: Arc<CronScheduler>,
}

impl ManualTrigger {
    pub fn new(scheduler: Arc<CronScheduler>) -> Self {
        Self { scheduler }
    }

    pub async fn trigger_job(&self, job_id: Uuid) -> Result<bool> {
        self.scheduler.trigger_job(job_id).await
    }

    pub async fn trigger_job_by_name(&self, job_name: &str) -> Result<bool> {
        let jobs = self.scheduler.list_jobs().await;

        for job in jobs {
            if job.name == job_name {
                return self.scheduler.trigger_job(job.id).await;
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_trigger_creation() {
        let scheduler = Arc::new(CronScheduler::new());
        let trigger = FileTrigger::new(scheduler);

        assert!(trigger.watcher.is_none());
        assert!(trigger.watched_paths.is_empty());
    }

    #[tokio::test]
    async fn test_webhook_trigger_creation() {
        let scheduler = Arc::new(CronScheduler::new());
        let trigger = WebhookTrigger::new(scheduler, 8080);

        assert!(trigger.server_handle.is_none());
        assert_eq!(trigger.port, 8080);
    }

    #[tokio::test]
    async fn test_manual_trigger() {
        let scheduler = Arc::new(CronScheduler::new());
        let trigger = ManualTrigger::new(scheduler.clone());

        // Add a test job
        let job = crate::scheduler::ScheduledJob::new(
            "test_job".to_string(),
            "test.yaml".to_string(),
            crate::scheduler::Schedule::Manual,
        );
        let job_id = scheduler.add_job(job).await.unwrap();

        // Test triggering by ID
        let result = trigger.trigger_job(job_id).await.unwrap();
        assert!(result);

        // Test triggering by name
        let result = trigger.trigger_job_by_name("test_job").await.unwrap();
        assert!(result);

        // Test triggering non-existent job
        let result = trigger.trigger_job_by_name("non_existent").await.unwrap();
        assert!(!result);
    }
}
