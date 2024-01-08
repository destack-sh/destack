import enum

from grpclib.server import Server
import typer

from bench.server.supervisor import GlobalSupervisor

app = typer.Typer(short_help="run the services")


@app.command()
def supervisor():
    """
    Run the supervisor.
    """
    server = Server([GlobalSupervisor()])
    # nocheckin: "proxy" ModuleHost / start and connect relevant Bench module hosts
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
