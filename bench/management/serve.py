import enum

import typer

app = typer.Typer()


@app.command()
def supervisor():
    """
    Run the supervisor.
    """
    raise NotImplementedError


class WorkerMode(enum.StrEnum):
    NODE = "node"
    PROCESS = "process"


@app.command()
def worker(mode: WorkerMode):
    """
    Run the worker.
    """
    raise NotImplementedError
