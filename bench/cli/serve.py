import asyncio
from time import time_ns
from uuid import UUID

import structlog
import typer
from grpclib.utils import graceful_exit

from bench.cli.utils import async_to_sync_blocking, check_is_consistent
from bench.language.const import ClientType
from bench.proto.services import GrpcServer, ServiceBase
from bench.runtime.runtime import Runtime
from bench.system.core import system_store_from_env
from bench.system.host import HostRouter
from bench.system.sharding import host_map_from_env
from bench.system.supervisor import Supervisor
from bench.utils.env import ENV, IS_DEV
from bench.utils.oracle import REAL_ORACLE
from bench.utils.utils import get_from_env, get_from_env_maybe
from bench.utils.watch import restart_on_file_changes

app = typer.Typer(short_help="run the services")
logger = structlog.get_logger(__name__)


async def _do_serve(
    handlers: list[ServiceBase], *, host: str, port: int, watch: bool, no_check: bool
):
    """Serves the given handlers."""
    if not no_check:
        await check_is_consistent(check_db=True)
    logger.info("serve", handlers=handlers, host=host, port=port, env=ENV)
    start = time_ns()
    server = GrpcServer(handlers=handlers)
    if IS_DEV and watch:
        _ = asyncio.create_task(restart_on_file_changes())  # noqa: RUF006
    try:
        with graceful_exit([server]):
            await server.start(host=host, port=port)
            await server.wait_closed()
    finally:
        logger.info("serve.exit", uptime=(time_ns() - start) / 1_000_000)


@app.command()
@async_to_sync_blocking
async def system(
    host: str, port: int, watch: bool = False, no_supervisor: bool = False, no_check: bool = False
):
    global_store = system_store_from_env()
    host_router = HostRouter(global_store=global_store, oracle=REAL_ORACLE)
    host_map = host_map_from_env()
    services: list[ServiceBase] = [host_router]
    if not no_supervisor:
        supervisor = Supervisor(global_store=global_store, oracle=REAL_ORACLE, host_map=host_map)
        services.append(supervisor)
    await _do_serve(handlers=services, host=host, port=port, watch=watch, no_check=no_check)


@app.command()
@async_to_sync_blocking
async def supervisor(host: str, port: int, watch: bool = False, no_check: bool = False):
    global_store = system_store_from_env()
    host_map = host_map_from_env()
    supervisor = Supervisor(global_store=global_store, oracle=REAL_ORACLE, host_map=host_map)
    await _do_serve(handlers=[supervisor], host=host, port=port, watch=watch, no_check=no_check)


@app.command()
@async_to_sync_blocking
async def host(host: str, port: int, watch: bool = False, no_check: bool = False):
    global_store = system_store_from_env()
    host_router = HostRouter(global_store=global_store, oracle=REAL_ORACLE)
    await _do_serve(handlers=[host_router], host=host, port=port, watch=watch, no_check=no_check)


@app.command()
@async_to_sync_blocking
async def runtime(host: str, port: int, watch: bool = False, no_check: bool = False):
    logger.info("serve.runtime", host=host, port=port, env=ENV)
    runtime = Runtime(
        supervisor_url=get_from_env("SUPERVISOR_URL", description="URL of the supervisor"),
        bench_id=get_from_env("BENCH_ID", typ=UUID, description="Node of current Bench"),
        client_type=get_from_env("CLIENT_TYPE", typ=ClientType, description="Type of client"),
        client_id=get_from_env("CLIENT_ID", typ=UUID, description="Node id of current client"),
        client_access_token=get_from_env(
            "CLIENT_ACCESS_TOKEN", description="Access token for client"
        ),
        machine_id=get_from_env_maybe(
            "MACHINE_ID", typ=UUID, description="Node id of current machine"
        ),
        max_threads=get_from_env(
            "RUNTIME_THREADS", typ=int, description="Maximum number of runtime threads"
        ),
        oracle=REAL_ORACLE,
    )
    await _do_serve(handlers=[runtime], host=host, port=port, watch=watch, no_check=no_check)
