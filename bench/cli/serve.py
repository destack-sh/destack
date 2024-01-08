import enum

import typer

from bench.proto.discovery import ExtendedServer
from bench.server.host import ModuleHostMultiplexer
from bench.server.supervisor import GlobalSupervisor

app = typer.Typer(short_help="run the services")


@app.command()
async def supervisor(host: str = None, port: int = None):
    """
    Run the supervisor.
    """
    server = ExtendedServer([GlobalSupervisor(), ModuleHostMultiplexer()])
    await server.start(host=host, port=port)


class WorkerMode(enum.StrEnum):
    NODE = "node"
    PROCESS = "process"


@app.command()
def worker(mode: WorkerMode, host: str = None, port: int = None):
    """
    Run the worker.
    """
    raise NotImplementedError
