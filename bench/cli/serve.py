import asyncio
from time import time_ns
from uuid import UUID

import structlog
import typer
from grpclib.utils import graceful_exit

from bench.cli.utils import async_to_sync
from bench.language import ClientType
from bench.pb2.system_grpc import SupervisorClient
from bench.proto import GrpcServer, Network, ServiceBase
from bench.proto.network import RealNetwork
from bench.utils.env import ENV, IS_DEV
from bench.utils.oracle import REAL_ORACLE
from bench.utils.utils import get_from_env, get_from_env_maybe
from bench.utils.watch import restart_on_file_changes

app = typer.Typer(short_help="run the services")
logger = structlog.get_logger(__name__)


async def _do_serve(
    handlers: list[ServiceBase], *, network: Network, host: str, port: int, watch: bool
):
    """Serves the given handlers."""
    logger.info("serve", handlers=handlers, host=host, port=port, env=ENV)
    start = time_ns()
    server = GrpcServer(handlers=handlers, network=network, oracle=REAL_ORACLE)
    if IS_DEV and watch:
        _ = asyncio.create_task(restart_on_file_changes())  # noqa: RUF006
    try:
        with graceful_exit([server]):
            await server.start(host=host, port=port)
            await server.wait_closed()
    finally:
        logger.info("serve.exit", uptime=(time_ns() - start) / 1_000_000)


@app.command()
@async_to_sync
async def system(
    host: str,
    port: int,
    watch: bool = False,
    no_supervisor: bool = False,
):
    from bench.system import (
        HOST_MAP,
        STORE_MAP,
        HostRouterService,
        SupervisorService,
        global_store_from_env,
        regional_store_from_env,
    )

    global_store = global_store_from_env()
    regional_store = regional_store_from_env()
    network = RealNetwork()
    host_router = HostRouterService(
        id="host-router",
        global_store=global_store,
        regional_store=regional_store,
        network=network,
        oracle=REAL_ORACLE,
    )
    services: list[ServiceBase] = [host_router]
    if not no_supervisor:
        supervisor = SupervisorService(
            id="supervisor",
            global_store=global_store,
            network=network,
            oracle=REAL_ORACLE,
            host_map=HOST_MAP,
            store_map=STORE_MAP,
        )
        services.append(supervisor)
    await _do_serve(handlers=services, network=network, host=host, port=port, watch=watch)


@app.command()
@async_to_sync
async def supervisor(host: str, port: int, watch: bool = False, no_check: bool = False):
    from bench.system import HOST_MAP, STORE_MAP, SupervisorService, global_store_from_env

    global_store = global_store_from_env()
    network = RealNetwork()
    supervisor = SupervisorService(
        id="supervisor",
        global_store=global_store,
        network=network,
        oracle=REAL_ORACLE,
        host_map=HOST_MAP,
        store_map=STORE_MAP,
    )
    await _do_serve(handlers=[supervisor], network=network, host=host, port=port, watch=watch)


@app.command()
@async_to_sync
async def host(host: str, port: int, watch: bool = False, no_check: bool = False):
    from bench.system import HostRouterService, global_store_from_env, regional_store_from_env

    global_store = global_store_from_env()
    regional_store = regional_store_from_env()
    network = RealNetwork()
    host_router = HostRouterService(
        id="host-router",
        global_store=global_store,
        regional_store=regional_store,
        network=network,
        oracle=REAL_ORACLE,
    )
    await _do_serve(handlers=[host_router], network=network, host=host, port=port, watch=watch)


@app.command()
@async_to_sync
async def runtime(host: str, port: int, *, thread_id: int = -1, watch: bool = False):
    from bench.runtime import RuntimeService, RuntimeThread, RuntimeThreadMode

    logger.info("serve.runtime", host=host, port=port, env=ENV)

    supervisor_url = get_from_env("SUPERVISOR_URL", description="Supervisor URL")
    bench_id = get_from_env("BENCH_ID", typ=UUID, description="Node of current Bench")
    client_type = get_from_env("CLIENT_TYPE", typ=ClientType, description="Type of client")
    client_id = get_from_env("CLIENT_ID", typ=UUID, description="Node id of current client")
    client_access_token = get_from_env("CLIENT_ACCESS_TOKEN", description="Access token for client")
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
    network = RealNetwork()
    supervisor_client = SupervisorClient(network.get_channel(supervisor_url, source_id="runtime"))

    if thread_id < 0:
        runtime = RuntimeService(
            id="runtime",
            supervisor=supervisor_client,
            network=network,
            oracle=REAL_ORACLE,
            bench_id=bench_id,
            client_type=client_type,
            client_id=client_id,
            client_access_token=client_access_token,
            machine_id=machine_id,
            max_threads=max_threads,
            max_concurrency_per_thread=max_concurrency_per_thread,
            mode=mode,
        )
        await _do_serve(handlers=[runtime], network=network, host=host, port=port, watch=watch)
    else:
        thread = RuntimeThread(
            id=f"runtime-thread-{thread_id}",
            supervisor=supervisor_client,
            network=network,
            oracle=REAL_ORACLE,
            bench_id=bench_id,
            client_type=client_type,
            client_id=client_id,
            client_access_token=client_access_token,
            machine_id=machine_id,
            mode=mode,
        )
        await _do_serve(handlers=[thread], network=network, host=host, port=port, watch=watch)
