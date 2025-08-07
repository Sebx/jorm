from typing import Dict, List, Tuple
import typer

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
        Run a single task (stub implementation).

        :param task: Task name
        :type task: str
        """
        typer.echo(f"→ Running task: {task}")
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
