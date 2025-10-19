/// In-memory DAG storage adapter
/// 
/// Implements DagRepository using in-memory storage for testing.

use crate::domain::{Dag, DagId, DagRepository, DomainResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};

/// In-memory implementation of DagRepository
/// 
/// Stores DAGs in memory, useful for testing without database access.
pub struct InMemoryDagStorage {
    /// Stored DAGs indexed by ID
    dags: Arc<RwLock<HashMap<String, Dag>>>,
}

impl InMemoryDagStorage {
    /// Creates a new in-memory DAG storage
    /// 
    /// # Returns
    /// * `InMemoryDagStorage` - New storage instance
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jorm::infrastructure::InMemoryDagStorage;
    /// 
    /// let storage = InMemoryDagStorage::new();
    /// ```
    pub fn new() -> Self {
        Self {
            dags: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Clears all stored DAGs
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jorm::infrastructure::InMemoryDagStorage;
    /// 
    /// let storage = InMemoryDagStorage::new();
    /// storage.clear();
    /// ```
    pub fn clear(&self) {
        let mut dags = self.dags.write().unwrap();
        dags.clear();
    }

    /// Returns the number of stored DAGs
    /// 
    /// # Returns
    /// * `usize` - Number of DAGs in storage
    pub fn count(&self) -> usize {
        let dags = self.dags.read().unwrap();
        dags.len()
    }
}

impl Default for InMemoryDagStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DagRepository for InMemoryDagStorage {
    async fn save(&self, dag: &Dag) -> DomainResult<()> {
        debug!(dag_id = %dag.id(), "Saving DAG to memory");
        
        let mut dags = self.dags.write().unwrap();
        dags.insert(dag.id().to_string(), dag.clone());
        
        info!(dag_id = %dag.id(), "DAG saved to memory");
        Ok(())
    }

    async fn find_by_id(&self, id: &DagId) -> DomainResult<Option<Dag>> {
        debug!(dag_id = %id, "Finding DAG in memory");
        
        let dags = self.dags.read().unwrap();
        let dag = dags.get(&id.to_string()).cloned();
        
        if dag.is_some() {
            info!(dag_id = %id, "DAG found in memory");
        } else {
            debug!(dag_id = %id, "DAG not found in memory");
        }
        
        Ok(dag)
    }

    async fn find_all(&self) -> DomainResult<Vec<Dag>> {
        debug!("Finding all DAGs in memory");
        
        let dags = self.dags.read().unwrap();
        let all_dags: Vec<Dag> = dags.values().cloned().collect();
        
        info!(count = all_dags.len(), "Found DAGs in memory");
        Ok(all_dags)
    }

    async fn delete(&self, id: &DagId) -> DomainResult<bool> {
        debug!(dag_id = %id, "Deleting DAG from memory");
        
        let mut dags = self.dags.write().unwrap();
        let removed = dags.remove(&id.to_string()).is_some();
        
        if removed {
            info!(dag_id = %id, "DAG deleted from memory");
        } else {
            debug!(dag_id = %id, "DAG not found for deletion");
        }
        
        Ok(removed)
    }

    async fn exists(&self, id: &DagId) -> DomainResult<bool> {
        debug!(dag_id = %id, "Checking if DAG exists in memory");
        
        let dags = self.dags.read().unwrap();
        let exists = dags.contains_key(&id.to_string());
        
        debug!(dag_id = %id, exists = exists, "DAG existence check");
        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Task, TaskConfig, TaskId};

    /// Helper to create a test DAG
    fn create_test_dag(id: &str) -> Dag {
        let dag_id = DagId::new(id).unwrap();
        let mut dag = Dag::new(dag_id, format!("Test DAG {}", id));
        
        let task_id = TaskId::new("task1").unwrap();
        let config = TaskConfig::Shell {
            command: "echo test".to_string(),
            working_dir: None,
        };
        let task = Task::new(task_id, "Task 1".to_string(), "shell".to_string(), config);
        dag.add_task(task).unwrap();
        
        dag
    }

    /// Test saving and retrieving a DAG
    #[tokio::test]
    async fn test_save_and_find() {
        let storage = InMemoryDagStorage::new();
        let dag = create_test_dag("test_dag");
        let dag_id = dag.id().clone();
        
        storage.save(&dag).await.unwrap();
        
        let found = storage.find_by_id(&dag_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id(), &dag_id);
    }

    /// Test finding non-existent DAG
    #[tokio::test]
    async fn test_find_nonexistent() {
        let storage = InMemoryDagStorage::new();
        let dag_id = DagId::new("nonexistent").unwrap();
        
        let found = storage.find_by_id(&dag_id).await.unwrap();
        assert!(found.is_none());
    }

    /// Test finding all DAGs
    #[tokio::test]
    async fn test_find_all() {
        let storage = InMemoryDagStorage::new();
        
        storage.save(&create_test_dag("dag1")).await.unwrap();
        storage.save(&create_test_dag("dag2")).await.unwrap();
        storage.save(&create_test_dag("dag3")).await.unwrap();
        
        let all = storage.find_all().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    /// Test deleting a DAG
    #[tokio::test]
    async fn test_delete() {
        let storage = InMemoryDagStorage::new();
        let dag = create_test_dag("test_dag");
        let dag_id = dag.id().clone();
        
        storage.save(&dag).await.unwrap();
        
        let deleted = storage.delete(&dag_id).await.unwrap();
        assert!(deleted);
        
        let found = storage.find_by_id(&dag_id).await.unwrap();
        assert!(found.is_none());
    }

    /// Test deleting non-existent DAG
    #[tokio::test]
    async fn test_delete_nonexistent() {
        let storage = InMemoryDagStorage::new();
        let dag_id = DagId::new("nonexistent").unwrap();
        
        let deleted = storage.delete(&dag_id).await.unwrap();
        assert!(!deleted);
    }

    /// Test checking existence
    #[tokio::test]
    async fn test_exists() {
        let storage = InMemoryDagStorage::new();
        let dag = create_test_dag("test_dag");
        let dag_id = dag.id().clone();
        
        let exists_before = storage.exists(&dag_id).await.unwrap();
        assert!(!exists_before);
        
        storage.save(&dag).await.unwrap();
        
        let exists_after = storage.exists(&dag_id).await.unwrap();
        assert!(exists_after);
    }

    /// Test clearing storage
    #[tokio::test]
    async fn test_clear() {
        let storage = InMemoryDagStorage::new();
        
        storage.save(&create_test_dag("dag1")).await.unwrap();
        storage.save(&create_test_dag("dag2")).await.unwrap();
        
        assert_eq!(storage.count(), 2);
        
        storage.clear();
        
        assert_eq!(storage.count(), 0);
    }

    /// Test default implementation
    #[test]
    fn test_default() {
        let _storage = InMemoryDagStorage::default();
    }
}
