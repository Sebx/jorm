import re
from typing import Dict, List, Tuple
import os
import yaml


def parse_dag_text(path: str) -> Dict:
    """
    Parse a .txt or .md DAG definition file and return its structure.

    :param path: Path to the DAG file (.txt or .md)
    :type path: str
    :return: Dictionary with keys: name, schedule, tasks, dependencies
    :rtype: Dict
    """
    ext = os.path.splitext(path)[1].lower()
    with open(path, encoding="utf-8") as f:
        content = f.read()
    if ext == ".txt":
        return _parse_txt(content)
    elif ext == ".md":
        return _parse_md(content)
    elif ext in [".yaml", ".yml"]:
        return _parse_yaml(content)
    else:
        raise ValueError(f"Formato de archivo no soportado: {ext}")

def _parse_txt(content: str) -> Dict:
    """
    Parse plain text DAG definition.

    :param content: File content
    :type content: str
    :return: Parsed DAG dictionary
    :rtype: Dict
    """
    lines = content.splitlines()
    name = ""
    schedule = ""
    tasks = []
    dependencies = []
    section = None
    for line in lines:
        l = line.strip()
        if l.startswith("dag:"):
            name = l.split(":", 1)[1].strip()
        elif l.startswith("schedule:"):
            schedule = l.split(":", 1)[1].strip()
        elif l.startswith("tasks:"):
            section = "tasks"
        elif l.startswith("dependencies:"):
            section = "dependencies"
        elif l.startswith("-") and section == "tasks":
            tasks.append(l[1:].strip())
        elif l.startswith("-") and section == "dependencies":
            dep = l[1:].strip()
            m = re.match(r"(.+) after (.+)", dep)
            if m:
                dependencies.append((m.group(1).strip(), m.group(2).strip()))
    return {"name": name, "schedule": schedule, "tasks": tasks, "dependencies": dependencies}

def _parse_md(content: str) -> Dict:
    """
    Parse Markdown DAG definition.

    :param content: File content
    :type content: str
    :return: Parsed DAG dictionary
    :rtype: Dict
    """
    name = ""
    schedule = ""
    tasks = []
    dependencies = []
    lines = content.splitlines()
    section = None
    for line in lines:
        l = line.strip()
        if l.lower().startswith("# dag:"):
            name = l.split(":", 1)[1].strip()
        elif l.lower().startswith("**schedule:**"):
            schedule = l.split("**schedule:**", 1)[1].strip().strip("*")
        elif l.lower().startswith("## tasks"):
            section = "tasks"
        elif l.lower().startswith("## dependencies"):
            section = "dependencies"
        elif l.startswith("-") and section == "tasks":
            task = re.sub(r"[`*]", "", l[1:].strip())
            tasks.append(task)
        elif l.startswith("-") and section == "dependencies":
            dep = re.sub(r"[`*]", "", l[1:].strip())
            m = re.match(r"(.+) runs after (.+)", dep)
            if m:
                dependencies.append((m.group(1).strip(), m.group(2).strip()))
    return {"name": name, "schedule": schedule, "tasks": tasks, "dependencies": dependencies}


def _parse_yaml(content: str) -> Dict:
    """
    Parse YAML DAG definition.

    :param content: File content
    :type content: str
    :return: Parsed DAG dictionary
    :rtype: Dict
    """
    try:
        data = yaml.safe_load(content)
    except yaml.YAMLError as e:
        raise ValueError(f"Invalid YAML format: {e}")
    
    if not isinstance(data, dict):
        raise ValueError("YAML content must be a dictionary")
    
    name = data.get("dag", "")
    schedule = data.get("schedule", "")
    tasks = []
    dependencies = []
    task_configs = {}
    
    # Extract tasks and their configurations
    tasks_data = data.get("tasks", {})
    if isinstance(tasks_data, dict):
        for task_name, task_config in tasks_data.items():
            tasks.append(task_name)
            task_configs[task_name] = task_config
            
            # Extract dependencies from depends_on field
            depends_on = task_config.get("depends_on", [])
            if isinstance(depends_on, str):
                depends_on = [depends_on]
            
            for dependency in depends_on:
                dependencies.append((task_name, dependency))
    
    result = {
        "name": name,
        "schedule": schedule,
        "tasks": tasks,
        "dependencies": dependencies,
        "task_configs": task_configs
    }
    
    return result


def validate_dag(parsed: Dict) -> Tuple[bool, List[str]]:
    """
    Validate a parsed DAG dictionary.

    Checks:
      - Syntax errors
      - Duplicate task names
      - Undefined tasks in dependencies
      - Cyclic dependencies
      - Invalid schedule expressions

    :param parsed: Parsed DAG dictionary
    :type parsed: Dict
    :return: Tuple (is_valid, list of errors)
    :rtype: Tuple[bool, List[str]]
    """
    errors = []
    if not parsed.get("name"):
        errors.append("DAG name is missing.")
    if not parsed.get("tasks"):
        errors.append("No tasks defined.")
    if len(parsed.get("tasks", [])) != len(set(parsed.get("tasks", []))):
        errors.append("Duplicate tasks found.")
    task_set = set(parsed.get("tasks", []))
    for a, b in parsed.get("dependencies", []):
        if a not in task_set:
            errors.append(f"Dependent task not defined: {a}")
        if b not in task_set:
            errors.append(f"Base task not defined: {b}")
    # Detect cycles
    if _has_cycle(parsed.get("tasks", []), parsed.get("dependencies", [])):
        errors.append("Cyclic dependencies detected.")
    # Validate schedule (example: every X minutes)
    sched = parsed.get("schedule", "").lower()
    if sched and not re.match(r"every \d+ (minute|minutes|hour|hours|day|days)", sched):
        errors.append(f"Invalid schedule expression: {sched}")
    return (len(errors) == 0, errors)

def _has_cycle(tasks: List[str], deps: List[Tuple[str, str]]) -> bool:
    """
    Detect cycles in the DAG dependencies.

    :param tasks: List of task names
    :type tasks: List[str]
    :param deps: List of (dependent, base) tuples
    :type deps: List[Tuple[str, str]]
    :return: True if a cycle exists, False otherwise
    :rtype: bool
    """
    from collections import defaultdict
    graph = defaultdict(list)
    for a, b in deps:
        graph[b].append(a)
    visited = set()
    stack = set()
    def visit(node):
        if node in stack:
            return True
        if node in visited:
            return False
        stack.add(node)
        for n in graph[node]:
            if visit(n):
                return True
        stack.remove(node)
        visited.add(node)
        return False
    return any(visit(t) for t in tasks)
