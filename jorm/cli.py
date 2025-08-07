import sys
import typer
from jorm.parser import parse_dag_text, validate_dag
from jorm.engine import execute_dag


app = typer.Typer()

@app.command("run")
def run_dag(file: str, no_validate: bool = typer.Option(False, "--no-validate", hidden=True)):
    """
    Run a DAG from a file (.txt or .md). Always validates before execution unless --no-validate is set.

    :param file: Path to the DAG file
    :type file: str
    :param no_validate: Skip validation (expert only)
    :type no_validate: bool
    """
    parsed = parse_dag_text(file)
    if not no_validate:
        ok, errors = validate_dag(parsed)
        if not ok:
            typer.secho("❌ Invalid DAG. Errors:", fg="red")
            for e in errors:
                typer.echo(f" - {e}")
            sys.exit(1)
    execute_dag(parsed)

@app.command("exec")
def exec_task(task: str):
    """
    Execute a single task (stub implementation).

    :param task: Task name
    :type task: str
    """
    typer.echo(f"Executing task: {task}")

@app.command("status")
def status():
    """
    Show current execution status (stub).
    """
    typer.echo("Status: (not implemented)")

@app.command("list")
def list_dags():
    """
    List available DAGs (stub).
    """
    typer.echo("Available DAGs: (not implemented)")

@app.command("describe")
def describe(file: str):
    """
    Describe the structure of a DAG.

    :param file: Path to the DAG file
    :type file: str
    """
    parsed = parse_dag_text(file)
    typer.echo(f"DAG: {parsed['name']}")
    typer.echo(f"Schedule: {parsed['schedule']}")
    typer.echo(f"Tasks: {', '.join(parsed['tasks'])}")
    typer.echo("Dependencies:")
    for a, b in parsed['dependencies']:
        typer.echo(f" - {a} after {b}")

@app.command("schedule")
def schedule(file: str):
    """
    Schedule recurring execution of a DAG (stub).

    :param file: Path to the DAG file
    :type file: str
    """
    typer.echo(f"Scheduling execution of {file} (not implemented)")

@app.command("validate")
def validate(file: str):
    """
    Validate the syntax and structure of a DAG.

    :param file: Path to the DAG file
    :type file: str
    """
    parsed = parse_dag_text(file)
    ok, errors = validate_dag(parsed)
    if ok:
        typer.secho("✅ DAG is valid", fg="green")
    else:
        typer.secho("❌ Invalid DAG. Errors:", fg="red")
        for e in errors:
            typer.echo(f" - {e}")
        sys.exit(1)

@app.command("chat")
def chat():
    """
    Semantic chat interface (stub).
    """
    typer.echo("Chat interface (not implemented)")

if __name__ == "__main__":
    app()
    # To enable shell autocomplete:
    #   eval "$(register-python-argcomplete jorm)"
