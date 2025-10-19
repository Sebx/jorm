/// File system DAG parser adapter
/// 
/// Parses DAGs from the file system using the existing parser infrastructure.

use crate::infrastructure::ports::DagParser;
use crate::parser::{parse_dag_file, Dag};
use async_trait::async_trait;
use std::path::Path;
use tracing::{debug, info, instrument};

/// File system implementation of DagParser
/// 
/// Reads DAG files from the file system and parses them using
/// the existing parser infrastructure.
pub struct FileSystemDagParser;

impl FileSystemDagParser {
    /// Creates a new file system DAG parser
    /// 
    /// # Returns
    /// * `FileSystemDagParser` - New parser instance
    /// 
    /// # Examples
    /// 
    /// ```
    /// use jorm::infrastructure::FileSystemDagParser;
    /// 
    /// let parser = FileSystemDagParser::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileSystemDagParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DagParser for FileSystemDagParser {
    #[instrument(skip(self))]
    async fn parse_from_file(&self, path: &Path) -> anyhow::Result<Dag> {
        debug!(path = ?path, "Parsing DAG from file");
        
        let path_str = path.to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: {:?}", path))?;
        
        let dag = parse_dag_file(path_str).await?;
        
        info!(
            dag_name = %dag.name,
            task_count = dag.tasks.len(),
            "DAG parsed successfully from file"
        );
        
        Ok(dag)
    }

    #[instrument(skip(self, content))]
    async fn parse_from_string(
        &self,
        content: &str,
        format: &str,
    ) -> anyhow::Result<Dag> {
        debug!(format = format, content_length = content.len(), "Parsing DAG from string");
        
        use crate::parser::parsers;
        
        let dag = match format.to_lowercase().as_str() {
            "yaml" | "yml" => parsers::parse_yaml(content)?,
            "json" => parsers::parse_yaml(content)?, // JSON is valid YAML
            "txt" => parsers::parse_txt(content)?,
            "md" | "markdown" => parsers::parse_md(content)?,
            _ => anyhow::bail!("Unsupported format: {}", format),
        };
        
        info!(
            dag_name = %dag.name,
            task_count = dag.tasks.len(),
            format = format,
            "DAG parsed successfully from string"
        );
        
        Ok(dag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Test parsing a DAG from a file
    #[tokio::test]
    async fn test_parse_from_file() {
        // Create a temporary DAG file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "dag: test_dag").unwrap();
        writeln!(temp_file, "tasks:").unwrap();
        writeln!(temp_file, "  - task1").unwrap();
        temp_file.flush().unwrap();

        let parser = FileSystemDagParser::new();
        let result = parser.parse_from_file(temp_file.path()).await;
        
        assert!(result.is_ok());
        let dag = result.unwrap();
        assert_eq!(dag.name, "test_dag");
    }

    /// Test parsing a DAG from a YAML string
    #[tokio::test]
    async fn test_parse_from_string_yaml() {
        let content = r#"
dag: test_dag
tasks:
  - task1
"#;

        let parser = FileSystemDagParser::new();
        let result = parser.parse_from_string(content, "yaml").await;
        
        assert!(result.is_ok());
        let dag = result.unwrap();
        assert_eq!(dag.name, "test_dag");
    }

    /// Test parsing a DAG from a text string
    #[tokio::test]
    async fn test_parse_from_string_txt() {
        let content = "dag: test_dag\ntasks:\n  - task1";

        let parser = FileSystemDagParser::new();
        let result = parser.parse_from_string(content, "txt").await;
        
        assert!(result.is_ok());
    }

    /// Test parsing with unsupported format
    #[tokio::test]
    async fn test_parse_unsupported_format() {
        let content = "some content";

        let parser = FileSystemDagParser::new();
        let result = parser.parse_from_string(content, "xml").await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported format"));
    }

    /// Test default implementation
    #[test]
    fn test_default() {
        let _parser = FileSystemDagParser::default();
    }
}
