"""
Pytest configuration and shared fixtures for Jorm test suite
"""
import pytest
import tempfile
import os
import shutil
from unittest.mock import Mock, MagicMock
import sqlite3
from pathlib import Path


@pytest.fixture
def temp_dir():
    """Create a temporary directory for test files"""
    temp_dir = tempfile.mkdtemp()
    yield temp_dir
    shutil.rmtree(temp_dir)


@pytest.fixture
def sample_dag_txt():
    """Sample DAG in text format"""
    return """
dag: test_pipeline
schedule: every 30 minutes

tasks:
  - extract_data
  - validate_data
  - transform_data
  - load_data

dependencies:
  - validate_data after extract_data
  - transform_data after validate_data
  - load_data after transform_data
"""


@pytest.fixture
def sample_dag_md():
    """Sample DAG in markdown format"""
    return """
# DAG: test_pipeline
**Schedule:** every 30 minutes

## Tasks
- `extract_data`
- `validate_data`
- `transform_data`
- `load_data`

## Dependencies
- `validate_data` runs after `extract_data`
- `transform_data` runs after `validate_data`
- `load_data` runs after `transform_data`
"""


@pytest.fixture
def sample_dag_yaml():
    """Sample DAG in enhanced YAML format"""
    return """
dag: test_pipeline
schedule: "0 */6 * * *"

env:
  DATABASE_URL: "${DB_URL}"
  OUTPUT_PATH: "/data/processed"

tasks:
  extract_data:
    type: shell
    command: "python extract.py --db ${DATABASE_URL}"
    timeout: 300
    outputs: ["raw_data.csv"]
    
  validate_data:
    type: python
    module: "validators.data"
    function: "validate_csv"
    args: ["${extract_data.outputs[0]}"]
    depends_on: [extract_data]
    
  transform_data:
    type: shell
    command: "python transform.py ${validate_data.output}"
    depends_on: [validate_data]
    
  load_data:
    type: http
    method: POST
    url: "https://api.example.com/data"
    data: "${transform_data.output}"
    depends_on: [transform_data]
"""


@pytest.fixture
def complex_dag():
    """Complex DAG with parallel tasks and multiple dependencies"""
    return {
        "name": "complex_pipeline",
        "schedule": "0 2 * * *",
        "tasks": ["A", "B", "C", "D", "E", "F", "G"],
        "dependencies": [
            ("B", "A"), ("C", "A"),  # B and C depend on A
            ("D", "B"), ("E", "C"),  # D depends on B, E depends on C
            ("F", "D"), ("F", "E"),  # F depends on both D and E
            ("G", "F")               # G depends on F
        ]
    }


@pytest.fixture
def mock_database():
    """Mock database for testing state management"""
    db_path = ":memory:"
    conn = sqlite3.connect(db_path)
    
    # Create tables for state management
    conn.execute("""
        CREATE TABLE dag_executions (
            id INTEGER PRIMARY KEY,
            dag_name TEXT,
            execution_id TEXT,
            status TEXT,
            start_time TIMESTAMP,
            end_time TIMESTAMP
        )
    """)
    
    conn.execute("""
        CREATE TABLE task_executions (
            id INTEGER PRIMARY KEY,
            execution_id TEXT,
            task_name TEXT,
            status TEXT,
            start_time TIMESTAMP,
            end_time TIMESTAMP,
            output TEXT,
            error_message TEXT
        )
    """)
    
    yield conn
    conn.close()


@pytest.fixture
def mock_language_model():
    """Mock language model for AI testing"""
    mock_model = MagicMock()
    
    # Mock responses for different AI operations
    mock_model.generate_response.return_value = "Generated response"
    mock_model.analyze_dag.return_value = {
        "bottlenecks": ["slow_task"],
        "optimizations": ["parallelize task_a and task_b"],
        "issues": []
    }
    mock_model.suggest_improvements.return_value = [
        "Consider adding retry logic to network tasks",
        "Task 'process_data' could be parallelized"
    ]
    mock_model.explain_error.return_value = "This error occurs when the input file is not found. Check the file path and permissions."
    
    return mock_model


@pytest.fixture
def mock_task_executor():
    """Mock task executor for testing execution logic"""
    executor = MagicMock()
    executor.execute_shell_command.return_value = {
        "stdout": "Command executed successfully",
        "stderr": "",
        "exit_code": 0,
        "duration": 1.5
    }
    executor.execute_python_function.return_value = {
        "result": {"processed_records": 100},
        "duration": 2.3
    }
    executor.execute_http_request.return_value = {
        "status_code": 200,
        "response": {"success": True},
        "duration": 0.8
    }
    return executor


@pytest.fixture
def mock_scheduler():
    """Mock scheduler for testing scheduling logic"""
    scheduler = MagicMock()
    scheduler.parse_cron.return_value = True
    scheduler.get_next_run_time.return_value = "2024-01-01 12:00:00"
    scheduler.is_due.return_value = False
    return scheduler


