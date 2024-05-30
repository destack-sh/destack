import asyncio
from uuid import UUID

import structlog
import typer
from grpclib.utils import graceful_exit

from bench.cli.utils import async_to_sync_blocking, check_is_consistent
from bench.proto.services import BenchServer, BenchServiceBase
from bench.runtime.runtime import Runtime
from bench.system.host import HostRouter
from bench.system.supervisor import Supervisor
from bench.utils.env import ENVIRONMENT, IS_DEBUG
from bench.utils.utils import get_from_env, get_from_env_maybe
from bench.utils.watch import restart_on_file_changes

app = typer.Typer(short_help="run the services")
logger = structlog.get_logger(__name__)


@app.command()
@async_to_sync_blocking
async def system(host: str, port: int, watch: bool = False, no_supervisor: bool = False):
    await check_is_consistent(check_db=True)
    logger.info("serve.system", host=host, port=port, env=ENVIRONMENT)
    services: list[BenchServiceBase] = [HostRouter()]
    if not no_supervisor:
        services.append(Supervisor())
    server = BenchServer(handlers=services)
    if IS_DEBUG and watch:
        _ = asyncio.create_task(restart_on_file_changes())  # noqa: RUF006
    with graceful_exit([server]):
        await server.start(host=host, port=port)
        await server.wait_closed()


@app.command()
@async_to_sync_blocking
async def runtime(host: str, port: int, watch: bool = False):
    await check_is_consistent(check_db=True)
    logger.info("serve.runtime", host=host, port=port, env=ENVIRONMENT)
    server = Runtime(
        supervisor_url=get_from_env("SUPERVISOR_URL"),
        bench_id=get_from_env("BENCH_ID", typ=UUID),
        client_id=get_from_env("CLIENT_ID", typ=UUID),
        client_access_token=get_from_env("CLIENT_ACCESS_TOKEN"),
        machine_id=get_from_env_maybe("MACHINE_ID", typ=UUID),
    )
    services = [server]
    server = BenchServer(services)

    if IS_DEBUG and watch:
        _ = asyncio.create_task(restart_on_file_changes())  # noqa: RUF006
    with graceful_exit([server]):
        await server.start(host=host, port=port)
        await server.wait_closed()
