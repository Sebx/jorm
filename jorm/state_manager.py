"""
State Management for Jorm DAG Execution
Handles persistence of execution state using SQLite
"""
import sqlite3
import time
from datetime import datetime
from typing import Dict, List, Optional
from pathlib import Path


class StateManager:
    """
    Manages execution state persistence using SQLite database.
    """
    
    def __init__(self, db_path: str = "jorm_state.db"):
        """
        Initialize the StateManager with database path.
        
        :param db_path: Path to SQLite database file
        :type db_path: str
        """
        self.db_path = db_path
        self.conn = sqlite3.connect(db_path, check_same_thread=False)
        self._init_database()
    
    def __del__(self):
        """Close database connection on cleanup."""
        if hasattr(self, 'conn'):
            self.conn.close()
    
    def _init_database(self):
        """Initialize the database schema."""
        self.conn.execute("""
                CREATE TABLE IF NOT EXISTS dag_executions (
                    execution_id TEXT PRIMARY KEY,
                    dag_name TEXT NOT NULL,
                    status TEXT NOT NULL,
                    start_time TIMESTAMP,
                    end_time TIMESTAMP,
                    duration REAL,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )
        """)
        
        self.conn.execute("""
                CREATE TABLE IF NOT EXISTS task_executions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    execution_id TEXT NOT NULL,
                    task_name TEXT NOT NULL,
                    status TEXT NOT NULL,
                    start_time TIMESTAMP,
                    end_time TIMESTAMP,
                    duration REAL,
                    output TEXT,
                    error_message TEXT,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (execution_id) REFERENCES dag_executions (execution_id)
                )
        """)
        
        self.conn.commit()
    
    def start_execution(self, execution_id: str, dag_name: str) -> None:
        """
        Start a new DAG execution.
        
        :param execution_id: Unique execution identifier
        :type execution_id: str
        :param dag_name: Name of the DAG being executed
        :type dag_name: str
        """
        self.conn.execute("""
            INSERT INTO dag_executions (execution_id, dag_name, status, start_time)
            VALUES (?, ?, ?, ?)
        """, (execution_id, dag_name, "running", datetime.now()))
        self.conn.commit()
    
    def complete_execution(self, execution_id: str, status: str) -> None:
        """
        Complete a DAG execution.
        
        :param execution_id: Execution identifier
        :type execution_id: str
        :param status: Final execution status (success, failed)
        :type status: str
        """
        # Get start time to calculate duration
        cursor = self.conn.execute("""
            SELECT start_time FROM dag_executions WHERE execution_id = ?
        """, (execution_id,))
        row = cursor.fetchone()
        
        if row:
            start_time = datetime.fromisoformat(row[0])
            duration = (datetime.now() - start_time).total_seconds()
            
            self.conn.execute("""
                UPDATE dag_executions 
                SET status = ?, end_time = ?, duration = ?
                WHERE execution_id = ?
            """, (status, datetime.now(), duration, execution_id))
            self.conn.commit()
    
    def start_task(self, execution_id: str, task_name: str) -> None:
        """
        Start a task execution.
        
        :param execution_id: Execution identifier
        :type execution_id: str
        :param task_name: Name of the task
        :type task_name: str
        """
        self.conn.execute("""
            INSERT INTO task_executions (execution_id, task_name, status, start_time)
            VALUES (?, ?, ?, ?)
        """, (execution_id, task_name, "running", datetime.now()))
        self.conn.commit()
    
    def complete_task(self, execution_id: str, task_name: str, status: str, 
                     duration: Optional[float] = None, output: Optional[str] = None, 
                     error_message: Optional[str] = None) -> None:
        """
        Complete a task execution.
        
        :param execution_id: Execution identifier
        :type execution_id: str
        :param task_name: Name of the task
        :type task_name: str
        :param status: Task status (success, failed)
        :type status: str
        :param duration: Task duration in seconds
        :type duration: Optional[float]
        :param output: Task output
        :type output: Optional[str]
        :param error_message: Error message if failed
        :type error_message: Optional[str]
        """
        self.conn.execute("""
            UPDATE task_executions 
            SET status = ?, end_time = ?, duration = ?, output = ?, error_message = ?
            WHERE execution_id = ? AND task_name = ? AND status = 'running'
        """, (status, datetime.now(), duration, output, error_message, execution_id, task_name))
        self.conn.commit()
    
    def get_execution_history(self, dag_name: str, limit: int = 10) -> List[Dict]:
        """
        Get execution history for a DAG.
        
        :param dag_name: Name of the DAG
        :type dag_name: str
        :param limit: Maximum number of executions to return
        :type limit: int
        :return: List of execution records
        :rtype: List[Dict]
        """
        self.conn.row_factory = sqlite3.Row
        cursor = self.conn.execute("""
            SELECT * FROM dag_executions 
            WHERE dag_name = ? 
            ORDER BY start_time DESC 
            LIMIT ?
        """, (dag_name, limit))
        
        return [dict(row) for row in cursor.fetchall()]
    
    def get_task_history(self, execution_id: str) -> List[Dict]:
        """
        Get task execution history for a specific execution.
        
        :param execution_id: Execution identifier
        :type execution_id: str
        :return: List of task execution records
        :rtype: List[Dict]
        """
        self.conn.row_factory = sqlite3.Row
        cursor = self.conn.execute("""
            SELECT * FROM task_executions 
            WHERE execution_id = ? 
            ORDER BY start_time ASC
        """, (execution_id,))
        
        return [dict(row) for row in cursor.fetchall()]
    
    def get_execution_status(self, execution_id: str) -> Optional[Dict]:
        """
        Get current status of an execution.
        
        :param execution_id: Execution identifier
        :type execution_id: str
        :return: Execution status record or None
        :rtype: Optional[Dict]
        """
        self.conn.row_factory = sqlite3.Row
        cursor = self.conn.execute("""
            SELECT * FROM dag_executions WHERE execution_id = ?
        """, (execution_id,))
        
        row = cursor.fetchone()
        return dict(row) if row else None