@pytest.fixture
def mock_cloud_storage():
    """Mock cloud storage for testing integrations"""
    storage = MagicMock()
    storage.upload_file.return_value = True
    storage.download_file.return_value = True
    storage.list_files.return_value = ["file1.txt", "file2.csv"]
    storage.delete_file.return_value = True
    return storage


@pytest.fixture
def mock_secrets_manager():
    """Mock secrets manager for testing secret management"""
    secrets = MagicMock()
    secrets.get_secret.return_value = "secret_value"
    secrets.set_secret.return_value = True
    secrets.delete_secret.return_value = True
    secrets.list_secrets.return_value = ["db_password", "api_key"]
    return secrets


@pytest.fixture
def sample_execution_history():
    """Sample execution history for testing analytics"""
    return [
        {
            "execution_id": "exec_001",
            "dag_name": "test_pipeline",
            "start_time": "2024-01-01 10:00:00",
            "end_time": "2024-01-01 10:05:00",
            "status": "success",
            "tasks": [
                {"name": "extract_data", "duration": 60, "status": "success"},
                {"name": "validate_data", "duration": 30, "status": "success"},
                {"name": "transform_data", "duration": 120, "status": "success"},
                {"name": "load_data", "duration": 90, "status": "success"}
            ]
        },
        {
            "execution_id": "exec_002",
            "dag_name": "test_pipeline",
            "start_time": "2024-01-01 10:30:00",
            "end_time": "2024-01-01 10:32:00",
            "status": "failed",
            "tasks": [
                {"name": "extract_data", "duration": 65, "status": "success"},
                {"name": "validate_data", "duration": 35, "status": "failed", "error": "Data validation failed"}
            ]
        }
    ]


@pytest.fixture
def mock_metrics_collector():
    """Mock metrics collector for testing monitoring"""
    collector = MagicMock()
    collector.record_metric.return_value = True
    collector.get_metrics.return_value = {
        "task_duration_avg": 75.5,
        "success_rate": 0.85,
        "error_rate": 0.15
    }
    return collector


@pytest.fixture
def sample_dag_files(temp_dir):
    """Create sample DAG files for testing"""
    dag_dir = Path(temp_dir) / "dags"
    dag_dir.mkdir()
    
    # Create sample text DAG
    txt_dag = dag_dir / "simple.txt"
    txt_dag.write_text("""
dag: simple
schedule: every 10 minutes

tasks:
  - extract_sales
  - transform_data
  - load_to_db

dependencies:
  - transform_data after extract_sales
  - load_to_db after transform_data
""")
    
    # Create sample markdown DAG
    md_dag = dag_dir / "simple.md"
    md_dag.write_text("""
# DAG: simple
**Schedule:** Every 10 minutes

## Tasks
- `extract_sales`
- `transform_data`
- `load_to_db`

## Dependencies
- `transform_data` runs after `extract_sales`
- `load_to_db` runs after `transform_data`
""")
    
    return dag_dir


# Pytest markers for different test categories
def pytest_configure(config):
    """Configure pytest markers"""
    config.addinivalue_line("markers", "unit: Unit tests")
    config.addinivalue_line("markers", "integration: Integration tests")
    config.addinivalue_line("markers", "e2e: End-to-end tests")
    config.addinivalue_line("markers", "slow: Slow running tests")
    config.addinivalue_line("markers", "phase1: Phase 1 features")
    config.addinivalue_line("markers", "phase2: Phase 2 features")
    config.addinivalue_line("markers", "phase3: Phase 3 features")
    config.addinivalue_line("markers", "phase4: Phase 4 features")
    config.addinivalue_line("markers", "phase5: Phase 5 features")
    config.addinivalue_line("markers", "ai: AI-related tests")
    config.addinivalue_line("markers", "security: Security-related tests")
    config.addinivalue_line("markers", "performance: Performance tests")


# Test data constants
TEST_CRON_EXPRESSIONS = [
    "0 */6 * * *",      # Every 6 hours
    "0 0 * * 0",        # Every Sunday at midnight
    "*/15 * * * *",     # Every 15 minutes
    "0 9-17 * * 1-5",   # Weekdays 9-5
    "0 0 1 * *",        # First day of every month
]

TEST_TASK_TYPES = [
    "shell",
    "python",
    "http",
    "file",
    "sql",
    "docker"
]

TEST_ERROR_SCENARIOS = [
    {"type": "FileNotFoundError", "message": "File not found: /path/to/file"},
    {"type": "ConnectionError", "message": "Failed to connect to database"},
    {"type": "TimeoutError", "message": "Task execution timed out"},
    {"type": "ValidationError", "message": "Data validation failed"},
]