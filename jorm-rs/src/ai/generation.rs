//! Natural language to DAG generation

use crate::ai::{GeneratedDAG, GeneratedDependency, GeneratedTask};
use crate::parser::{Dag, Task, TaskConfig};
use anyhow::Result;
use std::collections::HashMap;

/// Natural language generator for creating DAGs from descriptions
pub struct NaturalLanguageGenerator {
    model_provider: std::sync::Arc<dyn crate::ai::LanguageModelProvider>,
}

impl NaturalLanguageGenerator {
    /// Create a new natural language generator
    pub fn new(model_provider: std::sync::Arc<dyn crate::ai::LanguageModelProvider>) -> Self {
        Self { model_provider }
    }

    /// Generate a DAG from natural language description
    pub async fn generate_dag(&self, description: &str) -> Result<Dag> {
        // TODO: Implement actual natural language processing
        // For now, we'll create a simple parser for common patterns

        let generated_dag = self.parse_natural_language_description(description).await?;
        self.convert_generated_dag_to_dag(generated_dag)
    }

    /// Parse natural language description into structured format
    async fn parse_natural_language_description(&self, description: &str) -> Result<GeneratedDAG> {
        // Simple pattern matching for common DAG descriptions
        let description_lower = description.to_lowercase();

        if description_lower.contains("data pipeline") {
            self.generate_data_pipeline_dag(description).await
        } else if description_lower.contains("web scraping") {
            self.generate_web_scraping_dag(description).await
        } else if description_lower.contains("etl") || description_lower.contains("extract") {
            self.generate_etl_dag(description).await
        } else if description_lower.contains("machine learning") || description_lower.contains("ml")
        {
            self.generate_ml_pipeline_dag(description).await
        } else if description_lower.contains("sql")
            || description_lower.contains("database")
            || description_lower.contains("sqlserver")
        {
            self.generate_database_workflow_dag(description).await
        } else {
            // Generic DAG generation
            self.generate_generic_dag(description).await
        }
    }

