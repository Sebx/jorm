use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tera::Tera;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub daemon: DaemonConfig,
    pub scheduler: SchedulerSettings,
    pub triggers: TriggerConfig,
    pub templates: TemplateConfig,
    pub environments: HashMap<String, EnvironmentConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub pid_file: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
    pub log_level: LogLevel,
    pub max_concurrent_jobs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerSettings {
    pub check_interval_seconds: u64,
    pub job_timeout_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
    pub cleanup_completed_jobs_after_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    pub webhook: WebhookConfig,
    pub file_watcher: FileWatcherConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub enabled: bool,
    pub port: u16,
    pub bind_address: String,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWatcherConfig {
    pub enabled: bool,
    pub debounce_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub enabled: bool,
    pub template_dir: Option<PathBuf>,
    pub auto_reload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub variables: HashMap<String, String>,
    pub working_directory: Option<PathBuf>,
    pub python_path: Option<String>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig::default(),
            scheduler: SchedulerSettings::default(),
            triggers: TriggerConfig::default(),
            templates: TemplateConfig::default(),
            environments: HashMap::new(),
        }
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            pid_file: Some(PathBuf::from("/tmp/jorm-scheduler.pid")),
            log_file: Some(PathBuf::from("/tmp/jorm-scheduler.log")),
            log_level: LogLevel::Info,
            max_concurrent_jobs: num_cpus::get(),
        }
    }
}

impl Default for SchedulerSettings {
    fn default() -> Self {
        Self {
            check_interval_seconds: 60,
            job_timeout_seconds: 3600, // 1 hour
            max_retries: 3,
            retry_delay_seconds: 60,
            cleanup_completed_jobs_after_days: 30,
        }
    }
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            webhook: WebhookConfig::default(),
            file_watcher: FileWatcherConfig::default(),
        }
    }
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 8080,
            bind_address: "127.0.0.1".to_string(),
            auth_token: None,
        }
    }
}

impl Default for FileWatcherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            debounce_ms: 1000,
        }
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            template_dir: None,
            auto_reload: true,
        }
    }
}

pub struct ConfigManager {
    config: SchedulerConfig,
    template_engine: Option<Tera>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            config: SchedulerConfig::default(),
            template_engine: None,
        }
    }

    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path).context("Failed to read config file")?;

        let config: SchedulerConfig = match path.as_ref().extension().and_then(|s| s.to_str()) {
            Some("yaml") | Some("yml") => {
                serde_yaml::from_str(&content).context("Failed to parse YAML config")?
            }
            Some("json") => {
                serde_json::from_str(&content).context("Failed to parse JSON config")?
            }
            Some("toml") => {
                // TOML parsing would go here if needed
                serde_yaml::from_str(&content)
                    .context("Failed to parse config as YAML (TOML not implemented)")?
            }
            _ => {
                // Try to detect format from content
                if content.trim_start().starts_with('{') {
                    serde_json::from_str(&content).context("Failed to parse JSON config")?
                } else {
                    serde_yaml::from_str(&content).context("Failed to parse YAML config")?
                }
            }
        };

        let mut manager = Self {
            config,
            template_engine: None,
        };

        // Initialize template engine if enabled
        if manager.config.templates.enabled {
            manager.init_template_engine()?;
        }

        Ok(manager)
    }

    pub fn config(&self) -> &SchedulerConfig {
        &self.config
    }

    pub fn get_environment(&self, name: &str) -> Option<&EnvironmentConfig> {
        self.config.environments.get(name)
    }

    pub fn render_template(&self, template_name: &str, context: &tera::Context) -> Result<String> {
        if let Some(tera) = &self.template_engine {
            tera.render(template_name, context)
                .context("Failed to render template")
        } else {
            anyhow::bail!("Template engine not initialized")
        }
    }

    pub fn render_string(&self, template: &str, context: &tera::Context) -> Result<String> {
        if let Some(tera) = &self.template_engine {
            // Create a temporary Tera instance for rendering
            let mut temp_tera = tera.clone();
            temp_tera
                .render_str(template, context)
                .context("Failed to render template string")
        } else {
            anyhow::bail!("Template engine not initialized")
        }
    }

    pub fn substitute_variables(&self, text: &str, environment: Option<&str>) -> Result<String> {
        let mut context = tera::Context::new();

        // Add environment variables
        for (key, value) in std::env::vars() {
            context.insert(&format!("env.{}", key), &value);
        }

        // Add environment-specific variables
        if let Some(env_name) = environment {
            if let Some(env_config) = self.get_environment(env_name) {
                for (key, value) in &env_config.variables {
                    context.insert(key, value);
                }
            }
        }

        // Add current timestamp
        context.insert(
            "now",
            &chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
        );
        context.insert("timestamp", &chrono::Utc::now().timestamp());

        if let Some(tera) = &self.template_engine {
            // Create a temporary Tera instance for rendering
            let mut temp_tera = tera.clone();
            temp_tera
                .render_str(text, &context)
                .context("Failed to substitute variables")
        } else {
            // Simple variable substitution without template engine
            let mut result = text.to_string();

            // Replace environment variables
            for (key, value) in std::env::vars() {
                let pattern = format!("${{{}}}", key);
                result = result.replace(&pattern, &value);
                let pattern = format!("${{env.{}}}", key);
                result = result.replace(&pattern, &value);
            }

            Ok(result)
        }
    }

    fn init_template_engine(&mut self) -> Result<()> {
        let mut tera = if let Some(template_dir) = &self.config.templates.template_dir {
            let pattern = template_dir.join("**/*").to_string_lossy().to_string();
            Tera::new(&pattern).context("Failed to initialize template engine with directory")?
        } else {
            Tera::new("").context("Failed to initialize template engine")?
        };

        // Add custom filters and functions
        tera.register_filter("env", env_filter);
        tera.register_function("now", now_function);

        if self.config.templates.auto_reload {
            tera.autoescape_on(vec![".html", ".xml"]);
        }

        self.template_engine = Some(tera);
        Ok(())
    }
}

