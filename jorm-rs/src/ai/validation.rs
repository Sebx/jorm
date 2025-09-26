//! Script validation and auto-fixing system
//! 
//! This module provides comprehensive validation for generated Python scripts,
//! including dependency detection, syntax checking, and AI-powered error fixing.

use anyhow::{Result, Context};
use std::collections::HashSet;
use std::process::Command;
use std::path::Path;
use regex::Regex;

/// Script validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub dependencies: Vec<String>,
    pub fixed_script: Option<String>,
}

/// Validation error with context
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub error_type: ErrorType,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub suggestion: Option<String>,
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub warning_type: WarningType,
    pub message: String,
    pub line: Option<usize>,
    pub suggestion: Option<String>,
}

/// Types of validation errors
#[derive(Debug, Clone)]
pub enum ErrorType {
    SyntaxError,
    ImportError,
    IndentationError,
    NameError,
    TypeError,
    AttributeError,
    MissingDependency,
    RuntimeError,
}

/// Types of validation warnings
#[derive(Debug, Clone)]
pub enum WarningType {
    UnusedImport,
    DeprecatedFunction,
    PerformanceIssue,
    SecurityRisk,
    BestPractice,
}

/// Script validator and auto-fixer
pub struct ScriptValidator {
    ai_provider: std::sync::Arc<dyn crate::ai::LanguageModelProvider>,
}

impl ScriptValidator {
    /// Create a new script validator
    pub fn new(ai_provider: std::sync::Arc<dyn crate::ai::LanguageModelProvider>) -> Self {
        Self { ai_provider }
    }
    
    /// Validate a Python script comprehensively
    pub async fn validate_script(&self, script: &str, script_name: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            dependencies: Vec::new(),
            fixed_script: None,
        };
        
        // Step 1: Detect dependencies
        result.dependencies = self.detect_dependencies(script).await?;
        
        // Step 2: Check syntax with dry-run
        let syntax_result = self.check_syntax(script).await?;
        if !syntax_result.is_valid {
            result.is_valid = false;
            result.errors.extend(syntax_result.errors);
        }
        
        // Step 3: Check for common issues
        let common_issues = self.check_common_issues(script).await?;
        result.warnings.extend(common_issues);
        
        // Step 4: If there are errors, try to fix them with AI
        if !result.errors.is_empty() {
            println!("🔧 Found {} errors, attempting AI-powered fixes...", result.errors.len());
            if let Ok(fixed_script) = self.fix_script_with_ai(script, &result.errors).await {
                result.fixed_script = Some(fixed_script.clone());
                
                // Re-validate the fixed script
                let fixed_validation = self.validate_script(&fixed_script, script_name).await?;
                if fixed_validation.is_valid {
                    println!("✅ AI successfully fixed all errors!");
                    result.is_valid = true;
                    result.errors.clear();
                } else {
                    println!("⚠️ AI fixed some errors, but {} remain", fixed_validation.errors.len());
                    result.errors = fixed_validation.errors;
                }
            }
        }
        