    /// Generate a data pipeline DAG
    async fn generate_data_pipeline_dag(&self, description: &str) -> Result<GeneratedDAG> {
        let tasks = vec![
            GeneratedTask {
                name: "extract_data".to_string(),
                task_type: "http".to_string(),
                description: "Extract data from API".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("method".to_string(), serde_json::json!("GET"));
                    config.insert(
                        "url".to_string(),
                        serde_json::json!("https://api.example.com/data"),
                    );
                    config
                },
            },
            GeneratedTask {
                name: "validate_data".to_string(),
                task_type: "python".to_string(),
                description: "Validate extracted data".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("data_validation"));
                    config.insert("function".to_string(), serde_json::json!("validate"));
                    config
                },
            },
            GeneratedTask {
                name: "transform_data".to_string(),
                task_type: "python".to_string(),
                description: "Transform data for storage".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert(
                        "module".to_string(),
                        serde_json::json!("data_transformation"),
                    );
                    config.insert("function".to_string(), serde_json::json!("transform"));
                    config
                },
            },
            GeneratedTask {
                name: "load_data".to_string(),
                task_type: "python".to_string(),
                description: "Load data into database".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("data_loading"));
                    config.insert("function".to_string(), serde_json::json!("load"));
                    config
                },
            },
        ];

        let dependencies = vec![
            GeneratedDependency {
                from_task: "validate_data".to_string(),
                to_task: "extract_data".to_string(),
                dependency_type: "depends_on".to_string(),
            },
            GeneratedDependency {
                from_task: "transform_data".to_string(),
                to_task: "validate_data".to_string(),
                dependency_type: "depends_on".to_string(),
            },
            GeneratedDependency {
                from_task: "load_data".to_string(),
                to_task: "transform_data".to_string(),
                dependency_type: "depends_on".to_string(),
            },
        ];

        Ok(GeneratedDAG {
            dag_name: "data_pipeline".to_string(),
            description: description.to_string(),
            tasks,
            dependencies,
            confidence_score: 0.8,
            reasoning: "Generated based on common data pipeline patterns".to_string(),
        })
    }

    /// Generate a web scraping DAG
    async fn generate_web_scraping_dag(&self, description: &str) -> Result<GeneratedDAG> {
        let tasks = vec![
            GeneratedTask {
                name: "fetch_pages".to_string(),
                task_type: "http".to_string(),
                description: "Fetch web pages".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("method".to_string(), serde_json::json!("GET"));
                    config.insert("url".to_string(), serde_json::json!("https://example.com"));
                    config
                },
            },
            GeneratedTask {
                name: "parse_content".to_string(),
                task_type: "python".to_string(),
                description: "Parse HTML content".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("scraping"));
                    config.insert("function".to_string(), serde_json::json!("parse_html"));
                    config
                },
            },
            GeneratedTask {
                name: "extract_data".to_string(),
                task_type: "python".to_string(),
                description: "Extract structured data".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("scraping"));
                    config.insert("function".to_string(), serde_json::json!("extract_data"));
                    config
                },
            },
            GeneratedTask {
                name: "save_results".to_string(),
                task_type: "file".to_string(),
                description: "Save scraped data".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("operation".to_string(), serde_json::json!("create"));
                    config.insert(
                        "destination".to_string(),
                        serde_json::json!("scraped_data.json"),
                    );
                    config
                },
            },
        ];

        let dependencies = vec![
            GeneratedDependency {
                from_task: "parse_content".to_string(),
                to_task: "fetch_pages".to_string(),
                dependency_type: "depends_on".to_string(),
            },
            GeneratedDependency {
                from_task: "extract_data".to_string(),
                to_task: "parse_content".to_string(),
                dependency_type: "depends_on".to_string(),
            },
            GeneratedDependency {
                from_task: "save_results".to_string(),
                to_task: "extract_data".to_string(),
                dependency_type: "depends_on".to_string(),
            },
        ];

        Ok(GeneratedDAG {
            dag_name: "web_scraping".to_string(),
            description: description.to_string(),
            tasks,
            dependencies,
            confidence_score: 0.7,
            reasoning: "Generated based on common web scraping patterns".to_string(),
        })
    }

    /// Generate an ETL DAG
    async fn generate_etl_dag(&self, description: &str) -> Result<GeneratedDAG> {
        let tasks = vec![
            GeneratedTask {
                name: "extract".to_string(),
                task_type: "python".to_string(),
                description: "Extract data from source".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("etl"));
                    config.insert("function".to_string(), serde_json::json!("extract"));
                    config
                },
            },
            GeneratedTask {
                name: "transform".to_string(),
                task_type: "python".to_string(),
                description: "Transform extracted data".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("etl"));
                    config.insert("function".to_string(), serde_json::json!("transform"));
                    config
                },
            },
            GeneratedTask {
                name: "load".to_string(),
                task_type: "python".to_string(),
                description: "Load transformed data".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("etl"));
                    config.insert("function".to_string(), serde_json::json!("load"));
                    config
                },
            },
        ];

        let dependencies = vec![
            GeneratedDependency {
                from_task: "transform".to_string(),
                to_task: "extract".to_string(),
                dependency_type: "depends_on".to_string(),
            },
            GeneratedDependency {
                from_task: "load".to_string(),
                to_task: "transform".to_string(),
                dependency_type: "depends_on".to_string(),
            },
        ];

        Ok(GeneratedDAG {
            dag_name: "etl_pipeline".to_string(),
            description: description.to_string(),
            tasks,
            dependencies,
            confidence_score: 0.9,
            reasoning: "Generated based on standard ETL pattern".to_string(),
        })
    }

    /// Generate a machine learning pipeline DAG
    async fn generate_ml_pipeline_dag(&self, description: &str) -> Result<GeneratedDAG> {
        let tasks = vec![
            GeneratedTask {
                name: "load_data".to_string(),
                task_type: "python".to_string(),
                description: "Load training data".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("ml_pipeline"));
                    config.insert("function".to_string(), serde_json::json!("load_data"));
                    config
                },
            },
            GeneratedTask {
                name: "preprocess".to_string(),
                task_type: "python".to_string(),
                description: "Preprocess data".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("ml_pipeline"));
                    config.insert("function".to_string(), serde_json::json!("preprocess"));
                    config
                },
            },
            GeneratedTask {
                name: "train_model".to_string(),
                task_type: "python".to_string(),
                description: "Train machine learning model".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("ml_pipeline"));
                    config.insert("function".to_string(), serde_json::json!("train_model"));
                    config
                },
            },
            GeneratedTask {
                name: "evaluate_model".to_string(),
                task_type: "python".to_string(),
                description: "Evaluate model performance".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("module".to_string(), serde_json::json!("ml_pipeline"));
                    config.insert("function".to_string(), serde_json::json!("evaluate_model"));
                    config
                },
            },
        ];

        let dependencies = vec![
            GeneratedDependency {
                from_task: "preprocess".to_string(),
                to_task: "load_data".to_string(),
                dependency_type: "depends_on".to_string(),
            },
            GeneratedDependency {
                from_task: "train_model".to_string(),
                to_task: "preprocess".to_string(),
                dependency_type: "depends_on".to_string(),
            },
            GeneratedDependency {
                from_task: "evaluate_model".to_string(),
                to_task: "train_model".to_string(),
                dependency_type: "depends_on".to_string(),
            },
        ];

        Ok(GeneratedDAG {
            dag_name: "ml_pipeline".to_string(),
            description: description.to_string(),
            tasks,
            dependencies,
            confidence_score: 0.8,
            reasoning: "Generated based on common ML pipeline patterns".to_string(),
        })
    }

    /// Generate a generic DAG
    async fn generate_generic_dag(&self, description: &str) -> Result<GeneratedDAG> {
        let tasks = vec![
            GeneratedTask {
                name: "task1".to_string(),
                task_type: "shell".to_string(),
                description: "First task".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("command".to_string(), serde_json::json!("echo 'Task 1'"));
                    config
                },
            },
            GeneratedTask {
                name: "task2".to_string(),
                task_type: "shell".to_string(),
                description: "Second task".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert("command".to_string(), serde_json::json!("echo 'Task 2'"));
                    config
                },
            },
        ];

        let dependencies = vec![GeneratedDependency {
            from_task: "task2".to_string(),
            to_task: "task1".to_string(),
            dependency_type: "depends_on".to_string(),
        }];

        Ok(GeneratedDAG {
            dag_name: "generic_dag".to_string(),
            description: description.to_string(),
            tasks,
            dependencies,
            confidence_score: 0.5,
            reasoning: "Generated generic DAG structure".to_string(),
        })
    }

    /// Generate a database workflow DAG
    async fn generate_database_workflow_dag(&self, description: &str) -> Result<GeneratedDAG> {
        let tasks = vec![
            GeneratedTask {
                name: "extract_from_sqlserver1".to_string(),
                task_type: "python".to_string(),
                description: "Extract data from SQL Server database 1 and save to CSV".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert(
                        "module".to_string(),
                        serde_json::json!("database_operations"),
                    );
                    config.insert("function".to_string(), serde_json::json!("extract_to_csv"));
                    config.insert(
                        "script".to_string(),
                        serde_json::json!(self.generate_sqlserver_extract_script()),
                    );
                    config
                },
            },
            GeneratedTask {
                name: "load_to_sqlserver2".to_string(),
                task_type: "python".to_string(),
                description: "Load CSV data to SQL Server database 2".to_string(),
                configuration: {
                    let mut config = HashMap::new();
                    config.insert(
                        "module".to_string(),
                        serde_json::json!("database_operations"),
                    );
                    config.insert("function".to_string(), serde_json::json!("load_from_csv"));
                    config.insert(
                        "script".to_string(),
                        serde_json::json!(self.generate_sqlserver_load_script()),
                    );
                    config
                },
            },
        ];

        let dependencies = vec![GeneratedDependency {
            from_task: "load_to_sqlserver2".to_string(),
            to_task: "extract_from_sqlserver1".to_string(),
            dependency_type: "depends_on".to_string(),
        }];

        Ok(GeneratedDAG {
            dag_name: "database_workflow".to_string(),
            description: description.to_string(),
            tasks,
            dependencies,
            confidence_score: 0.9,
            reasoning: "Generated database workflow with SQL Server operations".to_string(),
        })
    }

    /// Generate Python script for SQL Server extraction
    fn generate_sqlserver_extract_script(&self) -> String {
        r#"#!/usr/bin/env python3
"""
SQL Server Database Extraction Script
Extracts data from SQL Server database and saves to CSV
"""

import pyodbc
import pandas as pd
import os
from typing import Optional

def get_credentials() -> tuple[str, str, str, str]:
    """Get database credentials from user input"""
    print("🔐 SQL Server Database 1 Credentials:")
    server = input("Server (e.g., localhost, 192.168.1.100): ").strip()
    database = input("Database name: ").strip()
    username = input("Username: ").strip()
    password = input("Password: ").strip()
    return server, database, username, password

def extract_to_csv(server: str, database: str, username: str, password: str, 
                   table_name: str = "table1", output_file: str = "extracted_data.csv") -> bool:
    """Extract data from SQL Server table and save to CSV"""
    try:
        # Connection string
        connection_string = (
            f"DRIVER={{ODBC Driver 17 for SQL Server}};"
            f"SERVER={server};"
            f"DATABASE={database};"
            f"UID={username};"
            f"PWD={password};"
            f"TrustServerCertificate=yes;"
        )
        
        print(f"🔗 Connecting to SQL Server: {server}/{database}")
        with pyodbc.connect(connection_string) as conn:
            print(f"📊 Extracting data from table: {table_name}")
            
            # Execute query
            query = f"SELECT * FROM {table_name}"
            df = pd.read_sql(query, conn)
            
            # Save to CSV
            df.to_csv(output_file, index=False)
            print(f"✅ Data extracted successfully: {output_file}")
            print(f"📈 Records extracted: {len(df)}")
            
            return True
            
    except Exception as e:
        print(f"❌ Error extracting data: {e}")
        return False

def main():
    """Main execution function"""
    print("🚀 Starting SQL Server data extraction...")
    
    # Get credentials
    server, database, username, password = get_credentials()
    
    # Extract data
    success = extract_to_csv(server, database, username, password)
    
    if success:
        print("✅ Extraction completed successfully!")
    else:
        print("❌ Extraction failed!")
        exit(1)

if __name__ == "__main__":
    main()
"#
        .to_string()
    }

    /// Generate Python script for SQL Server loading
    fn generate_sqlserver_load_script(&self) -> String {
        r#"#!/usr/bin/env python3
"""
SQL Server Database Loading Script
Loads data from CSV file to SQL Server database
"""

import pyodbc
import pandas as pd
import os
from typing import Optional

def get_credentials() -> tuple[str, str, str, str]:
    """Get database credentials from user input"""
    print("🔐 SQL Server Database 2 Credentials:")
    server = input("Server (e.g., localhost, 192.168.1.100): ").strip()
    database = input("Database name: ").strip()
    username = input("Username: ").strip()
    password = input("Password: ").strip()
    return server, database, username, password

def load_from_csv(server: str, database: str, username: str, password: str,
                  csv_file: str = "extracted_data.csv", table_name: str = "table2") -> bool:
    """Load data from CSV file to SQL Server table"""
    try:
        # Check if CSV file exists
        if not os.path.exists(csv_file):
            print(f"❌ CSV file not found: {csv_file}")
            return False
        
        # Read CSV file
        print(f"📖 Reading CSV file: {csv_file}")
        df = pd.read_csv(csv_file)
        print(f"📈 Records to load: {len(df)}")
        
        # Connection string
        connection_string = (
            f"DRIVER={{ODBC Driver 17 for SQL Server}};"
            f"SERVER={server};"
            f"DATABASE={database};"
            f"UID={username};"
            f"PWD={password};"
            f"TrustServerCertificate=yes;"
        )
        
        print(f"🔗 Connecting to SQL Server: {server}/{database}")
        with pyodbc.connect(connection_string) as conn:
            print(f"📊 Creating/updating table: {table_name}")
            
            # Create table if it doesn't exist
            create_table_sql = f"""
            IF NOT EXISTS (SELECT * FROM sysobjects WHERE name='{table_name}' AND xtype='U')
            CREATE TABLE {table_name} (
                id INT IDENTITY(1,1) PRIMARY KEY,
                -- Add your columns here based on CSV structure
                data NVARCHAR(MAX)
            )
            """
            
            cursor = conn.cursor()
            cursor.execute(create_table_sql)
            conn.commit()
            
            # Insert data
            print(f"📥 Loading data into {table_name}...")
            for index, row in df.iterrows():
                # Convert row to tuple for insertion
                values = tuple(row.values)
                placeholders = ','.join(['?' for _ in values])
                insert_sql = f"INSERT INTO {table_name} VALUES ({placeholders})"
                cursor.execute(insert_sql, values)
            
            conn.commit()
            print(f"✅ Data loaded successfully into {table_name}")
            print(f"📈 Records loaded: {len(df)}")
            
            return True
            
    except Exception as e:
        print(f"❌ Error loading data: {e}")
        return False

def main():
    """Main execution function"""
    print("🚀 Starting SQL Server data loading...")
    
    # Get credentials
    server, database, username, password = get_credentials()
    
    # Load data
    success = load_from_csv(server, database, username, password)
    
    if success:
        print("✅ Loading completed successfully!")
    else:
        print("❌ Loading failed!")
        exit(1)

if __name__ == "__main__":
    main()
"#
        .to_string()
    }

    /// Convert generated DAG to actual DAG structure
    fn convert_generated_dag_to_dag(&self, generated: GeneratedDAG) -> Result<Dag> {
        let mut dag = Dag::new(generated.dag_name);

        // Add tasks
        for task in generated.tasks {
            let mut task_config = TaskConfig::default();
            task_config.task_type = Some(task.task_type.clone());

            // Set task-specific configuration
            match task.task_type.as_str() {
                "shell" => {
                    if let Some(command) = task.configuration.get("command") {
                        task_config.command = Some(command.as_str().unwrap_or("").to_string());
                    }
                }
                "python" => {
                    if let Some(module) = task.configuration.get("module") {
                        task_config.module = Some(module.as_str().unwrap_or("").to_string());
                    }
                    if let Some(function) = task.configuration.get("function") {
                        task_config.function = Some(function.as_str().unwrap_or("").to_string());
                    }
                    if let Some(script) = task.configuration.get("script") {
                        task_config.data = Some(script.clone());
                    }
                }
                "http" => {
                    if let Some(method) = task.configuration.get("method") {
                        task_config.method = Some(method.as_str().unwrap_or("").to_string());
                    }
                    if let Some(url) = task.configuration.get("url") {
                        task_config.url = Some(url.as_str().unwrap_or("").to_string());
                    }
                }
                "file" => {
                    if let Some(operation) = task.configuration.get("operation") {
                        task_config.operation = Some(operation.as_str().unwrap_or("").to_string());
                    }
                    if let Some(destination) = task.configuration.get("destination") {
                        task_config.dest = Some(destination.as_str().unwrap_or("").to_string());
                    }
                }
                _ => {}
            }

            let task = Task {
                name: task.name,
                description: Some(task.description),
                config: task_config,
            };

            dag.add_task(task);
        }

        // Add dependencies
        for dep in generated.dependencies {
            dag.add_dependency(dep.from_task, dep.to_task);
        }

        Ok(dag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_data_pipeline_generation() {
        let generator = NaturalLanguageGenerator::new(std::sync::Arc::new(
            crate::ai::RemoteAPIProvider::new().unwrap(),
        ));

        let description = "Create a data pipeline to process customer data";
        let result = generator.generate_dag(description).await;

        assert!(result.is_ok());
        let dag = result.unwrap();
        assert_eq!(dag.name, "data_pipeline");
        assert!(dag.tasks.len() >= 4);
        assert!(dag.dependencies.len() >= 3);
    }

    #[tokio::test]
    async fn test_etl_generation() {
        let generator = NaturalLanguageGenerator::new(std::sync::Arc::new(
            crate::ai::RemoteAPIProvider::new().unwrap(),
        ));

        let description = "Create an ETL pipeline";
        let result = generator.generate_dag(description).await;

        assert!(result.is_ok());
        let dag = result.unwrap();
        assert_eq!(dag.name, "etl_pipeline");
        assert_eq!(dag.tasks.len(), 3);
        assert_eq!(dag.dependencies.len(), 2);
    }

    #[tokio::test]
    async fn test_generic_generation() {
        let generator = NaturalLanguageGenerator::new(std::sync::Arc::new(
            crate::ai::RemoteAPIProvider::new().unwrap(),
        ));

        let description = "Create a simple workflow";
        let result = generator.generate_dag(description).await;

        assert!(result.is_ok());
        let dag = result.unwrap();
        assert_eq!(dag.name, "generic_dag");
        assert_eq!(dag.tasks.len(), 2);
        assert_eq!(dag.dependencies.len(), 1);
    }
}
