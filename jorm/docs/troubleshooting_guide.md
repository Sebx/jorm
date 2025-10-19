# Troubleshooting Guide: Native Rust Executor

This guide provides solutions to common issues encountered when using the JORM native Rust executor.

## Table of Contents

- [General Troubleshooting](#general-troubleshooting)
- [Installation Issues](#installation-issues)
- [Configuration Problems](#configuration-problems)
- [Task Execution Failures](#task-execution-failures)
- [Performance Issues](#performance-issues)
- [State Management Problems](#state-management-problems)
- [Resource Monitoring Issues](#resource-monitoring-issues)
- [Migration-Specific Issues](#migration-specific-issues)
- [Debugging Tools](#debugging-tools)
- [Getting Help](#getting-help)

## General Troubleshooting

### Enable Debug Logging

First step in troubleshooting is to enable detailed logging:

```bash
# Enable debug logging
JORM_LOG_LEVEL=debug jorm --native-executor your-dag.txt

# Enable trace logging (very verbose)
JORM_LOG_LEVEL=trace jorm --native-executor your-dag.txt

# Save logs to file
jorm --native-executor your-dag.txt 2>&1 | tee execution.log
```

### Check System Requirements

Verify your system meets the requirements:

```bash
# Check available memory
free -h

# Check CPU cores
nproc

# Check disk space
df -h

# Check system load
uptime
```

### Validate DAG Syntax

Use dry-run mode to validate DAG syntax without execution:

```bash
# Validate DAG syntax
jorm --native-executor --dry-run your-dag.txt

# Validate with detailed output
jorm --native-executor --dry-run --verbose your-dag.txt
```

## Installation Issues

### Issue: Binary Not Found

**Error:**
```
bash: jorm: command not found
```

**Solutions:**

1. **Check Installation Path:**
```bash
# Find jorm binary
which jorm
find /usr -name "jorm" 2>/dev/null

# Add to PATH if needed
export PATH="/usr/local/bin:$PATH"
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bashrc
```

2. **Reinstall Binary:**
```bash
# Download latest release
curl -L https://github.com/your-org/jorm/releases/latest/download/jorm-linux-x64.tar.gz | tar xz
sudo mv jorm /usr/local/bin/
sudo chmod +x /usr/local/bin/jorm
```

3. **Build from Source:**
```bash
# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build JORM
git clone https://github.com/your-org/jorm.git
cd jorm/jorm
cargo build --release
sudo cp target/release/jorm /usr/local/bin/
```

### Issue: Permission Denied

**Error:**
```
Permission denied (os error 13)
```

**Solutions:**

1. **Fix Binary Permissions:**
```bash
sudo chmod +x /usr/local/bin/jorm
```

2. **Fix Directory Permissions:**
```bash
# For state database
sudo mkdir -p /var/lib/jorm
sudo chown $USER:$USER /var/lib/jorm

# For log files
sudo mkdir -p /var/log/jorm
sudo chown $USER:$USER /var/log/jorm
```

### Issue: Library Dependencies

**Error:**
```
error while loading shared libraries: libssl.so.1.1: cannot open shared object file
```

**Solutions:**

1. **Install Dependencies (Ubuntu/Debian):**
```bash
sudo apt update
sudo apt install libssl1.1 libssl-dev
```

2. **Install Dependencies (CentOS/RHEL):**
```bash
sudo yum install openssl-libs openssl-devel
```

3. **Install Dependencies (macOS):**
```bash
brew install openssl
```

## Configuration Problems

### Issue: Configuration File Not Found

**Error:**
```
Configuration file not found: jorm.toml
```

**Solutions:**

1. **Create Default Configuration:**
```bash
# Create configuration directory
mkdir -p ~/.config/jorm

# Create basic configuration
cat > ~/.config/jorm/config.toml << EOF
[executor]
max_concurrent_tasks = 4
default_timeout = "5m"
enable_state_persistence = false
log_level = "info"
EOF
```

2. **Specify Configuration Path:**
```bash
jorm --native-executor --config /path/to/jorm.toml your-dag.txt
```

### Issue: Invalid Configuration Values

**Error:**
```
Configuration error: max_concurrent_tasks must be greater than 0
```

**Solutions:**

1. **Validate Configuration:**
```bash
# Check configuration syntax
jorm --validate-config

# Test with minimal config
cat > test-config.toml << EOF
[executor]
max_concurrent_tasks = 1
default_timeout = "60s"
EOF

jorm --config test-config.toml --native-executor your-dag.txt
```

2. **Common Configuration Fixes:**
```toml
[executor]
# Must be > 0
max_concurrent_tasks = 4

# Must be valid duration format
default_timeout = "300s"  # or "5m" or "0.5h"

# Must be valid log level
log_level = "info"  # trace, debug, info, warn, error

[executor.resource_limits]
# Percentages must be 0.0-100.0
max_cpu_percent = 80.0
max_memory_percent = 70.0
```

### Issue: Environment Variable Conflicts

**Error:**
```
Environment variable JORM_MAX_CONCURRENT_TASKS conflicts with configuration
```

**Solutions:**

1. **Check Environment Variables:**
```bash
# List all JORM environment variables
env | grep JORM

# Unset conflicting variables
unset JORM_MAX_CONCURRENT_TASKS
unset JORM_DEFAULT_TIMEOUT
```

2. **Configuration Precedence:**
```bash
# Order of precedence (highest to lowest):
# 1. Command line arguments
# 2. Environment variables
# 3. Configuration file
# 4. Default values

# Use explicit configuration
jorm --config explicit-config.toml --native-executor your-dag.txt
```

## Task Execution Failures

### Issue: Shell Command Not Found

**Error:**
```
Task 'build' failed: Command 'cargo' not found
```

**Solutions:**

1. **Check PATH Environment:**
```bash
# Debug PATH in task
echo "task_debug: shell echo \$PATH" > debug.txt
jorm --native-executor debug.txt
```

2. **Set Environment Variables:**
```toml
[environment]
inherit_system_env = true
global_env_vars = { PATH = "/usr/local/bin:/usr/bin:/bin:/home/user/.cargo/bin" }
```

3. **Use Full Paths:**
```text
# Instead of:
build: shell cargo build

# Use:
build: shell /home/user/.cargo/bin/cargo build
```

### Issue: Python Script Failures

**Error:**
```
Task 'process' failed: Python interpreter not found
```

**Solutions:**

1. **Specify Python Path:**
```text
process: python_script script.py
  python_path: /usr/bin/python3
```

2. **Check Python Installation:**
```bash
# Find Python interpreters
which python3
which python
ls -la /usr/bin/python*

# Test Python execution
python3 --version
python3 -c "import sys; print(sys.path)"
```

3. **Set Python Environment:**
```text
process: python_script script.py
  env:
    PYTHONPATH: /opt/project/lib
    VIRTUAL_ENV: /opt/venv/project
```

### Issue: HTTP Request Failures

**Error:**
```
Task 'api_call' failed: Connection timeout
```

**Solutions:**

1. **Increase Timeout:**
```text
api_call: http GET https://api.example.com/data
  timeout: 60s
```

2. **Check Network Connectivity:**
```bash
# Test connectivity
curl -v https://api.example.com/data
ping api.example.com
nslookup api.example.com
```

3. **Configure Proxy (if needed):**
```bash
export HTTP_PROXY=http://proxy.company.com:8080
export HTTPS_PROXY=http://proxy.company.com:8080
```

### Issue: File Operation Failures

**Error:**
```
Task 'backup' failed: Permission denied
```

**Solutions:**

1. **Check File Permissions:**
```bash
# Check source file
ls -la source.txt

# Check destination directory
ls -ld /backup/directory/
```

2. **Fix Permissions:**
```bash
# Make directory writable
chmod 755 /backup/directory/

# Change ownership if needed
sudo chown $USER:$USER /backup/directory/
```

3. **Use Absolute Paths:**
```text
# Instead of relative paths:
backup: file copy results.txt backup/

# Use absolute paths:
backup: file copy /opt/project/results.txt /opt/backup/results.txt
```

## Performance Issues

### Issue: Slow Execution

**Symptoms:**
- Native executor slower than expected
- Tasks taking longer than with Python executor

**Diagnosis:**

1. **Enable Performance Monitoring:**
```bash
jorm --native-executor --enable-metrics your-dag.txt
```

2. **Check Resource Usage:**
```bash
# Monitor during execution
top -p $(pgrep jorm)
htop -p $(pgrep jorm)

# Check I/O usage
iotop -p $(pgrep jorm)
```

**Solutions:**

1. **Optimize Concurrency:**
```toml
# For CPU-bound tasks
[executor]
max_concurrent_tasks = 4  # Number of CPU cores

# For I/O-bound tasks
[executor]
max_concurrent_tasks = 16  # 2-4x CPU cores
```

2. **Reduce Resource Monitoring Overhead:**
```toml
[executor]
enable_resource_throttling = false  # Disable if not needed

[executor.resource_limits]
monitoring_interval = "30s"  # Increase interval
```

### Issue: High Memory Usage

**Error:**
```
Task execution failed: Out of memory
```

**Solutions:**

1. **Reduce Concurrency:**
```toml
[executor]
max_concurrent_tasks = 2  # Reduce concurrent tasks

[executor.resource_limits]
max_memory_percent = 60.0  # Lower memory threshold
```

2. **Enable Memory Monitoring:**
```toml
[executor]
enable_resource_throttling = true

[executor.resource_limits]
max_memory_percent = 70.0
monitoring_interval = "5s"
```

3. **Clean Up State Database:**
```toml
[state]
cleanup_completed_after = "1h"  # Clean up more frequently
max_execution_history = 50       # Reduce history size
```

### Issue: CPU Overload

**Symptoms:**
- System becomes unresponsive
- High CPU usage (>90%)

**Solutions:**

1. **Limit CPU Usage:**
```toml
[executor.resource_limits]
max_cpu_percent = 75.0  # Throttle at 75% CPU
max_concurrent_tasks = 4  # Reduce concurrency
```

2. **Use CPU Affinity:**
```bash
# Limit to specific CPU cores
taskset -c 0,1 jorm --native-executor your-dag.txt
```

3. **Nice Priority:**
```bash
# Lower process priority
nice -n 10 jorm --native-executor your-dag.txt
```

## State Management Problems

### Issue: Database Connection Errors

**Error:**
```
State persistence error: Unable to connect to database
```

**Solutions:**

1. **Check Database Path:**
```toml
[state]
# Use absolute path
database_url = "sqlite:/var/lib/jorm/state.db"

# Ensure directory exists
# mkdir -p /var/lib/jorm
```

2. **Fix Database Permissions:**
```bash
# Create directory with correct permissions
sudo mkdir -p /var/lib/jorm
sudo chown $USER:$USER /var/lib/jorm
chmod 755 /var/lib/jorm

# Fix database file permissions
chmod 644 /var/lib/jorm/state.db
```

3. **Use In-Memory Database (Testing):**
```toml
[state]
database_url = "sqlite::memory:"
```

### Issue: Checkpoint Failures

**Error:**
```
Failed to create checkpoint: Database is locked
```

**Solutions:**

1. **Check for Multiple Instances:**
```bash
# Find running JORM processes
ps aux | grep jorm
pgrep -f jorm

# Kill conflicting processes
pkill -f "jorm.*--native-executor"
```

2. **Adjust Checkpoint Settings:**
```toml
[state]
checkpoint_interval = "60s"  # Increase interval
enable_checkpoints = false   # Disable if not needed
```

3. **Use Separate Database:**
```toml
[state]
database_url = "sqlite:/tmp/jorm-state-${USER}.db"
```

### Issue: Recovery Failures

**Error:**
```
Execution recovery failed: Invalid checkpoint data
```

**Solutions:**

1. **Reset State Database:**
```bash
# Backup current database
cp /var/lib/jorm/state.db /var/lib/jorm/state.db.backup

# Remove corrupted database
rm /var/lib/jorm/state.db

# Restart execution (will create new database)
jorm --native-executor your-dag.txt
```

2. **Disable State Persistence:**
```toml
[executor]
enable_state_persistence = false
```

## Resource Monitoring Issues

### Issue: Resource Monitor Not Starting

**Error:**
```
Resource monitoring failed to start: Permission denied
```

**Solutions:**

1. **Check System Permissions:**
```bash
# Test system resource access
cat /proc/meminfo
cat /proc/stat
cat /proc/loadavg
```

2. **Disable Resource Monitoring:**
```toml
[executor]
enable_resource_throttling = false
```

3. **Use Alternative Monitoring:**
```bash
# External monitoring
while true; do
    echo "$(date): CPU: $(top -bn1 | grep "Cpu(s)" | awk '{print $2}'), Memory: $(free | grep Mem | awk '{printf "%.1f%%", $3/$2 * 100.0}')"
    sleep 10
done &
```

### Issue: Incorrect Resource Readings

**Symptoms:**
- Resource usage shows 0% or unrealistic values
- Tasks not being throttled when they should be

**Solutions:**

1. **Verify System Tools:**
```bash
# Check if system monitoring tools work
top -bn1 | head -20
free -h
cat /proc/meminfo | head -10
```

2. **Adjust Monitoring Settings:**
```toml
[executor.resource_limits]
monitoring_interval = "10s"  # Increase monitoring frequency
throttle_threshold = 0.8     # Lower threshold
```

## Migration-Specific Issues

### Issue: Output Format Differences

**Problem:**
- Scripts expecting Python executor output format fail

**Solutions:**

1. **Use Compatibility Mode:**
```bash
jorm --native-executor --python-compatible-output your-dag.txt
```

2. **Update Scripts:**
```bash
# Old format parsing
grep "Task completed:" output.log

# New format parsing
grep "✅.*completed successfully" output.log
```

### Issue: Environment Variable Differences

**Problem:**
- Tasks fail due to different environment variable handling

**Solutions:**

1. **Explicit Environment Configuration:**
```toml
[environment]
inherit_system_env = true
global_env_vars = {
    PATH = "/usr/local/bin:/usr/bin:/bin",
    PYTHONPATH = "/opt/project/lib",
    HOME = "/home/user"
}
```

2. **Debug Environment:**
```text
debug_env: shell env | sort > /tmp/jorm-env.txt
```

### Issue: Timing Differences

**Problem:**
- Race conditions appear due to faster execution

**Solutions:**

1. **Add Explicit Dependencies:**
```text
# Ensure proper ordering
setup: shell mkdir -p /tmp/workspace
process: shell process_data.sh
cleanup: shell rm -rf /tmp/workspace

process after setup
cleanup after process
```

2. **Add Delays if Needed:**
```text
wait_task: shell sleep 5
```

## Debugging Tools

### Built-in Debugging

```bash
# Dry run mode
jorm --native-executor --dry-run your-dag.txt

# Verbose output
jorm --native-executor --verbose your-dag.txt

# Debug logging
JORM_LOG_LEVEL=debug jorm --native-executor your-dag.txt

# Trace logging (very detailed)
JORM_LOG_LEVEL=trace jorm --native-executor your-dag.txt
```

### External Debugging Tools

```bash
# System monitoring
htop
iotop
nethogs

# Process tracing
strace -p $(pgrep jorm)
ltrace -p $(pgrep jorm)

# Memory debugging
valgrind --tool=memcheck jorm --native-executor your-dag.txt
```

### Log Analysis

```bash
# Extract errors
grep -E "(ERROR|FAIL)" /var/log/jorm/*.log

# Extract warnings
grep "WARN" /var/log/jorm/*.log

# Task execution timeline
grep -E "(Starting|completed)" /var/log/jorm/*.log | sort

# Performance metrics
grep "duration" /var/log/jorm/*.log
```

### Creating Debug DAGs

```text
# debug-system.txt
DAG: system_debug

check_env: shell env | sort
check_path: shell echo $PATH
check_python: shell which python3 && python3 --version
check_disk: shell df -h
check_memory: shell free -h
check_cpu: shell nproc && cat /proc/loadavg
check_network: shell ping -c 3 8.8.8.8

# Run all checks in parallel (no dependencies)
```

## Getting Help

### Community Resources

1. **Documentation:**
   - [API Examples](api_examples.md)
   - [Configuration Reference](configuration_reference.md)
   - [Migration Guide](migration_guide.md)

2. **Community Forums:**
   - [JORM Community](https://community.jorm.dev)
   - [GitHub Discussions](https://github.com/your-org/jorm/discussions)

3. **Issue Reporting:**
   - [GitHub Issues](https://github.com/your-org/jorm/issues)

### Preparing Bug Reports

When reporting issues, include:

1. **System Information:**
```bash
# System details
uname -a
cat /etc/os-release

# JORM version
jorm --version

# Configuration
cat ~/.config/jorm/config.toml
```

2. **Reproduction Steps:**
```bash
# Minimal DAG that reproduces the issue
cat > minimal-repro.txt << EOF
DAG: bug_reproduction

failing_task: shell echo "This task fails"
EOF

# Command that triggers the issue
jorm --native-executor minimal-repro.txt
```

3. **Logs:**
```bash
# Capture detailed logs
JORM_LOG_LEVEL=debug jorm --native-executor minimal-repro.txt 2>&1 | tee bug-report.log
```

4. **Environment:**
```bash
# Environment variables
env | grep JORM

# Resource usage
free -h
df -h
```

### Emergency Support

For critical production issues:

1. **Immediate Rollback:**
```bash
# Switch back to Python executor
export JORM_USE_PYTHON_EXECUTOR=true
jorm your-critical-dag.txt
```

2. **Contact Support:**
   - Email: support@jorm.dev
   - Slack: #jorm-support
   - Phone: +1-555-JORM-HELP (for enterprise customers)

Remember to include your support contract number and severity level when contacting emergency support.