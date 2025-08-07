from typing import Dict
from .executor import TaskExecutor

def execute_dag(parsed: Dict):
    """
    Execute the tasks of the DAG respecting dependencies.

    :param parsed: Parsed DAG dictionary
    :type parsed: Dict
    """
    executor = TaskExecutor(parsed)
    executor.run()
