# JORM Native Executor Configuration Reference

This document provides a comprehensive reference for configuring the JORM native executor.

## Table of Contents

- [ExecutorConfig](#executorconfig)
- [StateConfig](#stateconfig)
- [ResourceLimits](#resourcelimits)
- [TaskRetryConfig](#taskretryconfig)
- [EnvironmentConfig](#environmentconfig)
- [Configuration Profiles](#configuration-profiles)
- [Environment Variables](#environment-variables)
- [Configuration Files](#configuration-files)

## ExecutorConfig

The main configuration structure for the native executor.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_concurrent_tasks` | `usize` | `num_cpus::get()` | Maximum number of tasks to run concurrently |
| `default_timeout` | `Duration` | `300s` | Default timeout for tasks without explicit timeout |
| `enable_state_persistence` | `bool` | `false` | Enable execution state persistence for recovery |
| `enable_resource_throttling` | `bool` | `false` | Enable resource monitoring and throttling |
| `resource_limits` | `Option<ResourceLimits>` | `None` | Resource limits for throttling (required if throttling enabled) |
| `enable_metrics_collection` | `bool` | `true` | Enable detailed metrics collection |
| `metrics_export_format` | `MetricsFormat` | `Json` | Format for metrics export |
| `log_level` | `LogLevel` | `Info` | Logging level for executor operations |
| `working_directory` | `Option<PathBuf>` | `None` | Base working directory for task execution |

### Example

```rust
use jorm::executor::{ExecutorConfig, ResourceLimits, MetricsFormat, LogLevel};
use std::time::Duration;
use std::path::PathBuf;

let config = ExecutorConfig {
    max_concurrent_tasks: 8,
    default_timeout: Duration::from_secs(600),
    enable_state_persistence: true,
    enable_resource_throttling: true,
    resource_limits: Some(ResourceLimits {
        max_cpu_percent: 80.0,
        max_memory_percent: 70.0,
        max_concurrent_tasks: 6,
    }),
    enable_metrics_collection: true,
    metrics_export_format: MetricsFormat::Json,
    log_level: LogLevel::Info,
    working_directory: Some(PathBuf::from("/opt/jorm/workspace")),
};
```

### Configuration Validation

The executor validates configuration on creation:

- `max_concurrent_tasks` must be > 0
- If `enable_resource_throttling` is true, `resource_limits` must be provided
- `resource_limits.max_concurrent_tasks` should be ≤ `max_concurrent_tasks`
- `default_timeout` must be > 0

## StateConfig

Configuration for execution state persistence and recovery.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `database_url` | `String` | Required | SQLite database URL for state storage |
| `enable_checkpoints` | `bool` | `true` | Enable periodic execution checkpoints |
| `checkpoint_interval` | `Duration` | `30s` | Interval between checkpoints |
| `cleanup_completed_after` | `Option<Duration>` | `7 days` | Auto-cleanup completed executions after duration |
| `max_execution_history` | `Option<usize>` | `1000` | Maximum number of executions to keep in history |

### Example

```rust
use jorm::executor::StateConfig;
use std::time::Duration;

let state_config = StateConfig {
    database_url: "sqlite:/var/lib/jorm/state.db".to_string(),
    enable_checkpoints: true,
    checkpoint_interval: Duration::from_secs(60), // 1 minute
    cleanup_completed_after: Some(Duration::from_secs(7 * 24 * 3600)), // 7 days
    max_execution_history: Some(500),
};
```

### Database Schema

The state database automatically creates these tables:

- `executions`: Main execution records
- `tasks`: Individual task execution records
- `checkpoints`: Execution checkpoints for recovery
- `metadata`: Additional execution metadata

## ResourceLimits

Configuration for resource monitoring and throttling.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_cpu_percent` | `f64` | Required | Maximum CPU usage percentage (0.0-100.0) |
| `max_memory_percent` | `f64` | Required | Maximum memory usage percentage (0.0-100.0) |
| `max_concurrent_tasks` | `usize` | Required | Maximum concurrent tasks under resource pressure |
| `monitoring_interval` | `Duration` | `5s` | Interval for resource usage checks |
| `throttle_threshold` | `f64` | `0.9` | Threshold multiplier for throttling (0.0-1.0) |

### Example

```rust
use jorm::executor::ResourceLimits;
use std::time::Duration;

let limits = ResourceLimits {
    max_cpu_percent: 85.0,
    max_memory_percent: 80.0,
    max_concurrent_tasks: 12,
    monitoring_interval: Duration::from_secs(10),
    throttle_threshold: 0.85, // Start throttling at 85% of limits
};
```

### Resource Monitoring Behavior

- **CPU Monitoring**: Tracks system-wide CPU usage
- **Memory Monitoring**: Tracks system-wide memory usage
- **Task Throttling**: Queues new tasks when limits are exceeded
- **Adaptive Concurrency**: Reduces concurrent tasks under resource pressure

## TaskRetryConfig

Configuration for task retry behavior.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_attempts` | `u32` | `1` | Maximum retry attempts (including initial attempt) |
| `initial_delay` | `Duration` | `1s` | Initial delay before first retry |
| `max_delay` | `Duration` | `300s` | Maximum delay between retries |
| `backoff_strategy` | `BackoffStrategy` | `Fixed` | Strategy for calculating retry delays |
| `retry_on_exit_codes` | `Vec<i32>` | `[]` | Exit codes that trigger retries |
| `retry_on_timeout` | `bool` | `false` | Whether to retry on timeout |

### BackoffStrategy

```rust
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,
    /// Linear increase: delay = initial_delay * attempt
    Linear,
    /// Exponential backoff: delay = initial_delay * multiplier^(attempt-1)
    Exponential { multiplier: f64 },
    /// Custom function for calculating delay
    Custom(fn(u32, Duration) -> Duration),
}
```

### Example

```rust
use jorm::executor::{TaskRetryConfig, BackoffStrategy};
use std::time::Duration;

let retry_config = TaskRetryConfig {
    max_attempts: 3,
    initial_delay: Duration::from_secs(2),
    max_delay: Duration::from_secs(60),
    backoff_strategy: BackoffStrategy::Exponential { multiplier: 2.0 },
    retry_on_exit_codes: vec![1, 2, 130], // Common failure codes
    retry_on_timeout: true,
};
```

## EnvironmentConfig

Configuration for environment variable management.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `inherit_system_env` | `bool` | `true` | Inherit system environment variables |
| `global_env_vars` | `HashMap<String, String>` | `{}` | Global environment variables for all tasks |
| `secure_env_vars` | `HashMap<String, SecureValue>` | `{}` | Secure environment variables (encrypted) |
| `env_var_interpolation` | `bool` | `true` | Enable environment variable interpolation |
| `interpolation_syntax` | `InterpolationSyntax` | `DollarBrace` | Syntax for variable interpolation |

### SecureValue

```rust
pub enum SecureValue {
    /// Plain text value (not recommended for secrets)
    Plain(String),
    /// Base64 encoded value
    Base64(String),
    /// Reference to external secret store
    SecretRef { store: String, key: String },
    /// Environment variable reference
    EnvRef(String),
}
```

### InterpolationSyntax

```rust
pub enum InterpolationSyntax {
    /// ${VAR} or ${VAR:-default}
    DollarBrace,
    /// $VAR
    DollarVar,
    /// {{VAR}} or {{VAR|default}}
    DoubleBrace,
}
```

### Example

```rust
use jorm::executor::{EnvironmentConfig, SecureValue, InterpolationSyntax};
use std::collections::HashMap;

let mut global_env = HashMap::new();
global_env.insert("APP_ENV".to_string(), "production".to_string());
global_env.insert("LOG_LEVEL".to_string(), "info".to_string());

let mut secure_env = HashMap::new();
secure_env.insert("API_KEY".to_string(), SecureValue::EnvRef("PROD_API_KEY".to_string()));
secure_env.insert("DB_PASSWORD".to_string(), SecureValue::SecretRef {
    store: "vault".to_string(),
    key: "db/prod/password".to_string(),
});

let env_config = EnvironmentConfig {
    inherit_system_env: true,
    global_env_vars: global_env,
    secure_env_vars: secure_env,
    env_var_interpolation: true,
    interpolation_syntax: InterpolationSyntax::DollarBrace,
};
```

## Configuration Profiles

JORM supports configuration profiles for different environments.

### Profile Structure

```rust
use jorm::executor::{ConfigProfile, ExecutorConfig};

pub struct ConfigProfile {
    pub name: String,
    pub description: String,
    pub executor_config: ExecutorConfig,
    pub state_config: Option<StateConfig>,
    pub environment_config: Option<EnvironmentConfig>,
}
```

### Built-in Profiles

#### Development Profile

```rust
let dev_profile = ConfigProfile {
    name: "development".to_string(),
    description: "Development environment configuration".to_string(),
    executor_config: ExecutorConfig {
        max_concurrent_tasks: 4,
        default_timeout: Duration::from_secs(300),
        enable_state_persistence: false,
        enable_resource_throttling: false,
        resource_limits: None,
        log_level: LogLevel::Debug,
        ..Default::default()
    },
    state_config: None,
    environment_config: Some(EnvironmentConfig {
        inherit_system_env: true,
        global_env_vars: {
            let mut env = HashMap::new();
            env.insert("APP_ENV".to_string(), "development".to_string());
            env.insert("DEBUG".to_string(), "true".to_string());
            env
        },
        ..Default::default()
    }),
};
```

#### Production Profile

```rust
let prod_profile = ConfigProfile {
    name: "production".to_string(),
    description: "Production environment configuration".to_string(),
    executor_config: ExecutorConfig {
        max_concurrent_tasks: 16,
        default_timeout: Duration::from_secs(1800),
        enable_state_persistence: true,
        enable_resource_throttling: true,
        resource_limits: Some(ResourceLimits {
            max_cpu_percent: 85.0,
            max_memory_percent: 80.0,
            max_concurrent_tasks: 12,
            monitoring_interval: Duration::from_secs(10),
            throttle_threshold: 0.9,
        }),
        log_level: LogLevel::Info,
        ..Default::default()
    },
    state_config: Some(StateConfig {
        database_url: "sqlite:/var/lib/jorm/prod_state.db".to_string(),
        enable_checkpoints: true,
        checkpoint_interval: Duration::from_secs(30),
        cleanup_completed_after: Some(Duration::from_secs(30 * 24 * 3600)), // 30 days
        max_execution_history: Some(10000),
    }),
    environment_config: Some(EnvironmentConfig {
        inherit_system_env: false, // Explicit environment in production
        global_env_vars: {
            let mut env = HashMap::new();
            env.insert("APP_ENV".to_string(), "production".to_string());
            env.insert("LOG_LEVEL".to_string(), "info".to_string());
            env
        },
        ..Default::default()
    }),
};
```

### Using Profiles

```rust
use jorm::executor::{ConfigManager, NativeExecutor};

// Load configuration profile
let config_manager = ConfigManager::new();
let profile = config_manager.load_profile("production")?;

// Create executor from profile
let executor = if let Some(state_config) = profile.state_config {
    NativeExecutor::with_state_management(profile.executor_config, state_config).await?
} else {
    NativeExecutor::new(profile.executor_config)
};
```

## Environment Variables

JORM recognizes these environment variables for configuration:

### Executor Configuration

| Variable | Type | Description |
|----------|------|-------------|
| `JORM_MAX_CONCURRENT_TASKS` | `usize` | Override max concurrent tasks |
| `JORM_DEFAULT_TIMEOUT` | `u64` | Default timeout in seconds |
| `JORM_ENABLE_STATE_PERSISTENCE` | `bool` | Enable state persistence |
| `JORM_ENABLE_RESOURCE_THROTTLING` | `bool` | Enable resource throttling |
| `JORM_LOG_LEVEL` | `string` | Log level (trace, debug, info, warn, error) |
| `JORM_WORKING_DIRECTORY` | `string` | Base working directory |

### State Configuration

| Variable | Type | Description |
|----------|------|-------------|
| `JORM_STATE_DATABASE_URL` | `string` | State database URL |
| `JORM_CHECKPOINT_INTERVAL` | `u64` | Checkpoint interval in seconds |
| `JORM_CLEANUP_COMPLETED_AFTER` | `u64` | Cleanup completed executions after seconds |

### Resource Limits

| Variable | Type | Description |
|----------|------|-------------|
| `JORM_MAX_CPU_PERCENT` | `f64` | Maximum CPU usage percentage |
| `JORM_MAX_MEMORY_PERCENT` | `f64` | Maximum memory usage percentage |
| `JORM_RESOURCE_MAX_CONCURRENT_TASKS` | `usize` | Max concurrent tasks under resource pressure |

### Example Environment Setup

```bash
# Development environment
export JORM_MAX_CONCURRENT_TASKS=4
export JORM_DEFAULT_TIMEOUT=300
export JORM_ENABLE_STATE_PERSISTENCE=false
export JORM_LOG_LEVEL=debug

# Production environment
export JORM_MAX_CONCURRENT_TASKS=16
export JORM_DEFAULT_TIMEOUT=1800
export JORM_ENABLE_STATE_PERSISTENCE=true
export JORM_ENABLE_RESOURCE_THROTTLING=true
export JORM_MAX_CPU_PERCENT=85.0
export JORM_MAX_MEMORY_PERCENT=80.0
export JORM_STATE_DATABASE_URL="sqlite:/var/lib/jorm/state.db"
export JORM_LOG_LEVEL=info
```

## Configuration Files

JORM supports configuration files in TOML, JSON, and YAML formats.

### TOML Configuration

```toml
# jorm.toml
[executor]
max_concurrent_tasks = 8
default_timeout = "10m"
enable_state_persistence = true
enable_resource_throttling = true
log_level = "info"

[executor.resource_limits]
max_cpu_percent = 80.0
max_memory_percent = 70.0
max_concurrent_tasks = 6
monitoring_interval = "10s"

[state]
database_url = "sqlite:jorm_state.db"
enable_checkpoints = true
checkpoint_interval = "30s"
cleanup_completed_after = "7d"
max_execution_history = 1000

[environment]
inherit_system_env = true
env_var_interpolation = true

[environment.global_env_vars]
APP_ENV = "production"
LOG_LEVEL = "info"
```

### JSON Configuration

```json
{
  "executor": {
    "max_concurrent_tasks": 8,
    "default_timeout": "10m",
    "enable_state_persistence": true,
    "enable_resource_throttling": true,
    "log_level": "info",
    "resource_limits": {
      "max_cpu_percent": 80.0,
      "max_memory_percent": 70.0,
      "max_concurrent_tasks": 6,
      "monitoring_interval": "10s"
    }
  },
  "state": {
    "database_url": "sqlite:jorm_state.db",
    "enable_checkpoints": true,
    "checkpoint_interval": "30s",
    "cleanup_completed_after": "7d",
    "max_execution_history": 1000
  },
  "environment": {
    "inherit_system_env": true,
    "env_var_interpolation": true,
    "global_env_vars": {
      "APP_ENV": "production",
      "LOG_LEVEL": "info"
    }
  }
}
```

### YAML Configuration

```yaml
executor:
  max_concurrent_tasks: 8
  default_timeout: 10m
  enable_state_persistence: true
  enable_resource_throttling: true
  log_level: info
  resource_limits:
    max_cpu_percent: 80.0
    max_memory_percent: 70.0
    max_concurrent_tasks: 6
    monitoring_interval: 10s

state:
  database_url: "sqlite:jorm_state.db"
  enable_checkpoints: true
  checkpoint_interval: 30s
  cleanup_completed_after: 7d
  max_execution_history: 1000

environment:
  inherit_system_env: true
  env_var_interpolation: true
  global_env_vars:
    APP_ENV: production
    LOG_LEVEL: info
```

### Loading Configuration Files

```rust
use jorm::executor::ConfigManager;

// Load from file
let config_manager = ConfigManager::new();
let config = config_manager.load_from_file("jorm.toml")?;

// Load with profile override
let config = config_manager.load_from_file_with_profile("jorm.toml", "production")?;

// Load from multiple sources (environment variables override file values)
let config = config_manager
    .load_from_file("jorm.toml")?
    .merge_from_env()?;
```

### Configuration Precedence

Configuration values are applied in this order (later values override earlier ones):

1. Default values
2. Configuration file values
3. Environment variable values
4. Command-line argument values (if applicable)
5. Programmatic configuration values

This allows for flexible configuration management across different deployment scenarios.