        Ok(result)
    }
    
    /// Detect Python dependencies from script
    async fn detect_dependencies(&self, script: &str) -> Result<Vec<String>> {
        let mut dependencies = HashSet::new();
        
        // Common patterns for imports
        let import_patterns = vec![
            r"import\s+([a-zA-Z_][a-zA-Z0-9_]*)",
            r"from\s+([a-zA-Z_][a-zA-Z0-9_]*)\s+import",
            r"import\s+([a-zA-Z_][a-zA-Z0-9_]*)\s+as",
        ];
        
        for pattern in import_patterns {
            let regex = Regex::new(pattern)?;
            for cap in regex.captures_iter(script) {
                if let Some(module) = cap.get(1) {
                    let module_name = module.as_str();
                    // Filter out standard library modules
                    if !self.is_standard_library_module(module_name) {
                        dependencies.insert(module_name.to_string());
                    }
                }
            }
        }
        
        // Add common dependencies that might be missing
        if script.contains("pandas") || script.contains("pd.") {
            dependencies.insert("pandas".to_string());
        }
        if script.contains("pyodbc") || script.contains("connect") {
            dependencies.insert("pyodbc".to_string());
        }
        if script.contains("requests") || script.contains("get(") || script.contains("post(") {
            dependencies.insert("requests".to_string());
        }
        if script.contains("json") && !script.contains("import json") {
            dependencies.insert("json".to_string());
        }
        if script.contains("os.") && !script.contains("import os") {
            dependencies.insert("os".to_string());
        }
        
        Ok(dependencies.into_iter().collect())
    }
    
    /// Check if a module is part of Python standard library
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
    
    /// Check Python syntax using dry-run mode
    async fn check_syntax(&self, script: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            dependencies: Vec::new(),
            fixed_script: None,
        };
        
        // Create a temporary file for syntax checking
        let temp_file = format!("temp_script_{}.py", uuid::Uuid::new_v4());
        tokio::fs::write(&temp_file, script).await?;
        
        // Run Python syntax check
        let output = Command::new("python")
            .args(&["-m", "py_compile", &temp_file])
            .output();
        
        // Clean up temp file
        let _ = tokio::fs::remove_file(&temp_file).await;
        
        match output {
            Ok(output) => {
                if !output.status.success() {
                    result.is_valid = false;
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    
                    // Parse Python error messages
                    for line in stderr.lines() {
                        if line.contains("SyntaxError") || line.contains("IndentationError") {
                            let error = self.parse_python_error(line, &script)?;
                            result.errors.push(error);
                        }
                    }
                }
            }
            Err(_) => {
                // Python not available, try alternative syntax checking
                result.warnings.push(ValidationWarning {
                    warning_type: WarningType::BestPractice,
                    message: "Python syntax check not available. Install Python for full validation.".to_string(),
                    line: None,
                    suggestion: Some("Install Python to enable syntax validation".to_string()),
                });
            }
        }
        
        Ok(result)
    }
    
    /// Parse Python error messages into ValidationError
    fn parse_python_error(&self, error_line: &str, script: &str) -> Result<ValidationError> {
        let mut error = ValidationError {
            error_type: ErrorType::SyntaxError,
            message: error_line.to_string(),
            line: None,
            column: None,
            suggestion: None,
        };
        
        // Try to extract line number
        if let Some(cap) = Regex::new(r"line (\d+)")?.captures(error_line) {
            if let Ok(line_num) = cap[1].parse::<usize>() {
                error.line = Some(line_num);
            }
        }
        
        // Try to extract column number
        if let Some(cap) = Regex::new(r"column (\d+)")?.captures(error_line) {
            if let Ok(col_num) = cap[1].parse::<usize>() {
                error.column = Some(col_num);
            }
        }
        
        // Determine error type
        if error_line.contains("IndentationError") {
            error.error_type = ErrorType::IndentationError;
        } else if error_line.contains("NameError") {
            error.error_type = ErrorType::NameError;
        } else if error_line.contains("TypeError") {
            error.error_type = ErrorType::TypeError;
        } else if error_line.contains("AttributeError") {
            error.error_type = ErrorType::AttributeError;
        }
        
        Ok(error)
    }
    
    /// Check for common issues and best practices
    async fn check_common_issues(&self, script: &str) -> Result<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();
        
        // Check for hardcoded credentials
        if script.contains("password") && script.contains("=") {
            warnings.push(ValidationWarning {
                warning_type: WarningType::SecurityRisk,
                message: "Hardcoded credentials detected".to_string(),
                line: None,
                suggestion: Some("Use environment variables or secure credential storage".to_string()),
            });
        }
        
        // Check for missing error handling
        if script.contains("connect(") && !script.contains("try:") {
            warnings.push(ValidationWarning {
                warning_type: WarningType::BestPractice,
                message: "Database connection without error handling".to_string(),
                line: None,
                suggestion: Some("Add try-except blocks around database operations".to_string()),
            });
        }
        
        // Check for potential performance issues
        if script.contains("for ") && script.contains("in ") && script.contains("iterrows()") {
            warnings.push(ValidationWarning {
                warning_type: WarningType::PerformanceIssue,
                message: "Using iterrows() can be slow for large DataFrames".to_string(),
                line: None,
                suggestion: Some("Consider using vectorized operations or to_dict('records')".to_string()),
            });
        }
        
        // Check for unused imports
        let lines: Vec<&str> = script.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim().starts_with("import ") {
                let module = line.trim().split_whitespace().nth(1).unwrap_or("");
                if !script.contains(&format!("{}.", module)) && !script.contains(&format!("{}(", module)) {
                    warnings.push(ValidationWarning {
                        warning_type: WarningType::UnusedImport,
                        message: format!("Unused import: {}", module),
                        line: Some(i + 1),
                        suggestion: Some("Remove unused import".to_string()),
                    });
                }
            }
        }
        
        Ok(warnings)
    }
    
    /// Use AI to fix script errors
    async fn fix_script_with_ai(&self, script: &str, errors: &[ValidationError]) -> Result<String> {
        let error_context = errors.iter()
            .map(|e| format!("- {}: {}", e.error_type, e.message))
            .collect::<Vec<_>>()
            .join("\n");
        
        let prompt = format!(
            "Fix the following Python script errors:\n\n{}\n\nScript:\n```python\n{}\n```\n\nProvide only the corrected Python code:",
            error_context,
            script
        );
        
        let context = crate::ai::ModelContext {
            conversation_history: Vec::new(),
        };
        
        let response = self.ai_provider.generate_response(&prompt, &context).await?;
        
        // Extract code from response (look for code blocks)
        if let Some(code) = self.extract_code_from_response(&response) {
            Ok(code)
        } else {
            Err(anyhow::anyhow!("AI response did not contain valid Python code"))
        }
    }
    
    /// Extract Python code from AI response
    fn extract_code_from_response(&self, response: &str) -> Option<String> {
        // Look for code blocks
        if let Some(start) = response.find("```python") {
            let start = start + 9; // Length of "```python"
            if let Some(end) = response[start..].find("```") {
                return Some(response[start..start + end].trim().to_string());
            }
        }
        
        // Look for code blocks without language specification
        if let Some(start) = response.find("```") {
            let start = start + 3;
            if let Some(end) = response[start..].find("```") {
                let code = response[start..start + end].trim();
                if code.contains("def ") || code.contains("import ") {
                    return Some(code.to_string());
                }
            }
        }
        
        // If no code blocks, return the whole response if it looks like Python
        if response.contains("def ") || response.contains("import ") {
            Some(response.trim().to_string())
        } else {
            None
        }
    }
    
    /// Generate requirements.txt content for dependencies
    pub fn generate_requirements(&self, dependencies: &[String]) -> String {
        let mut requirements = String::new();
        requirements.push_str("# Generated requirements for Jorm DAG\n");
        requirements.push_str("# Install with: pip install -r requirements.txt\n\n");
        
        for dep in dependencies {
            match dep.as_str() {
                "pandas" => requirements.push_str("pandas>=1.5.0\n"),
                "pyodbc" => requirements.push_str("pyodbc>=4.0.0\n"),
                "requests" => requirements.push_str("requests>=2.28.0\n"),
                "numpy" => requirements.push_str("numpy>=1.21.0\n"),
                "sqlalchemy" => requirements.push_str("sqlalchemy>=1.4.0\n"),
                "psycopg2" => requirements.push_str("psycopg2-binary>=2.9.0\n"),
                "mysql-connector" => requirements.push_str("mysql-connector-python>=8.0.0\n"),
                _ => requirements.push_str(&format!("{}\n", dep)),
            }
        }
        
        requirements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dependency_detection() {
        let validator = ScriptValidator::new(
            std::sync::Arc::new(crate::ai::RemoteAPIProvider::new().unwrap())
        );
        
        let script = r#"
import pandas as pd
import pyodbc
from typing import Optional

def main():
    df = pd.read_csv("data.csv")
    conn = pyodbc.connect("connection_string")
"#;
        
        let deps = validator.detect_dependencies(script).await.unwrap();
        assert!(deps.contains(&"pandas".to_string()));
        assert!(deps.contains(&"pyodbc".to_string()));
    }
    
    #[tokio::test]
    async fn test_syntax_validation() {
        let validator = ScriptValidator::new(
            std::sync::Arc::new(crate::ai::RemoteAPIProvider::new().unwrap())
        );
        
        let valid_script = "print('Hello, World!')";
        let result = validator.validate_script(valid_script, "test").await.unwrap();
        assert!(result.is_valid);
    }
}

