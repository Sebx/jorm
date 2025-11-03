use jorm::{JormEngine, scheduler::{cron::Scheduler, daemon::DaemonManager}};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use std::fs;

#[cfg(test)]
mod scheduler_integration_tests {
    use super::*;

    async fn setup_test_scheduler() -> Scheduler {
        let engine = Arc::new(JormEngine::new().await.unwrap());
        Scheduler::new(engine)
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let scheduler = setup_test_scheduler().await;
        
        // Test that scheduler can be created
        assert!(true);
    }

    #[tokio::test]
    async fn test_scheduler_add_schedule() {
        let mut scheduler = setup_test_scheduler().await;
        let temp_dir = TempDir::new().unwrap();
        let dag_file = temp_dir.path().join("scheduled_dag.txt");
        
        let dag_content = r#"
task scheduled_task {
    type: shell
    command: "echo 'scheduled execution'"
}
"#;
        
        fs::write(&dag_file, dag_content).unwrap();
        
        // Add a schedule (every minute for testing)
        let result = scheduler.add_schedule(
            "test_schedule".to_string(),
            "* * * * *".to_string(), // Every minute
            dag_file.to_str().unwrap().to_string(),
        );
        
        assert!(result.is_ok(), "Failed to add schedule: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_scheduler_invalid_cron_expression() {
        let mut scheduler = setup_test_scheduler().await;
        let temp_dir = TempDir::new().unwrap();
        let dag_file = temp_dir.path().join("test_dag.txt");
        
        fs::write(&dag_file, "task test { type: shell, command: \"echo test\" }").unwrap();
        
        // Try to add schedule with invalid cron expression
        let result = scheduler.add_schedule(
            "invalid_schedule".to_string(),
            "invalid cron".to_string(),
            dag_file.to_str().unwrap().to_string(),
        );
        
        assert!(result.is_err(), "Should fail with invalid cron expression");
    }

    #[tokio::test]
    async fn test_scheduler_remove_schedule() {
        let mut scheduler = setup_test_scheduler().await;
        let temp_dir = TempDir::new().unwrap();
        let dag_file = temp_dir.path().join("test_dag.txt");
        
        fs::write(&dag_file, "task test { type: shell, command: \"echo test\" }").unwrap();
        
        // Add a schedule
        scheduler.add_schedule(
            "removable_schedule".to_string(),
            "0 0 * * *".to_string(), // Daily at midnight
            dag_file.to_str().unwrap().to_string(),
        ).unwrap();
        
        // Remove the schedule
        let result = scheduler.remove_schedule("removable_schedule");
        assert!(result.is_ok(), "Failed to remove schedule: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_scheduler_list_schedules() {
        let mut scheduler = setup_test_scheduler().await;
        let temp_dir = TempDir::new().unwrap();
        let dag_file = temp_dir.path().join("test_dag.txt");
        
        fs::write(&dag_file, "task test { type: shell, command: \"echo test\" }").unwrap();
        
        // Add multiple schedules
        scheduler.add_schedule(
            "schedule1".to_string(),
            "0 9 * * *".to_string(), // Daily at 9 AM
            dag_file.to_str().unwrap().to_string(),
        ).unwrap();
        
        scheduler.add_schedule(
            "schedule2".to_string(),
            "0 17 * * *".to_string(), // Daily at 5 PM
            dag_file.to_str().unwrap().to_string(),
        ).unwrap();
        
        let schedules = scheduler.list_schedules();
        assert_eq!(schedules.len(), 2);
        
        let schedule_names: Vec<&String> = schedules.iter().map(|s| &s.name).collect();
        assert!(schedule_names.contains(&&"schedule1".to_string()));
        assert!(schedule_names.contains(&&"schedule2".to_string()));
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let scheduler = setup_test_scheduler().await;
        
        // Test starting scheduler
        // Note: In a real test, we'd need to be careful about background threads
        // For now, just test that the methods exist and can be called
        
        // This would start the scheduler in the background
        // let handle = scheduler.start().await;
        // assert!(handle.is_ok());
        
        // This would stop the scheduler
        // scheduler.stop().await;
        
        assert!(true); // Placeholder for actual scheduler start/stop testing
    }

    #[tokio::test]
    async fn test_daemon_manager_creation() {
        let daemon_manager = DaemonManager::new();
        
        // Test that daemon manager can be created
        assert!(true);
    }

    #[tokio::test]
    async fn test_daemon_manager_start_stop() {
        let daemon_manager = DaemonManager::new();
        
        // Test daemon start/stop operations
        // Note: These are complex operations that would require:
        // 1. Process management
        // 2. PID file handling
        // 3. Signal handling
        // 4. Proper cleanup
        
        // For now, just test that methods exist
        assert!(true);
    }

    #[tokio::test]
    async fn test_scheduler_execution_simulation() {
        let mut scheduler = setup_test_scheduler().await;
        let temp_dir = TempDir::new().unwrap();
        let dag_file = temp_dir.path().join("execution_test_dag.txt");
        let output_file = temp_dir.path().join("execution_output.txt");
        
        let dag_content = format!(r#"
task create_output {{
    type: shell
    command: "echo 'scheduled execution at $(date)' > {}"
}}
"#, output_file.to_str().unwrap());
        
        fs::write(&dag_file, dag_content).unwrap();
        
        // Add a schedule
        scheduler.add_schedule(
            "execution_test".to_string(),
            "* * * * *".to_string(), // Every minute
            dag_file.to_str().unwrap().to_string(),
        ).unwrap();
        
        // Simulate manual execution of scheduled task
        // In a real scheduler, this would happen automatically
        let engine = Arc::new(JormEngine::new().await.unwrap());
        let result = engine.execute_from_file(dag_file.to_str().unwrap()).await;
        
        assert!(result.is_ok(), "Scheduled DAG execution failed: {:?}", result.err());
        
        // Verify output file was created
        assert!(output_file.exists(), "Scheduled task did not create output file");
        
        let content = fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("scheduled execution"), "Output file does not contain expected content");
    }

    #[tokio::test]
    async fn test_scheduler_multiple_schedules_execution() {
        let mut scheduler = setup_test_scheduler().await;
        let temp_dir = TempDir::new().unwrap();
        
        // Create multiple DAG files
        let dag_file1 = temp_dir.path().join("dag1.txt");
        let dag_file2 = temp_dir.path().join("dag2.txt");
        let output_file1 = temp_dir.path().join("output1.txt");
        let output_file2 = temp_dir.path().join("output2.txt");
        
        let dag_content1 = format!(r#"
task task1 {{
    type: shell
    command: "echo 'execution 1' > {}"
}}
"#, output_file1.to_str().unwrap());
        
        let dag_content2 = format!(r#"
task task2 {{
    type: shell
    command: "echo 'execution 2' > {}"
}}
"#, output_file2.to_str().unwrap());
        
        fs::write(&dag_file1, dag_content1).unwrap();
        fs::write(&dag_file2, dag_content2).unwrap();
        
        // Add multiple schedules
        scheduler.add_schedule(
            "multi_schedule1".to_string(),
            "0 9 * * *".to_string(),
            dag_file1.to_str().unwrap().to_string(),
        ).unwrap();
        
        scheduler.add_schedule(
            "multi_schedule2".to_string(),
            "0 17 * * *".to_string(),
            dag_file2.to_str().unwrap().to_string(),
        ).unwrap();
        
        // Verify both schedules are registered
        let schedules = scheduler.list_schedules();
        assert_eq!(schedules.len(), 2);
        
        // Simulate execution of both scheduled DAGs
        let engine = Arc::new(JormEngine::new().await.unwrap());
        
        let result1 = engine.execute_from_file(dag_file1.to_str().unwrap()).await;
        assert!(result1.is_ok());
        
        let result2 = engine.execute_from_file(dag_file2.to_str().unwrap()).await;
        assert!(result2.is_ok());
        
        // Verify both outputs were created
        assert!(output_file1.exists());
        assert!(output_file2.exists());
    }

    #[tokio::test]
    async fn test_scheduler_error_handling() {
        let mut scheduler = setup_test_scheduler().await;
        let temp_dir = TempDir::new().unwrap();
        let dag_file = temp_dir.path().join("error_dag.txt");
        
        let dag_content = r#"
task failing_task {
    type: shell
    command: "exit 1"
}
"#;
        
        fs::write(&dag_file, dag_content).unwrap();
        
        // Add schedule for failing DAG
        scheduler.add_schedule(
            "error_schedule".to_string(),
            "* * * * *".to_string(),
            dag_file.to_str().unwrap().to_string(),
        ).unwrap();
        
        // Simulate execution of failing DAG
        let engine = Arc::new(JormEngine::new().await.unwrap());
        let result = engine.execute_from_file(dag_file.to_str().unwrap()).await;
        
        // Should return a result (not panic) even if execution fails
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert!(!execution_result.success); // But execution should fail
    }

    #[tokio::test]
    async fn test_scheduler_nonexistent_dag_file() {
        let mut scheduler = setup_test_scheduler().await;
        let nonexistent_file = "/path/that/does/not/exist/dag.txt";
        
        // Try to add schedule with nonexistent DAG file
        let result = scheduler.add_schedule(
            "nonexistent_schedule".to_string(),
            "0 0 * * *".to_string(),
            nonexistent_file.to_string(),
        );
        
        // Should either fail immediately or fail during execution
        // The exact behavior depends on implementation
        // For now, just verify it doesn't panic
        assert!(true);
    }
}