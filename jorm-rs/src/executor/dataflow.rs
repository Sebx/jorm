use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    pub task_id: String,
    pub execution_id: Uuid,
    pub output_type: OutputType,
    pub data: OutputData,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputType {
    Text,
    Json,
    File,
    Binary,
    Structured(String), // Schema name
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputData {
    Text(String),
    Json(serde_json::Value),
    File(PathBuf),
    Binary(Vec<u8>),
    Reference(String), // Reference to external storage
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowConfig {
    pub max_output_size_mb: usize,
    pub storage_backend: StorageBackend,
    pub cleanup_after_hours: u32,
    pub validation_schemas: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    Memory,
    File { base_path: PathBuf },
    Database { connection_string: String },
    S3 { bucket: String, region: String },
}

pub struct DataFlowManager {
    config: DataFlowConfig,
    outputs: HashMap<String, TaskOutput>,
    validators: HashMap<String, Box<dyn OutputValidator>>,
}

pub trait OutputValidator: Send + Sync {
    fn validate(&self, data: &OutputData) -> Result<()>;
    fn get_schema(&self) -> &serde_json::Value;
}

pub struct JsonSchemaValidator {
    schema: serde_json::Value,
}

impl JsonSchemaValidator {
    pub fn new(schema: serde_json::Value) -> Self {
        Self { schema }
    }
}

impl OutputValidator for JsonSchemaValidator {
    fn validate(&self, data: &OutputData) -> Result<()> {
        match data {
            OutputData::Json(value) => {
                // In a full implementation, this would use a JSON schema validation library
                // For now, just check if it's valid JSON
                if value.is_object() || value.is_array() {
                    Ok(())
                } else {
                    anyhow::bail!("Invalid JSON structure")
                }
            }
            OutputData::Text(text) => {
                // Try to parse as JSON
                serde_json::from_str::<serde_json::Value>(text)
                    .context("Text is not valid JSON")?;
                Ok(())
            }
            _ => anyhow::bail!("JSON validator can only validate JSON or text data"),
        }
    }

    fn get_schema(&self) -> &serde_json::Value {
        &self.schema
    }
}

impl Default for DataFlowConfig {
    fn default() -> Self {
        Self {
            max_output_size_mb: 100,
            storage_backend: StorageBackend::Memory,
            cleanup_after_hours: 24,
            validation_schemas: HashMap::new(),
        }
    }
}

impl DataFlowManager {
    pub fn new(config: DataFlowConfig) -> Self {
        Self {
            config,
            outputs: HashMap::new(),
            validators: HashMap::new(),
        }
    }

    pub fn add_validator(&mut self, name: String, validator: Box<dyn OutputValidator>) {
        self.validators.insert(name, validator);
    }

    pub async fn store_output(
        &mut self,
        task_id: String,
        data: OutputData,
        output_type: OutputType,
    ) -> Result<TaskOutput> {
        let execution_id = Uuid::new_v4();

        // Validate data size
        let data_size = self.calculate_data_size(&data)?;
        if data_size > self.config.max_output_size_mb * 1024 * 1024 {
            anyhow::bail!(
                "Output data size ({} MB) exceeds maximum allowed size ({} MB)",
                data_size / (1024 * 1024),
                self.config.max_output_size_mb
            );
        }

        // Validate data if validator exists
        if let OutputType::Structured(schema_name) = &output_type {
            if let Some(validator) = self.validators.get(schema_name) {
                validator
                    .validate(&data)
                    .context("Output validation failed")?;
            }
        }

        // Store data based on backend
        let stored_data = match &self.config.storage_backend {
            StorageBackend::Memory => data,
            StorageBackend::File { base_path } => {
                self.store_to_file(base_path, &execution_id, data).await?
            }
            StorageBackend::Database { .. } => self.store_to_database(&execution_id, data).await?,
            StorageBackend::S3 { .. } => self.store_to_s3(&execution_id, data).await?,
        };

        let output = TaskOutput {
            task_id: task_id.clone(),
            execution_id,
            output_type,
            data: stored_data,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
        };

        self.outputs.insert(task_id, output.clone());
        Ok(output)
    }

    pub fn get_output(&self, task_id: &str) -> Option<&TaskOutput> {
        self.outputs.get(task_id)
    }

    pub async fn get_output_data(&self, task_id: &str) -> Result<Option<OutputData>> {
        if let Some(output) = self.outputs.get(task_id) {
            match &self.config.storage_backend {
                StorageBackend::Memory => Ok(Some(output.data.clone())),
                StorageBackend::File { base_path } => {
                    self.load_from_file(base_path, &output.execution_id).await
                }
                StorageBackend::Database { .. } => {
                    self.load_from_database(&output.execution_id).await
                }
                StorageBackend::S3 { .. } => self.load_from_s3(&output.execution_id).await,
            }
        } else {
            Ok(None)
        }
    }

    pub fn list_outputs(&self) -> Vec<&TaskOutput> {
        self.outputs.values().collect()
    }

    pub async fn cleanup_old_outputs(&mut self) -> Result<usize> {
        let cutoff_time =
            chrono::Utc::now() - chrono::Duration::hours(self.config.cleanup_after_hours as i64);
        let mut removed_count = 0;

        let old_outputs: Vec<String> = self
            .outputs
            .iter()
            .filter(|(_, output)| output.created_at < cutoff_time)
            .map(|(task_id, _)| task_id.clone())
            .collect();

        for task_id in old_outputs {
            if let Some(output) = self.outputs.remove(&task_id) {
                // Clean up storage if needed
                match &self.config.storage_backend {
                    StorageBackend::File { base_path } => {
                        self.delete_from_file(base_path, &output.execution_id)
                            .await?;
                    }
                    StorageBackend::Database { .. } => {
                        self.delete_from_database(&output.execution_id).await?;
                    }
                    StorageBackend::S3 { .. } => {
                        self.delete_from_s3(&output.execution_id).await?;
                    }
                    StorageBackend::Memory => {
                        // No additional cleanup needed for memory storage
                    }
                }
                removed_count += 1;
            }
        }

        Ok(removed_count)
    }

    pub fn substitute_output_references(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Replace ${task_name.output} patterns
        for (task_id, output) in &self.outputs {
            let pattern = format!("${{{}.output}}", task_id);
            if let Ok(output_str) = self.output_to_string(&output.data) {
                result = result.replace(&pattern, &output_str);
            }
        }

        result
    }

    fn calculate_data_size(&self, data: &OutputData) -> Result<usize> {
        match data {
            OutputData::Text(text) => Ok(text.len()),
            OutputData::Json(value) => {
                let serialized = serde_json::to_string(value)?;
                Ok(serialized.len())
            }
            OutputData::Binary(bytes) => Ok(bytes.len()),
            OutputData::File(path) => std::fs::metadata(path)
                .map(|m| m.len() as usize)
                .context("Failed to get file size"),
            OutputData::Reference(_) => Ok(0), // References don't count toward size limit
        }
    }

    fn output_to_string(&self, data: &OutputData) -> Result<String> {
        match data {
            OutputData::Text(text) => Ok(text.clone()),
            OutputData::Json(value) => {
                serde_json::to_string(value).context("Failed to serialize JSON")
            }
            OutputData::File(path) => Ok(path.to_string_lossy().to_string()),
            OutputData::Binary(_) => Ok("[Binary Data]".to_string()),
            OutputData::Reference(ref_str) => Ok(ref_str.clone()),
        }
    }

    async fn store_to_file(
        &self,
        base_path: &PathBuf,
        execution_id: &Uuid,
        data: OutputData,
    ) -> Result<OutputData> {
        let file_path = base_path.join(format!("{}.json", execution_id));

        // Ensure directory exists
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create storage directory")?;
        }

        let serialized = serde_json::to_string(&data).context("Failed to serialize output data")?;

        tokio::fs::write(&file_path, serialized)
            .await
            .context("Failed to write output to file")?;

        Ok(OutputData::Reference(
            file_path.to_string_lossy().to_string(),
        ))
    }

    async fn load_from_file(
        &self,
        base_path: &PathBuf,
        execution_id: &Uuid,
    ) -> Result<Option<OutputData>> {
        let file_path = base_path.join(format!("{}.json", execution_id));

        if !file_path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&file_path)
            .await
            .context("Failed to read output file")?;

        let data: OutputData =
            serde_json::from_str(&content).context("Failed to deserialize output data")?;

        Ok(Some(data))
    }

    async fn delete_from_file(&self, base_path: &PathBuf, execution_id: &Uuid) -> Result<()> {
        let file_path = base_path.join(format!("{}.json", execution_id));

        if file_path.exists() {
            tokio::fs::remove_file(&file_path)
                .await
                .context("Failed to delete output file")?;
        }

        Ok(())
    }

    async fn store_to_database(
        &self,
        _execution_id: &Uuid,
        data: OutputData,
    ) -> Result<OutputData> {
        // TODO: Implement database storage
        Ok(data)
    }

    async fn load_from_database(&self, _execution_id: &Uuid) -> Result<Option<OutputData>> {
        // TODO: Implement database loading
        Ok(None)
    }

    async fn delete_from_database(&self, _execution_id: &Uuid) -> Result<()> {
        // TODO: Implement database deletion
        Ok(())
    }

    async fn store_to_s3(&self, _execution_id: &Uuid, data: OutputData) -> Result<OutputData> {
        // TODO: Implement S3 storage
        Ok(data)
    }

    async fn load_from_s3(&self, _execution_id: &Uuid) -> Result<Option<OutputData>> {
        // TODO: Implement S3 loading
        Ok(None)
    }

    async fn delete_from_s3(&self, _execution_id: &Uuid) -> Result<()> {
        // TODO: Implement S3 deletion
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_dataflow_manager_creation() {
        let config = DataFlowConfig::default();
        let manager = DataFlowManager::new(config);

        assert_eq!(manager.outputs.len(), 0);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_output() {
        let config = DataFlowConfig::default();
        let mut manager = DataFlowManager::new(config);

        let data = OutputData::Text("Hello, World!".to_string());
        let output = manager
            .store_output("test_task".to_string(), data, OutputType::Text)
            .await
            .unwrap();

        assert_eq!(output.task_id, "test_task");

        let retrieved = manager.get_output("test_task").unwrap();
        assert_eq!(retrieved.task_id, "test_task");
    }

    #[tokio::test]
    async fn test_file_storage_backend() {
        let temp_dir = TempDir::new().unwrap();
        let config = DataFlowConfig {
            storage_backend: StorageBackend::File {
                base_path: temp_dir.path().to_path_buf(),
            },
            ..Default::default()
        };
        let mut manager = DataFlowManager::new(config);

        let data = OutputData::Json(serde_json::json!({"key": "value"}));
        let output = manager
            .store_output("test_task".to_string(), data, OutputType::Json)
            .await
            .unwrap();

        // Data should be stored as reference
        match &output.data {
            OutputData::Reference(_) => {
                // Good, data was stored to file
            }
            _ => panic!("Expected data to be stored as reference"),
        }

        // Should be able to retrieve the data
        let retrieved_data = manager.get_output_data("test_task").await.unwrap().unwrap();
        match retrieved_data {
            OutputData::Json(value) => {
                assert_eq!(value["key"], "value");
            }
            _ => panic!("Expected JSON data"),
        }
    }

    #[tokio::test]
    async fn test_output_size_limit() {
        let config = DataFlowConfig {
            max_output_size_mb: 1, // 1 MB limit
            ..Default::default()
        };
        let mut manager = DataFlowManager::new(config);

        // Create data larger than 1 MB
        let large_data = "x".repeat(2 * 1024 * 1024); // 2 MB
        let data = OutputData::Text(large_data);

        let result = manager
            .store_output("test_task".to_string(), data, OutputType::Text)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_output_reference_substitution() {
        let config = DataFlowConfig::default();
        let mut manager = DataFlowManager::new(config);

        let data = OutputData::Text("Hello, World!".to_string());
        manager
            .store_output("greeting".to_string(), data, OutputType::Text)
            .await
            .unwrap();

        let template = "The greeting is: ${greeting.output}";
        let result = manager.substitute_output_references(template);

        assert_eq!(result, "The greeting is: Hello, World!");
    }

    #[tokio::test]
    async fn test_cleanup_old_outputs() {
        let config = DataFlowConfig {
            cleanup_after_hours: 0, // Immediate cleanup
            ..Default::default()
        };
        let mut manager = DataFlowManager::new(config);

        let data = OutputData::Text("Test".to_string());
        manager
            .store_output("test_task".to_string(), data, OutputType::Text)
            .await
            .unwrap();

        assert_eq!(manager.outputs.len(), 1);

        // Wait a bit to ensure the timestamp is in the past
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let removed_count = manager.cleanup_old_outputs().await.unwrap();
        assert_eq!(removed_count, 1);
        assert_eq!(manager.outputs.len(), 0);
    }
}
