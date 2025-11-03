use crate::core::error::JormError;
use crate::parser::dag_parser::DagParser;
use crate::executor::TaskExecutor;
use crate::nlp::generator::{NlpProcessor, DagPreview, DagEdit};

#[derive(Debug, serde::Serialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub message: String,
    pub task_results: Vec<TaskResult>,
}

#[derive(Debug, serde::Serialize)]
pub struct TaskResult {
    pub task_name: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

pub struct JormEngine {
    nlp_processor: NlpProcessor,
    dag_parser: DagParser,
    task_executor: TaskExecutor,
}

impl JormEngine {
    pub async fn new() -> Result<Self, JormError> {
        Ok(Self {
            nlp_processor: NlpProcessor::new().await?,
            dag_parser: DagParser::new(),
            task_executor: TaskExecutor::new(),
        })
    }

    pub async fn execute_from_file(&self, path: &str) -> Result<ExecutionResult, JormError> {
        // Parse the DAG file
        let dag = self.dag_parser.parse_file(path).await?;
        
        // Validate the DAG
        dag.validate()?;
        
        // Execute the DAG
        self.task_executor.execute_dag(&dag).await
    }

    pub async fn execute_from_natural_language(&self, description: &str) -> Result<ExecutionResult, JormError> {
        // Generate DAG from natural language
        let dag_content = self.nlp_processor.generate_dag(description).await?;
        
        // Parse the generated DAG
        let dag = self.dag_parser.parse_content(&dag_content)?;
        
        // Validate the DAG
        dag.validate()?;
        
        // Execute the DAG
        self.task_executor.execute_dag(&dag).await
    }

    pub async fn generate_dag_from_nl(&self, description: &str) -> Result<String, JormError> {
        self.nlp_processor.generate_dag(description).await
    }

    pub async fn generate_dag_with_preview(&self, description: &str) -> Result<DagPreview, JormError> {
        self.nlp_processor.generate_dag_with_preview(description).await
    }

    pub async fn execute_with_preview(&self, description: &str, auto_execute: bool) -> Result<(DagPreview, Option<ExecutionResult>), JormError> {
        let preview = self.generate_dag_with_preview(description).await?;
        
        if auto_execute {
            let execution_result = self.execute_from_dag_content(&preview.generated_dag_content).await?;
            Ok((preview, Some(execution_result)))
        } else {
            Ok((preview, None))
        }
    }

    pub async fn execute_from_dag_content(&self, dag_content: &str) -> Result<ExecutionResult, JormError> {
        // Parse the DAG content
        let dag = self.dag_parser.parse_content(dag_content)?;
        
        // Validate the DAG
        dag.validate()?;
        
        // Execute the DAG
        self.task_executor.execute_dag(&dag).await
    }

    pub fn edit_generated_dag(&self, dag_content: &str, edits: Vec<DagEdit>) -> Result<String, JormError> {
        self.nlp_processor.edit_generated_dag(dag_content, edits)
    }

    pub async fn execute_edited_dag(&self, original_dag: &str, edits: Vec<DagEdit>) -> Result<ExecutionResult, JormError> {
        let edited_dag = self.edit_generated_dag(original_dag, edits)?;
        self.execute_from_dag_content(&edited_dag).await
    }
}