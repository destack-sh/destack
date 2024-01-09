import asyncio

from grpclib.reflection.service import ServerReflection
from grpclib.utils import graceful_exit
import typer

from bench.cli.utils import _async_to_sync
from bench.proto.mesh import BenchServer
from bench.runtime.node import Worker
from bench.server.host import ModuleHostMultiplexer
from bench.server.supervisor import GlobalSupervisor
from bench.utils.monitoring import restart_on_file_changes
from bench.utils.utils import get_from_env, DEBUG, LOCAL

app = typer.Typer(short_help="run the services")


@app.command()
@_async_to_sync
async def server(host: str, port: int, watch: bool = False):
    """
    Run the supervisor.
    """
    services = [GlobalSupervisor(), ModuleHostMultiplexer()]
    if DEBUG or LOCAL:
        services = ServerReflection.extend(services)
        if watch:
            asyncio.create_task(restart_on_file_changes())
    server = BenchServer(services)
    with graceful_exit([server]):
        await server.start(host=host, port=port)
        await server.wait_closed()


@app.command()
@_async_to_sync
async def worker(host: str, port: int, watch: bool = False):
    """
    Run the worker.
    """
    worker = Worker(
        worker_set_id=get_from_env("WORKER_SET_ID", default=None),
        worker_id=get_from_env("WORKER_ID", default=None),
        bench_id=get_from_env("BENCH_ID", default=None),
        module_id=get_from_env("MODULE_ID", default=None),
    )
    services = [worker]
    if DEBUG or LOCAL:
        services = ServerReflection.extend(services)
        if watch:
            asyncio.create_task(restart_on_file_changes())
    server = BenchServer(services)
    with graceful_exit([server]):
        await server.start(host=host, port=port)
        await server.wait_closed()