fn env_filter(value: &tera::Value, _: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    if let Some(var_name) = value.as_str() {
        if let Ok(env_value) = std::env::var(var_name) {
            Ok(tera::Value::String(env_value))
        } else {
            Ok(tera::Value::Null)
        }
    } else {
        Err(tera::Error::msg("env filter requires a string argument"))
    }
}

fn now_function(_: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let now = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();
    Ok(tera::Value::String(now))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = SchedulerConfig::default();
        assert_eq!(config.scheduler.check_interval_seconds, 60);
        assert_eq!(config.daemon.max_concurrent_jobs, num_cpus::get());
        assert!(!config.triggers.webhook.enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = SchedulerConfig::default();

        // Test YAML serialization
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: SchedulerConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(
            config.scheduler.check_interval_seconds,
            deserialized.scheduler.check_interval_seconds
        );

        // Test JSON serialization
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SchedulerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            config.scheduler.check_interval_seconds,
            deserialized.scheduler.check_interval_seconds
        );
    }

    #[test]
    fn test_config_manager_creation() {
        let manager = ConfigManager::new();
        assert_eq!(manager.config().scheduler.check_interval_seconds, 60);
    }

    #[test]
    fn test_variable_substitution() {
        let manager = ConfigManager::new();
        std::env::set_var("TEST_VAR", "test_value");

        let result = manager
            .substitute_variables("Hello ${TEST_VAR}!", None)
            .unwrap();
        assert_eq!(result, "Hello test_value!");

        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_load_yaml_config() {
        let config_content = r#"
daemon:
  pid_file: "/tmp/test.pid"
  log_level: "Debug"
  max_concurrent_jobs: 4
scheduler:
  check_interval_seconds: 30
  job_timeout_seconds: 1800
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();

        let manager = ConfigManager::load_from_file(temp_file.path()).unwrap();
        assert_eq!(manager.config().scheduler.check_interval_seconds, 30);
        assert_eq!(manager.config().scheduler.job_timeout_seconds, 1800);
        assert_eq!(manager.config().daemon.max_concurrent_jobs, 4);
    }
}
