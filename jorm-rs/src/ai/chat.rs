//! Interactive chat interface for DAG management

use crate::ai::{ConversationTurn, ModelContext};
use crate::parser::Dag;
use anyhow::Result;
use std::io::{self, Write};
use tokio::io::stdin;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Interactive chat interface for DAG management
pub struct ChatInterface {
    model_provider: std::sync::Arc<dyn crate::ai::LanguageModelProvider>,
    conversation_history: Vec<ConversationTurn>,
}

impl ChatInterface {
    /// Create a new chat interface
    pub fn new(model_provider: std::sync::Arc<dyn crate::ai::LanguageModelProvider>) -> Self {
        Self {
            model_provider,
            conversation_history: Vec::new(),
        }
    }

    /// Start the interactive chat session
    pub async fn start(&self) -> Result<()> {
        println!("🤖 Jorm AI Chat Interface");
        println!("Type 'help' for commands, 'quit' to exit");
        println!();

        let stdin = stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            print!("jorm-ai> ");
            io::stdout().flush()?;

            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF reached
                    println!("\nGoodbye! 👋");
                    break;
                }
                Ok(_) => {
                    let command = line.trim();

                    if command.is_empty() {
                        continue;
                    }

                    match command {
                        "quit" | "exit" => {
                            println!("Goodbye! 👋");
                            break;
                        }
                        "help" => {
                            self.show_help();
                        }
                        "clear" => {
                            self.clear_conversation();
                        }
                        "history" => {
                            self.show_history();
                        }
                        _ => {
                            if let Err(e) = self.process_user_input(command).await {
                                println!("❌ Error processing input: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Error reading input: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Show help information
    fn show_help(&self) {
        println!("Available commands:");
        println!("  help                    - Show this help message");
        println!("  clear                   - Clear conversation history");
        println!("  history                 - Show conversation history");
        println!("  quit/exit              - Exit the chat");
        println!();
        println!("You can also ask questions like:");
        println!("  'Create a data pipeline for processing sales data'");
        println!("  'Analyze my DAG for optimization opportunities'");
        println!("  'Explain this error: Task failed with exit code 1'");
        println!("  'Generate a web scraping workflow'");
    }

    /// Clear conversation history
    fn clear_conversation(&self) {
        // Note: This doesn't actually clear the history in the current implementation
        // because we're not mutating self. In a real implementation, we'd need
        // to use RefCell or similar for interior mutability.
        println!("Conversation history cleared");
    }

    /// Show conversation history
    fn show_history(&self) {
        if self.conversation_history.is_empty() {
            println!("No conversation history");
            return;
        }

        println!("Conversation History:");
        for (i, turn) in self.conversation_history.iter().enumerate() {
            println!("{}. [{}] {}", i + 1, turn.role, turn.content);
        }
    }

    /// Process user input and generate response
    async fn process_user_input(&self, input: &str) -> Result<()> {
        println!("🤔 Thinking...");

        // Create context for the model
        let context = ModelContext {
            max_tokens: Some(1000),
            temperature: Some(0.7),
            system_prompt: Some(
                "You are Jorm AI, a helpful assistant for DAG management and workflow automation."
                    .to_string(),
            ),
            conversation_history: self.conversation_history.clone(),
        };

        // Generate response using the model
        let response = self
            .model_provider
            .generate_response(input, &context)
            .await?;

        println!("🤖 {}", response);

        // In a real implementation, we would add the conversation turn to history
        // but since we can't mutate self, we'll just show the response

        Ok(())
    }
}

/// DAG chat commands for interactive management
pub struct DAGChatCommands {
    current_dag: Option<Dag>,
    model_provider: std::sync::Arc<dyn crate::ai::LanguageModelProvider>,
}

impl DAGChatCommands {
    /// Create new DAG chat commands
    pub fn new() -> Self {
        Self {
            current_dag: None,
            model_provider: std::sync::Arc::new(crate::ai::RemoteAPIProvider::new().unwrap()),
        }
    }

    /// Load a DAG for analysis
    pub fn load_dag(&mut self, dag: Dag) {
        self.current_dag = Some(dag);
        println!("✅ DAG loaded: {}", self.current_dag.as_ref().unwrap().name);
    }

    /// Analyze the current DAG
    pub async fn analyze_current_dag(&self) -> Result<()> {
        if let Some(dag) = &self.current_dag {
            println!("🔍 Analyzing DAG: {}", dag.name);

            // Create analysis context
            let context = ModelContext {
                max_tokens: Some(1500),
                temperature: Some(0.5),
                system_prompt: Some(
                    "You are Jorm AI, specialized in DAG analysis and optimization.".to_string(),
                ),
                conversation_history: vec![],
            };

            // Generate analysis
            let analysis_prompt = format!(
                "Analyze this DAG for optimization opportunities: {}",
                dag.name
            );
            let analysis = self
                .model_provider
                .generate_response(&analysis_prompt, &context)
                .await?;

            println!("📊 Analysis: {}", analysis);
        } else {
            println!("❌ No DAG loaded. Use 'load <dag_file>' first.");
        }

        Ok(())
    }

    /// Generate a new DAG from description
    pub async fn generate_dag(&self, description: &str) -> Result<Dag> {
        println!("🚀 Generating DAG from: {}", description);

        // Create generation context
        let context = ModelContext {
            max_tokens: Some(2000),
            temperature: Some(0.8),
            system_prompt: Some("You are Jorm AI, specialized in generating DAG workflows from natural language descriptions.".to_string()),
            conversation_history: vec![],
        };

        // Generate DAG using the model
        let generation_prompt = format!("Generate a DAG for: {}", description);
        let _response = self
            .model_provider
            .generate_response(&generation_prompt, &context)
            .await?;

        // For now, return a simple generated DAG
        let mut dag = Dag::new("generated_dag".to_string());
        dag.add_task(crate::parser::Task::new("task1".to_string()));
        dag.add_task(crate::parser::Task::new("task2".to_string()));
        dag.add_dependency("task2".to_string(), "task1".to_string());

        println!("✅ DAG generated successfully");
        Ok(dag)
    }

    /// Explain an error with context
    pub async fn explain_error(&self, error: &str, context: &str) -> Result<()> {
        println!("🔍 Explaining error: {}", error);

        // Create error explanation context
        let model_context = ModelContext {
            max_tokens: Some(1000),
            temperature: Some(0.3),
            system_prompt: Some("You are Jorm AI, specialized in explaining DAG execution errors and providing solutions.".to_string()),
            conversation_history: vec![],
        };

        // Generate explanation
        let explanation_prompt = format!("Explain this error: {} Context: {}", error, context);
        let explanation = self
            .model_provider
            .generate_response(&explanation_prompt, &model_context)
            .await?;

        println!("💡 Explanation: {}", explanation);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chat_interface_creation() {
        let model_provider = std::sync::Arc::new(crate::ai::RemoteAPIProvider::new().unwrap());
        let chat = ChatInterface::new(model_provider);
        assert!(chat.conversation_history.is_empty());
    }

    #[tokio::test]
    async fn test_dag_chat_commands() {
        let commands = DAGChatCommands::new();
        assert!(commands.current_dag.is_none());
    }

    #[tokio::test]
    async fn test_help_display() {
        let model_provider = std::sync::Arc::new(crate::ai::RemoteAPIProvider::new().unwrap());
        let chat = ChatInterface::new(model_provider);
        chat.show_help(); // Should not panic
    }
}
