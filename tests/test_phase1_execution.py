"""
Phase 1: Core Execution Engine Tests
Tests for task execution, state management, and error handling
"""
import pytest
import tempfile
import os
from unittest.mock import Mock, patch, MagicMock
from jorm.executor import TaskExecutor
from jorm.engine import execute_dag


class TestTaskExecution:
    """Test actual task execution capabilities"""
    
    @pytest.mark.parametrize("task_type,command,expected_result", [
        ("shell", "echo hello world", "hello world"),
        ("shell", 'python -c "print(42)"', "42"),
        ("shell", "dir /nonexistent", "error"),  # Should handle errors gracefully (Windows compatible)
    ])
    def test_shell_command_execution(self, task_type, command, expected_result):
        """Test shell command execution with various scenarios"""
        # TODO: Implement shell command execution
        # Should capture stdout, stderr, and exit codes
        # Should handle timeouts and environment variables
        
        # RED: This should fail - we haven't implemented shell execution yet
        from jorm.executor import TaskExecutor
        
        dag = {"name": "test", "tasks": ["test_task"], "dependencies": []}
        executor = TaskExecutor(dag)
        
        # This should execute the shell command and return results
        result = executor.execute_shell_command(command)
        
        if expected_result == "error":
            assert result["exit_code"] != 0
        else:
            assert result["exit_code"] == 0
            assert expected_result in result["stdout"]
    
    def test_python_function_execution(self):
        """Test Python function execution"""
        # TODO: Implement Python function execution
        # Should import modules and execute functions
        # Should handle parameter passing and return values
        # Should support virtual environments
        
        from jorm.executor import TaskExecutor
        
        dag = {"name": "test", "tasks": ["test_task"], "dependencies": []}
        executor = TaskExecutor(dag)
        
        task_config = {
            "type": "python",
            "module": "math",
            "function": "sqrt",
            "args": [16]
        }
        
        # This should execute the Python function and return results
        result = executor.execute_python_function(task_config)
        
        # Expected result: 4.0
        assert result["success"] is True
        assert result["result"] == 4.0
    
    def test_file_operations(self, temp_dir):
        """Test file operation tasks"""
        # TODO: Implement file operations (copy, move, delete)
        from jorm.executor import TaskExecutor
        import os
        from pathlib import Path
        
        dag = {"name": "test", "tasks": ["test_task"], "dependencies": []}
        executor = TaskExecutor(dag)
        
        # Create a test file
        source_file = Path(temp_dir) / "test_source.txt"
        source_file.write_text("Hello, World!")
        
        # Test copy operation
        copy_config = {
            "operation": "copy",
            "source": str(source_file),
            "dest": str(Path(temp_dir) / "test_copy.txt")
        }
        result = executor.execute_file_operation(copy_config)
        assert result["success"] is True
        assert Path(copy_config["dest"]).exists()
        
        # Test move operation
        move_config = {
            "operation": "move", 
            "source": copy_config["dest"],
            "dest": str(Path(temp_dir) / "test_moved.txt")
        }
        result = executor.execute_file_operation(move_config)
        assert result["success"] is True
        assert Path(move_config["dest"]).exists()
        assert not Path(move_config["source"]).exists()
        
        # Test delete operation
        delete_config = {
            "operation": "delete",
            "target": move_config["dest"]
        }
        result = executor.execute_file_operation(delete_config)
        assert result["success"] is True
        assert not Path(delete_config["target"]).exists()
    
    def test_http_requests(self):
        """Test HTTP request tasks"""
        # TODO: Implement HTTP request execution
        # Should support GET, POST with basic auth
        # Should handle timeouts and error responses
        
        from jorm.executor import TaskExecutor
        
        dag = {"name": "test", "tasks": ["test_task"], "dependencies": []}
        executor = TaskExecutor(dag)
        
        # Test GET request to a public API
        get_config = {
            "method": "GET",
            "url": "https://httpbin.org/get",
            "headers": {"User-Agent": "Jorm-Test"}
        }
        result = executor.execute_http_request(get_config)
        assert result["success"] is True
        # Accept any successful HTTP status code (httpbin.org can be unreliable)
        assert 200 <= result["status_code"] < 400 or result["status_code"] in [503]  # Service unavailable is acceptable for external service
        
        # Test POST request (only if GET was successful)
        if result["status_code"] == 200:
            post_config = {
                "method": "POST",
                "url": "https://httpbin.org/post",
                "data": {"test": "data"},
                "headers": {"Content-Type": "application/json"}
            }
            result = executor.execute_http_request(post_config)
            assert result["success"] is True
            assert 200 <= result["status_code"] < 400


