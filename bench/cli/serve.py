import asyncio

from grpclib.utils import graceful_exit
import structlog
import typer

from bench.cli.utils import _async_to_sync_blocking, _check_is_consistent
from bench.proto.services import BenchServer
from bench.server.host import BenchHostMultiplexer
from bench.server.supervisor import GlobalSupervisor
from bench.utils.monitoring import restart_on_file_changes
from bench.utils.utils import get_from_env, IS_DEBUG

app = typer.Typer(short_help="run the services")
logger = structlog.get_logger(__name__)


@app.command()
@_async_to_sync_blocking
async def control(host: str, port: int, watch: bool = False):
    await _check_is_consistent(check_db=True)
    logger.info("serve.control", host=host, port=port)
    services = [GlobalSupervisor(), BenchHostMultiplexer()]
    server = BenchServer(services)
    if IS_DEBUG and watch:
        asyncio.create_task(restart_on_file_changes())
    with graceful_exit([server]):
        await server.start(host=host, port=port)
        await server.wait_closed()


@app.command()
@_async_to_sync_blocking
async def user(host: str, port: int, watch: bool = False):
    await _check_is_consistent(check_db=True)
    logger.info("serve.user", host=host, port=port)
    server = Server(
        server_id=get_from_env("SERVER_ID", default=None),
        bench_id=get_from_env("BENCH_ID", default=None),
    )
    services = [server]
    server = BenchServer(services)

    if IS_DEBUG and watch:
        asyncio.create_task(restart_on_file_changes())
    with graceful_exit([server]):
        await server.start(host=host, port=port)
        await server.wait_closed()
