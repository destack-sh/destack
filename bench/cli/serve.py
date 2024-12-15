import asyncio
from time import time_ns
from uuid import UUID

import structlog
import typer
from grpclib.utils import graceful_exit

from bench.cli.utils import async_to_sync_blocking
from bench.language.const import ClientType
from bench.proto.services import GrpcServer, ServiceBase
from bench.runtime.thread import RuntimeThread
from bench.system.utils.session import regional_store_from_env
from bench.utils.env import ENV, IS_DEV
from bench.utils.oracle import REAL_ORACLE
from bench.utils.utils import get_from_env, get_from_env_maybe
from bench.utils.watch import restart_on_file_changes

app = typer.Typer(short_help="run the services")
logger = structlog.get_logger(__name__)


async def _do_serve(handlers: list[ServiceBase], *, host: str, port: int, watch: bool):
    """Serves the given handlers."""
    logger.info("serve", handlers=handlers, host=host, port=port, env=ENV)
    start = time_ns()
    server = GrpcServer(handlers=handlers, oracle=REAL_ORACLE)
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
    host: str,
    port: int,
    watch: bool = False,
    no_supervisor: bool = False,
):
    from bench.system.host.router import HostRouterService
    from bench.system.supervisor.service import SupervisorService
    from bench.system.utils.session import global_store_from_env, regional_store_from_env
    from bench.system.utils.sharding import host_map_from_env

    global_store = global_store_from_env()
    regional_store = regional_store_from_env()
    host_router = HostRouterService(
        global_store=global_store, regional_store=regional_store, oracle=REAL_ORACLE
    )
    host_map = host_map_from_env()
    services: list[ServiceBase] = [host_router]
    if not no_supervisor:
        supervisor = SupervisorService(
            global_store=global_store, oracle=REAL_ORACLE, host_map=host_map
        )
        services.append(supervisor)
    await _do_serve(handlers=services, host=host, port=port, watch=watch)


@app.command()
@async_to_sync_blocking
async def supervisor(host: str, port: int, watch: bool = False, no_check: bool = False):
    from bench.system.supervisor.service import SupervisorService
    from bench.system.utils.session import global_store_from_env
    from bench.system.utils.sharding import host_map_from_env

    global_store = global_store_from_env()
    host_map = host_map_from_env()
    supervisor = SupervisorService(global_store=global_store, oracle=REAL_ORACLE, host_map=host_map)
    await _do_serve(handlers=[supervisor], host=host, port=port, watch=watch)


@app.command()
@async_to_sync_blocking
async def host(host: str, port: int, watch: bool = False, no_check: bool = False):
    from bench.system.host.router import HostRouterService
    from bench.system.utils.session import global_store_from_env

    global_store = global_store_from_env()
    regional_store = regional_store_from_env()
    host_router = HostRouterService(
        global_store=global_store, regional_store=regional_store, oracle=REAL_ORACLE
    )
    await _do_serve(handlers=[host_router], host=host, port=port, watch=watch)


@app.command()
@async_to_sync_blocking
async def runtime(host: str, port: int, *, thread_id: int = -1, watch: bool = False):
    from bench.runtime.service import RuntimeService, RuntimeThreadMode

    logger.info("serve.runtime", host=host, port=port, env=ENV)

    supervisor_url = get_from_env("SUPERVISOR_URL", description="Supervisor URL")
    bench_id = get_from_env("BENCH_ID", typ=UUID, description="Node of current Bench")
    client_type = get_from_env("CLIENT_TYPE", typ=ClientType, description="Type of client")
    client_id = get_from_env("CLIENT_ID", typ=UUID, description="Node id of current client")
    client_access_token = get_from_env("CLIENT_ACCESS_TOKEN", description="Access token for client")
    server_id = get_from_env_maybe("SERVER_ID", typ=UUID, description="Node id of current server")
    machine_id = get_from_env_maybe(
        "MACHINE_ID", typ=UUID, description="Node id of current machine"
    )
    max_threads = get_from_env(
        "RUNTIME_THREADS", typ=int, default=1, description="Maximum number of runtime threads"
    )
    max_concurrency_per_thread = get_from_env(
        "RUNTIME_CONCURRENCY",
        typ=int,
        default=10,
        description="Maximum number of concurrent runs per runtime thread",
    )
    mode = get_from_env(
        "RUNTIME_THREAD_MODE",
        typ=RuntimeThreadMode,
        default=RuntimeThreadMode.PROCESS,
        description="How to run runtime threads",
    )

    if thread_id < 0:
        runtime = RuntimeService(
            supervisor_url=supervisor_url,
            bench_id=bench_id,
            client_type=client_type,
            client_id=client_id,
            client_access_token=client_access_token,
            server_id=server_id,
            machine_id=machine_id,
            max_threads=max_threads,
            max_concurrency_per_thread=max_concurrency_per_thread,
            oracle=REAL_ORACLE,
            mode=mode,
        )
        await _do_serve(handlers=[runtime], host=host, port=port, watch=watch)
    else:
        thread = RuntimeThread(
            id=int(thread_id),
            supervisor_url=supervisor_url,
            bench_id=bench_id,
            client_type=client_type,
            client_id=client_id,
            client_access_token=client_access_token,
            server_id=server_id,
            machine_id=machine_id,
            oracle=REAL_ORACLE,
            mode=mode,
        )
        await _do_serve(handlers=[thread], host=host, port=port, watch=watch)