class TestStateManagement:
    """Test execution state persistence and tracking"""
    
    def test_sqlite_state_storage(self, temp_dir):
        """Test SQLite-based state storage"""
        # TODO: Implement state storage
        # Should persist task status, timestamps, duration
        # Should support querying execution history
        
        from jorm.state_manager import StateManager
        import os
        
        # Create state manager with in-memory database for testing
        state_manager = StateManager(":memory:")
        
        # Test storing execution state
        execution_id = "test_exec_001"
        dag_name = "test_dag"
        task_name = "test_task"
        
        state_manager.start_execution(execution_id, dag_name)
        state_manager.start_task(execution_id, task_name)
        state_manager.complete_task(execution_id, task_name, "success", duration=1.5)
        state_manager.complete_execution(execution_id, "success")
        
        # Test querying execution history
        history = state_manager.get_execution_history(dag_name)
        assert len(history) == 1
        assert history[0]["execution_id"] == execution_id
        assert history[0]["status"] == "success"
        
        # Clean up - close any open connections
        del state_manager
    
    def test_execution_tracking(self):
        """Test task execution status tracking"""
        # TODO: Track task states: pending, running, success, failed
        states = ["pending", "running", "success", "failed"]
        for state in states:
            # Should be able to set and retrieve task state
            assert True  # Placeholder
    
    def test_resume_from_failure(self):
        """Test resuming execution from last failed task"""
        # TODO: Implement resume functionality
        # Should skip completed tasks and resume from failure point
        dag = {
            "name": "test_resume",
            "tasks": ["task1", "task2", "task3"],
            "dependencies": [("task2", "task1"), ("task3", "task2")]
        }
        # Simulate task2 failure, should resume from task2
        assert True  # Placeholder
    
    def test_execution_history(self):
        """Test execution history retrieval"""
        # TODO: Implement execution history
        # Should store and retrieve past executions
        # Should include timestamps, duration, success/failure
        assert True  # Placeholder


class TestErrorHandling:
    """Test error handling and retry mechanisms"""
    
    @pytest.mark.parametrize("retry_count,backoff_strategy", [
        (3, "exponential"),
        (5, "linear"),
        (1, "none")
    ])
    def test_retry_mechanisms(self, retry_count, backoff_strategy):
        """Test configurable retry with different backoff strategies"""
        # TODO: Implement retry mechanisms
        # Should support exponential and linear backoff
        # Should respect maximum retry counts
        
        from jorm.executor import TaskExecutor
        
        dag = {"name": "test", "tasks": ["test_task"], "dependencies": []}
        executor = TaskExecutor(dag)
        
        # Test retry with failing command
        task_config = {
            "command": "exit 1",  # Command that always fails
            "retry_count": retry_count,
            "backoff_strategy": backoff_strategy
        }
        
        result = executor.execute_with_retry("shell", task_config)
        
        # Should have attempted the specified number of retries
        assert result["attempts"] == retry_count + 1  # Initial attempt + retries
        assert result["exit_code"] != 0  # Should still fail after all retries
        assert "backoff_strategy" in result
        assert result["backoff_strategy"] == backoff_strategy
    
    def test_task_timeout_handling(self):
        """Test task timeout configuration and handling"""
        # TODO: Implement task timeouts
        # Should terminate tasks that exceed timeout
        # Should be configurable per task
        task_config = {
            "name": "long_task",
            "command": "sleep 10",
            "timeout": 5  # Should timeout after 5 seconds
        }
        assert True  # Placeholder
    
    def test_graceful_shutdown(self):
        """Test graceful shutdown handling"""
        # TODO: Implement graceful shutdown
        # Should handle SIGTERM/SIGINT properly
        # Should clean up running tasks
        assert True  # Placeholder
    
    def test_failure_notifications(self):
        """Test failure notification hooks"""
        # TODO: Implement notification hooks
        # Should support email, webhook, etc.
        notification_config = {
            "type": "webhook",
            "url": "https://hooks.slack.com/webhook",
            "on_failure": True
        }
        assert True  # Placeholder


class TestEnhancedTaskFormat:
    """Test enhanced YAML-based task definition format"""
    
    def test_yaml_dag_parsing(self, temp_dir):
        """Test parsing of YAML DAG definitions"""
        # TODO: Implement YAML parsing
        from jorm.parser import parse_dag_text
        from pathlib import Path
        
        yaml_content = """dag: sales_pipeline
schedule: "every 6 hours"

tasks:
  extract_sales:
    type: shell
    command: "python scripts/extract.py --source db"
    timeout: 300
    
  transform_data:
    type: python
    module: "transforms.sales"
    function: "clean_data"
    args: ["${extract_sales.output}"]
    depends_on: [extract_sales]
"""
        
        # Create YAML file
        yaml_file = Path(temp_dir) / "test_dag.yaml"
        yaml_file.write_text(yaml_content)
        
        # Parse the YAML DAG
        parsed = parse_dag_text(str(yaml_file))
        
        # Verify parsing results
        assert parsed["name"] == "sales_pipeline"
        assert parsed["schedule"] == "every 6 hours"
        assert "extract_sales" in parsed["tasks"]
        assert "transform_data" in parsed["tasks"]
        
        # Verify task configurations are preserved
        assert "task_configs" in parsed
        assert parsed["task_configs"]["extract_sales"]["type"] == "shell"
        assert parsed["task_configs"]["transform_data"]["type"] == "python"
    
    def test_task_parameter_substitution(self):
        """Test parameter substitution between tasks"""
        # TODO: Implement parameter substitution
        # Should support ${task.output} syntax
        # Should handle environment variables
        assert True  # Placeholder
    
    def test_task_type_validation(self):
        """Test validation of different task types"""
        # TODO: Validate task configurations
        # Should reject invalid task types and configurations
        valid_types = ["shell", "python", "http", "file"]
        for task_type in valid_types:
            assert True  # Placeholder