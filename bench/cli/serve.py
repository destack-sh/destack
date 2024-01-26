import asyncio

from grpclib.utils import graceful_exit
import structlog
import typer

from bench.cli.utils import _async_to_sync_blocking, _check_is_consistent
from bench.proto.services import BenchServer
from bench.runtime.node import Worker
from bench.server.host import PackageHostMultiplexer
from bench.server.supervisor import GlobalSupervisor
from bench.utils.monitoring import restart_on_file_changes
from bench.utils.utils import get_from_env, IS_DEBUG

app = typer.Typer(short_help="run the services")
logger = structlog.get_logger(__name__)


@app.command()
@_async_to_sync_blocking
async def server(host: str, port: int, watch: bool = False):
    """
    Run the supervisor.
    """
    await _check_is_consistent(check_db=True)
    logger.info("serve.server", host=host, port=port)
    services = [GlobalSupervisor(), PackageHostMultiplexer()]
    server = BenchServer(services)
    if IS_DEBUG and watch:
        asyncio.create_task(restart_on_file_changes())
    with graceful_exit([server]):
        await server.start(host=host, port=port)
        await server.wait_closed()


@app.command()
@_async_to_sync_blocking
async def worker(host: str, port: int, watch: bool = False):
    """
    Run the worker.
    """
    await _check_is_consistent(check_db=True)
    logger.info("serve.worker", host=host, port=port)
    worker = Worker(
        worker_set_id=get_from_env("WORKER_SET_ID", default=None),
        worker_id=get_from_env("WORKER_ID", default=None),
        bench_id=get_from_env("BENCH_ID", default=None),
        package_id=get_from_env("package_ID", default=None),
    )
    services = [worker]
    server = BenchServer(services)

    if IS_DEBUG and watch:
        asyncio.create_task(restart_on_file_changes())
    with graceful_exit([server]):
        await server.start(host=host, port=port)
        await server.wait_closed()
