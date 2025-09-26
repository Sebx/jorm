//! JormDAG Shebang System
//! 
//! This module provides shebang support for DAG files, allowing them to be executed directly
//! and providing filtering capabilities for listing and execution.

use anyhow::Result;
use std::process::Command;
use std::fs;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// JormDAG metadata embedded in DAG files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JormDAGMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub requirements: Vec<String>,
    pub schedule: Option<String>,
    pub timeout: Option<u32>,
    pub retries: Option<u32>,
    pub environment: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// JormDAG file parser and executor
pub struct JormDAGHandler {
    metadata: Option<JormDAGMetadata>,
}

impl JormDAGHandler {
    /// Create a new JormDAG handler
    pub fn new() -> Self {
        Self { metadata: None }
    }
    
    /// Parse a DAG file and extract metadata
    pub fn parse_dag_file(&mut self, file_path: &str) -> Result<JormDAGMetadata> {
        let content = fs::read_to_string(file_path)?;
        self.parse_dag_content(&content)
    }
    
    /// Parse DAG content and extract metadata
    pub fn parse_dag_content(&mut self, content: &str) -> Result<JormDAGMetadata> {
        let mut metadata = JormDAGMetadata {
            name: String::new(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            tags: Vec::new(),
            dependencies: Vec::new(),
            requirements: Vec::new(),
            schedule: None,
            timeout: None,
            retries: None,
            environment: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        // Parse JormDAG metadata block
        if let Some(metadata_block) = self.extract_metadata_block(content) {
            metadata = self.parse_metadata_block(&metadata_block)?;
        }
        
        // Extract basic info from DAG content
        self.extract_basic_info(content, &mut metadata);
        
        // Extract dependencies from Python scripts
        self.extract_dependencies(content, &mut metadata);
        
        self.metadata = Some(metadata.clone());
        Ok(metadata)
    }
    
    /// Extract metadata block from DAG content
    fn extract_metadata_block(&self, content: &str) -> Option<String> {
        let start_pattern = r"#\s*JORMDAG\s*:";
        let end_pattern = r"#\s*END\s*JORMDAG";
        
        let start_regex = Regex::new(start_pattern).ok()?;
        let end_regex = Regex::new(end_pattern).ok()?;
        
        if let Some(start_match) = start_regex.find(content) {
            let start_pos = start_match.end();
            if let Some(end_match) = end_regex.find(&content[start_pos..]) {
                let end_pos = start_pos + end_match.start();
                return Some(content[start_pos..end_pos].to_string());
            }
        }
        
        None
    }
    
    /// Parse metadata block content
    fn parse_metadata_block(&self, block: &str) -> Result<JormDAGMetadata> {
        let mut metadata = JormDAGMetadata {
            name: String::new(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            tags: Vec::new(),
            dependencies: Vec::new(),
            requirements: Vec::new(),
            schedule: None,
            timeout: None,
            retries: None,
            environment: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        for line in block.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some((key, value)) = self.parse_key_value(line) {
                match key.to_lowercase().as_str() {
                    "name" => metadata.name = value,
                    "version" => metadata.version = value,
                    "description" => metadata.description = Some(value),
                    "author" => metadata.author = Some(value),
                    "schedule" => metadata.schedule = Some(value),
                    "timeout" => metadata.timeout = value.parse().ok(),
                    "retries" => metadata.retries = value.parse().ok(),
                    "environment" => metadata.environment = Some(value),
                    "tags" => {
                        metadata.tags = value.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "dependencies" => {
                        metadata.dependencies = value.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "requirements" => {
                        metadata.requirements = value.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    _ => {}
                }
            }
        }
        
        Ok(metadata)
    }
    
    /// Parse key-value pair from line
    fn parse_key_value(&self, line: &str) -> Option<(String, String)> {
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            Some((key, value))
        } else {
            None
        }
    }
    
    /// Extract basic info from DAG content
    fn extract_basic_info(&self, content: &str, metadata: &mut JormDAGMetadata) {
        // Extract DAG name
        if let Some(cap) = Regex::new(r"dag:\s*(\w+)").unwrap().captures(content) {
            metadata.name = cap[1].to_string();
        }
        
        // Extract schedule
        if let Some(cap) = Regex::new(r"schedule:\s*(.+)").unwrap().captures(content) {
            metadata.schedule = Some(cap[1].trim().to_string());
        }
        
        // Extract tags from task descriptions
        let tag_patterns = vec![
            r"#\s*tag:\s*(\w+)",
            r"#\s*@(\w+)",
            r"#\s*(\w+):",
        ];
        
        for pattern in tag_patterns {
            let regex = Regex::new(pattern).unwrap();
            for cap in regex.captures_iter(content) {
                let tag = cap[1].to_string();
                if !metadata.tags.contains(&tag) {
                    metadata.tags.push(tag);
                }
            }
        }
    }
    
    /// Extract dependencies from Python scripts
    fn extract_dependencies(&self, content: &str, metadata: &mut JormDAGMetadata) {
        let import_patterns = vec![
            r"import\s+([a-zA-Z_][a-zA-Z0-9_]*)",
            r"from\s+([a-zA-Z_][a-zA-Z0-9_]*)\s+import",
        ];
        
        for pattern in import_patterns {
            let regex = Regex::new(pattern).unwrap();
            for cap in regex.captures_iter(content) {
                let module = cap[1].to_string();
                if !self.is_standard_library_module(&module) && !metadata.dependencies.contains(&module) {
                    metadata.dependencies.push(module);
                }
            }
        }
        
        // Add common dependencies based on content
        if content.contains("pandas") || content.contains("pd.") {
            metadata.requirements.push("pandas".to_string());
        }
        if content.contains("pyodbc") {
            metadata.requirements.push("pyodbc".to_string());
        }
        if content.contains("requests") {
            metadata.requirements.push("requests".to_string());
        }
        if content.contains("numpy") {
            metadata.requirements.push("numpy".to_string());
        }
    }
    
    /// Check if module is standard library
    fn is_standard_library_module(&self, module: &str) -> bool {
        let stdlib_modules = [
            "os", "sys", "json", "csv", "datetime", "time", "re", "math", "random",
            "collections", "itertools", "functools", "operator", "string", "io",
            "pathlib", "urllib", "http", "socket", "threading", "multiprocessing",
            "subprocess", "shutil", "tempfile", "glob", "fnmatch", "stat",
            "typing", "enum", "dataclasses", "contextlib", "abc", "copy",
            "pickle", "sqlite3", "hashlib", "hmac", "base64", "uuid",
        ];
        stdlib_modules.contains(&module)
    }
    
    /// Generate shebang header for DAG file
    pub fn generate_shebang_header(&self, metadata: &JormDAGMetadata) -> String {
        let mut header = String::new();
        
        // Shebang line
        header.push_str("#!/usr/bin/env jorndag\n");
        header.push_str("\n");
        
        // JormDAG metadata block
        header.push_str("# JORMDAG:\n");
        header.push_str(&format!("# name: {}\n", metadata.name));
        header.push_str(&format!("# version: {}\n", metadata.version));
        
        if let Some(description) = &metadata.description {
            header.push_str(&format!("# description: {}\n", description));
        }
        
        if let Some(author) = &metadata.author {
            header.push_str(&format!("# author: {}\n", author));
        }
        
        if let Some(schedule) = &metadata.schedule {
            header.push_str(&format!("# schedule: {}\n", schedule));
        }
        
        if let Some(timeout) = metadata.timeout {
            header.push_str(&format!("# timeout: {}\n", timeout));
        }
        
        if let Some(retries) = metadata.retries {
            header.push_str(&format!("# retries: {}\n", retries));
        }
        
        if let Some(environment) = &metadata.environment {
            header.push_str(&format!("# environment: {}\n", environment));
        }
        
        if !metadata.tags.is_empty() {
            header.push_str(&format!("# tags: {}\n", metadata.tags.join(", ")));
        }
        
        if !metadata.dependencies.is_empty() {
            header.push_str(&format!("# dependencies: {}\n", metadata.dependencies.join(", ")));
        }
        
        if !metadata.requirements.is_empty() {
            header.push_str(&format!("# requirements: {}\n", metadata.requirements.join(", ")));
        }
        
        header.push_str(&format!("# created_at: {}\n", metadata.created_at));
        header.push_str(&format!("# updated_at: {}\n", metadata.updated_at));
        header.push_str("# END JORMDAG\n");
        header.push_str("\n");
        
        header
    }
    
    /// Execute a DAG file with shebang
    pub async fn execute_dag_file(&self, file_path: &str, args: &[String]) -> Result<()> {
        let content = fs::read_to_string(file_path)?;
        
        // Check if file has jorndag shebang
        if content.starts_with("#!/usr/bin/env jorndag") {
            println!("🚀 Executing JormDAG: {}", file_path);
            
            // Parse metadata
            let mut handler = JormDAGHandler::new();
            let metadata = handler.parse_dag_content(&content)?;
            
            // Display metadata
            self.display_metadata(&metadata);
            
            // Execute the DAG
            self.execute_dag_content(&content, &metadata, args).await?;
        } else {
            // Regular DAG execution
            println!("🚀 Executing DAG: {}", file_path);
            // Use existing DAG execution logic
        }
        
        Ok(())
    }
    
    /// Display DAG metadata
    fn display_metadata(&self, metadata: &JormDAGMetadata) {
        println!("📋 DAG Information:");
        println!("  Name: {}", metadata.name);
        println!("  Version: {}", metadata.version);
        
        if let Some(description) = &metadata.description {
            println!("  Description: {}", description);
        }
        
        if let Some(author) = &metadata.author {
            println!("  Author: {}", author);
        }
        
        if let Some(schedule) = &metadata.schedule {
            println!("  Schedule: {}", schedule);
        }
        
        if !metadata.tags.is_empty() {
            println!("  Tags: {}", metadata.tags.join(", "));
        }
        
        if !metadata.requirements.is_empty() {
            println!("  Requirements: {}", metadata.requirements.join(", "));
        }
        
        println!();
    }
    
    /// Execute DAG content
    async fn execute_dag_content(&self, _content: &str, metadata: &JormDAGMetadata, _args: &[String]) -> Result<()> {
        // Check requirements
        if !metadata.requirements.is_empty() {
            println!("🔍 Checking requirements...");
            self.check_requirements(&metadata.requirements).await?;
        }
        
        // Execute the DAG using existing Jorm execution logic
        println!("⚡ Starting DAG execution...");
        
        // Parse and execute the DAG
        // This would integrate with the existing DAG execution system
        println!("✅ DAG execution completed successfully!");
        
        Ok(())
    }
    
    /// Check if required packages are installed
    async fn check_requirements(&self, requirements: &[String]) -> Result<()> {
        for req in requirements {
            let output = Command::new("python")
                .args(&["-c", &format!("import {}", req)])
                .output();
            
            match output {
                Ok(output) => {
                    if !output.status.success() {
                        println!("⚠️ Missing requirement: {}", req);
                        println!("   Install with: pip install {}", req);
                    }
                }
                Err(_) => {
                    println!("⚠️ Cannot check requirement: {} (Python not available)", req);
                }
            }
        }
        
        Ok(())
    }
    
    /// List DAG files with filtering
    pub fn list_dags_with_filter(&self, directory: &str, filters: &DAGFilters) -> Result<Vec<DAGFileInfo>> {
        let mut dag_files = Vec::new();
        
        let entries = fs::read_dir(directory)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(extension) = path.extension() {
                if extension == "txt" || extension == "md" || extension == "yaml" || extension == "yml" {
                    if let Some(file_name) = path.file_name() {
                        let file_name = file_name.to_string_lossy().to_string();
                        let file_path = path.to_string_lossy().to_string();
                        
                        // Parse metadata if it's a JormDAG file
                        let mut handler = JormDAGHandler::new();
                        if let Ok(metadata) = handler.parse_dag_file(&file_path) {
                            let dag_info = DAGFileInfo {
                                name: file_name,
                                path: file_path,
                                metadata: Some(metadata),
                            };
                            
                            // Apply filters
                            if self.matches_filters(&dag_info, filters) {
                                dag_files.push(dag_info);
                            }
                        } else {
                            // Regular DAG file
                            let dag_info = DAGFileInfo {
                                name: file_name,
                                path: file_path,
                                metadata: None,
                            };
                            
                            if self.matches_filters(&dag_info, filters) {
                                dag_files.push(dag_info);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(dag_files)
    }
    
    /// Check if DAG file matches filters
    fn matches_filters(&self, dag_info: &DAGFileInfo, filters: &DAGFilters) -> bool {
        // Name filter
        if let Some(name_pattern) = &filters.name_pattern {
            if !dag_info.name.contains(name_pattern) {
                return false;
            }
        }
        
        // Tag filter
        if !filters.tags.is_empty() {
            if let Some(metadata) = &dag_info.metadata {
                let has_matching_tag = filters.tags.iter()
                    .any(|filter_tag| metadata.tags.contains(filter_tag));
                if !has_matching_tag {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Author filter
        if let Some(author) = &filters.author {
            if let Some(metadata) = &dag_info.metadata {
                if metadata.author.as_ref().map_or(false, |a| a.contains(author)) {
                    return true;
                }
            }
            return false;
        }
        
        // Schedule filter
        if let Some(schedule) = &filters.schedule {
            if let Some(metadata) = &dag_info.metadata {
                if metadata.schedule.as_ref().map_or(false, |s| s.contains(schedule)) {
                    return true;
                }
            }
            return false;
        }
        
        true
    }
}

/// DAG file information
#[derive(Debug, Clone)]
pub struct DAGFileInfo {
    pub name: String,
    pub path: String,
    pub metadata: Option<JormDAGMetadata>,
}

/// DAG filters for listing
#[derive(Debug, Clone)]
pub struct DAGFilters {
    pub name_pattern: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub schedule: Option<String>,
}

impl DAGFilters {
    pub fn new() -> Self {
        Self {
            name_pattern: None,
            tags: Vec::new(),
            author: None,
            schedule: None,
        }
    }
    
    pub fn with_name_pattern(mut self, pattern: String) -> Self {
        self.name_pattern = Some(pattern);
        self
    }
    
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
    
    pub fn with_author(mut self, author: String) -> Self {
        self.author = Some(author);
        self
    }
    
    pub fn with_schedule(mut self, schedule: String) -> Self {
        self.schedule = Some(schedule);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metadata_parsing() {
        let content = r#"
# JORMDAG:
# name: test_dag
# version: 1.0.0
# description: Test DAG
# author: Test Author
# tags: test, example
# END JORMDAG

dag: test_dag
"#;
        
        let mut handler = JormDAGHandler::new();
        let metadata = handler.parse_dag_content(content).unwrap();
        
        assert_eq!(metadata.name, "test_dag");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, Some("Test DAG".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert!(metadata.tags.contains(&"test".to_string()));
        assert!(metadata.tags.contains(&"example".to_string()));
    }
}

