from typing import Dict, List, Tuple
import typer
import subprocess
import time
import importlib
import shutil
from pathlib import Path
import requests
import json

class TaskExecutor:
    """
    Executes tasks in a DAG according to their dependencies.
    """
    def __init__(self, dag: Dict):
        """
        Initialize the TaskExecutor.

        :param dag: Parsed DAG dictionary
        :type dag: Dict
        """
        self.dag = dag
        self.tasks = dag.get("tasks", [])
        self.dependencies = dag.get("dependencies", [])
        self.executed = set()

    def run(self):
        """
        Run all tasks in the DAG in topological order.
        """
        typer.secho(f"Running DAG: {self.dag.get('name')}", fg="green")
        for task in self._topological_sort():
            self._run_task(task)

    def _run_task(self, task: str):
        """
        Run a single task with enhanced execution capabilities.

        :param task: Task name or task configuration
        :type task: str or Dict
        """
        typer.echo(f"→ Running task: {task}")
        
        # For now, just mark as executed (will be enhanced for YAML DAGs)
        # TODO: Implement task configuration parsing for different task types
        self.executed.add(task)

    def _topological_sort(self) -> List[str]:
        """
        Return tasks in topological order according to dependencies.

        :return: List of task names in execution order
        :rtype: List[str]
        """
        from collections import defaultdict, deque
        graph = defaultdict(list)
        indegree = {t: 0 for t in self.tasks}
        for a, b in self.dependencies:
            graph[b].append(a)
            indegree[a] += 1
        queue = deque([t for t in self.tasks if indegree[t] == 0])
        order = []
        while queue:
            node = queue.popleft()
            order.append(node)
            for neighbor in graph[node]:
                indegree[neighbor] -= 1
                if indegree[neighbor] == 0:
                    queue.append(neighbor)
        if len(order) != len(self.tasks):
            raise RuntimeError("Cyclic dependencies detected during execution.")
        return order

    def execute_shell_command(self, command: str, timeout: int = 300) -> Dict:
        """
        Execute a shell command and return results.
        
        :param command: Shell command to execute
        :type command: str
        :param timeout: Command timeout in seconds
        :type timeout: int
        :return: Dictionary with stdout, stderr, exit_code, and duration
        :rtype: Dict
        """
        start_time = time.time()
        
        try:
            # Execute the command
            result = subprocess.run(
                command,
                shell=True,
                capture_output=True,
                text=True,
                timeout=timeout
            )
            
            duration = time.time() - start_time
            
            return {
                "stdout": result.stdout.strip(),
                "stderr": result.stderr.strip(),
                "exit_code": result.returncode,
                "duration": duration
            }
            
        except subprocess.TimeoutExpired:
            duration = time.time() - start_time
            return {
                "stdout": "",
                "stderr": f"Command timed out after {timeout} seconds",
                "exit_code": -1,
                "duration": duration
            }
        except Exception as e:
            duration = time.time() - start_time
            return {
                "stdout": "",
                "stderr": str(e),
                "exit_code": -1,
                "duration": duration
            }

    def execute_python_function(self, task_config: Dict) -> Dict:
        """
        Execute a Python function and return results.
        
        :param task_config: Task configuration with module, function, and args
        :type task_config: Dict
        :return: Dictionary with success, result, error, and duration
        :rtype: Dict
        """
        start_time = time.time()
        
        try:
            # Import the module
            module_name = task_config.get("module")
            function_name = task_config.get("function")
            args = task_config.get("args", [])
            kwargs = task_config.get("kwargs", {})
            
            if not module_name or not function_name:
                raise ValueError("Module and function names are required")
            
            # Import the module
            module = importlib.import_module(module_name)
            
            # Get the function
            func = getattr(module, function_name)
            
            # Execute the function
            result = func(*args, **kwargs)
            
            duration = time.time() - start_time
            
            return {
                "success": True,
                "result": result,
                "error": None,
                "duration": duration
            }
            
        except Exception as e:
            duration = time.time() - start_time
            return {
                "success": False,
                "result": None,
                "error": str(e),
                "duration": duration
            }

    def execute_file_operation(self, operation_config: Dict) -> Dict:
        """
        Execute a file operation (copy, move, delete) and return results.
        
        :param operation_config: Operation configuration with operation type and parameters
        :type operation_config: Dict
        :return: Dictionary with success, error, and duration
        :rtype: Dict
        """
        start_time = time.time()
        
        try:
            operation = operation_config.get("operation")
            
            if operation == "copy":
                source = operation_config.get("source")
                dest = operation_config.get("dest")
                
                if not source or not dest:
                    raise ValueError("Source and destination are required for copy operation")
                
                shutil.copy2(source, dest)
                
            elif operation == "move":
                source = operation_config.get("source")
                dest = operation_config.get("dest")
                
                if not source or not dest:
                    raise ValueError("Source and destination are required for move operation")
                
                shutil.move(source, dest)
                
            elif operation == "delete":
                target = operation_config.get("target")
                
                if not target:
                    raise ValueError("Target is required for delete operation")
                
                target_path = Path(target)
                if target_path.is_file():
                    target_path.unlink()
                elif target_path.is_dir():
                    shutil.rmtree(target)
                else:
                    raise FileNotFoundError(f"Target not found: {target}")
                    
            else:
                raise ValueError(f"Unsupported file operation: {operation}")
            
            duration = time.time() - start_time
            
            return {
                "success": True,
                "error": None,
                "duration": duration
            }
            
        except Exception as e:
            duration = time.time() - start_time
            return {
                "success": False,
                "error": str(e),
                "duration": duration
            }

    def execute_http_request(self, request_config: Dict) -> Dict:
        """
        Execute an HTTP request and return results.
        
        :param request_config: Request configuration with method, URL, headers, data
        :type request_config: Dict
        :return: Dictionary with success, status_code, response, error, and duration
        :rtype: Dict
        """
        start_time = time.time()
        
        try:
            method = request_config.get("method", "GET").upper()
            url = request_config.get("url")
            headers = request_config.get("headers", {})
            data = request_config.get("data")
            timeout = request_config.get("timeout", 30)
            
            if not url:
                raise ValueError("URL is required for HTTP request")
            
            # Prepare request arguments
            request_args = {
                "url": url,
                "headers": headers,
                "timeout": timeout
            }
            
            # Handle data for POST/PUT requests
            if data and method in ["POST", "PUT", "PATCH"]:
                if isinstance(data, dict):
                    if headers.get("Content-Type") == "application/json":
                        request_args["json"] = data
                    else:
                        request_args["data"] = data
                else:
                    request_args["data"] = data
            
            # Execute the request
            response = requests.request(method, **request_args)
            
            duration = time.time() - start_time
            
            # Try to parse JSON response, fallback to text
            try:
                response_data = response.json()
            except (json.JSONDecodeError, ValueError):
                response_data = response.text
            
            return {
                "success": True,
                "status_code": response.status_code,
                "response": response_data,
                "headers": dict(response.headers),
                "error": None,
                "duration": duration
            }
            
        except Exception as e:
            duration = time.time() - start_time
            return {
                "success": False,
                "status_code": None,
                "response": None,
                "headers": None,
                "error": str(e),
                "duration": duration
            }

    def execute_with_retry(self, task_type: str, task_config: Dict) -> Dict:
        """
        Execute a task with retry logic and backoff strategies.
        
        :param task_type: Type of task (shell, python, http, file)
        :type task_type: str
        :param task_config: Task configuration including retry settings
        :type task_config: Dict
        :return: Dictionary with execution results and retry information
        :rtype: Dict
        """
        retry_count = task_config.get("retry_count", 0)
        backoff_strategy = task_config.get("backoff_strategy", "exponential")
        base_delay = task_config.get("base_delay", 1.0)
        
        attempts = 0
        last_result = None
        
        for attempt in range(retry_count + 1):  # Initial attempt + retries
            attempts += 1
            
            # Execute the task based on type
            if task_type == "shell":
                command = task_config.get("command")
                result = self.execute_shell_command(command)
                success = result["exit_code"] == 0
            elif task_type == "python":
                result = self.execute_python_function(task_config)
                success = result["success"]
            elif task_type == "http":
                result = self.execute_http_request(task_config)
                success = result["success"] and 200 <= result["status_code"] < 400
            elif task_type == "file":
                result = self.execute_file_operation(task_config)
                success = result["success"]
            else:
                return {
                    "success": False,
                    "error": f"Unsupported task type: {task_type}",
                    "attempts": 1,
                    "backoff_strategy": backoff_strategy
                }
            
            last_result = result
            
            # If successful or no more retries, return result
            if success or attempt == retry_count:
                break
            
            # Calculate delay for next attempt
            if backoff_strategy == "exponential":
                delay = base_delay * (2 ** attempt)
            elif backoff_strategy == "linear":
                delay = base_delay * (attempt + 1)
            else:  # "none" or any other strategy
                delay = base_delay
            
            # Wait before retry (but not after the last attempt)
            if attempt < retry_count:
                time.sleep(delay)
        
        # Add retry information to result
        last_result["attempts"] = attempts
        last_result["backoff_strategy"] = backoff_strategy
        
        return last_result
