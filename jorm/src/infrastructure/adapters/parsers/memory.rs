/// In-memory DAG parser adapter
/// 
/// Stores DAGs in memory for testing purposes. Useful for unit tests
/// that don't need file system access.

use crate::infrastructure::ports::DagParser;
use crate::parser::Dag;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};

/// In-memory implementation of DagParser
/// 
/// Stores DAGs in memory, useful for testing without file system access.
pub struct InMemoryDagParser {
    /// Stored DAGs indexed by path
    dags: Arc<RwLock<HashMap<String, Dag>>>,
}

impl InMemoryDagParser {
    /// Creates a new in-memory DAG parser
    /// 
    /// # Returns
    /// * `InMemoryDagParser` - New parser instance
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jorm::infrastructure::InMemoryDagParser;
    /// 
    /// let parser = InMemoryDagParser::new();
    /// ```
    pub fn new() -> Self {
        Self {
            dags: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Stores a DAG for later retrieval
    /// 
    /// # Arguments
    /// * `path` - Path key to store the DAG under
    /// * `dag` - The DAG to store
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jorm::infrastructure::InMemoryDagParser;
    /// use jorm::parser::Dag;
    /// 
    /// let parser = InMemoryDagParser::new();
    /// let dag = Dag::new("test_dag".to_string());
    /// parser.store("test.yaml", dag);
    /// ```
    pub fn store(&self, path: &str, dag: Dag) {
        let mut dags = self.dags.write().unwrap();
        dags.insert(path.to_string(), dag);
    }

    /// Clears all stored DAGs
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jorm::infrastructure::InMemoryDagParser;
    /// 
    /// let parser = InMemoryDagParser::new();
    /// parser.clear();
    /// ```
    pub fn clear(&self) {
        let mut dags = self.dags.write().unwrap();
        dags.clear();
    }
}

impl Default for InMemoryDagParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DagParser for InMemoryDagParser {
    async fn parse_from_file(&self, path: &Path) -> anyhow::Result<Dag> {
        debug!(path = ?path, "Parsing DAG from memory");
        
        let path_str = path.to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: {:?}", path))?;
        
        let dags = self.dags.read().unwrap();
        let dag = dags.get(path_str)
            .ok_or_else(|| anyhow::anyhow!("DAG not found in memory: {}", path_str))?
            .clone();
        
        info!(
            dag_name = %dag.name,
            task_count = dag.tasks.len(),
            "DAG retrieved from memory"
        );
        
        Ok(dag)
    }

    async fn parse_from_string(
        &self,
        content: &str,
        format: &str,
    ) -> anyhow::Result<Dag> {
        debug!(format = format, "Parsing DAG from string in memory");
        
        // Use the file system parser for actual parsing
        use crate::parser::parsers;
        
        let dag = match format.to_lowercase().as_str() {
            "yaml" | "yml" => parsers::parse_yaml(content)?,
            "json" => parsers::parse_yaml(content)?,
            "txt" => parsers::parse_txt(content)?,
            "md" | "markdown" => parsers::parse_md(content)?,
            _ => anyhow::bail!("Unsupported format: {}", format),
        };
        
        info!(
            dag_name = %dag.name,
            task_count = dag.tasks.len(),
            "DAG parsed from string in memory"
        );
        
        Ok(dag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Test storing and retrieving a DAG
    #[tokio::test]
    async fn test_store_and_retrieve() {
        let parser = InMemoryDagParser::new();
        let dag = Dag::new("test_dag".to_string());
        
        parser.store("test.yaml", dag.clone());
        
        let path = PathBuf::from("test.yaml");
        let result = parser.parse_from_file(&path).await;
        
        assert!(result.is_ok());
        let retrieved_dag = result.unwrap();
        assert_eq!(retrieved_dag.name, "test_dag");
    }

    /// Test retrieving non-existent DAG
    #[tokio::test]
    async fn test_retrieve_nonexistent() {
        let parser = InMemoryDagParser::new();
        let path = PathBuf::from("nonexistent.yaml");
        
        let result = parser.parse_from_file(&path).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    /// Test parsing from string
    #[tokio::test]
    async fn test_parse_from_string() {
        let parser = InMemoryDagParser::new();
        let content = "dag: test_dag\ntasks:\n  - task1";
        
        let result = parser.parse_from_string(content, "yaml").await;
        assert!(result.is_ok());
    }

    /// Test clearing stored DAGs
    #[tokio::test]
    async fn test_clear() {
        let parser = InMemoryDagParser::new();
        let dag = Dag::new("test_dag".to_string());
        
        parser.store("test.yaml", dag);
        parser.clear();
        
        let path = PathBuf::from("test.yaml");
        let result = parser.parse_from_file(&path).await;
        assert!(result.is_err());
    }

    /// Test default implementation
    #[test]
    fn test_default() {
        let _parser = InMemoryDagParser::default();
    }
}
