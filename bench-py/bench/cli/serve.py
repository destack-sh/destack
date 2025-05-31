import asyncio
import signal
from collections.abc import Collection, Iterator
from contextlib import contextmanager
from time import time_ns

import structlog
import typer
from fastuuid import UUID

from bench.pb2 import SupervisorClient
from bench.proto import GrpcServer, Network, RealNetwork, ServiceBase
from bench.utils.env import ENV, IS_DEV
from bench.utils.oracle import REAL_ORACLE
from bench.utils.telemetry import capture_exception
from bench.utils.utils import get_from_env, get_from_env_maybe
from bench.utils.watch import restart_on_file_changes

from .utils import async_to_sync

app = typer.Typer(short_help="run the services")
logger = structlog.get_logger(__name__)


@contextmanager
def _guard_server(
    server: GrpcServer,
    *,
    signals: Collection[int] = (signal.SIGINT, signal.SIGTERM),
) -> Iterator[None]:
    """
    Utility context-manager to help properly shutdown server in response to the OS signals.
    Adapted from grpclib.utils.graceful_exit.
    """
    loop = asyncio.get_event_loop()
    signals = set(signals)
    flag: list[bool] = []

    def _stop(sig_num: "signal.Signals") -> None:
        fail = False
        server.close()
        for service in server._services:
            try:
                service.stop()
            except RuntimeError:
                # probably server wasn't started yet
                fail = True
        if fail:
            # using second stage in case of error will ensure that non-closed
            # server wont start later
            _kill(sig_num)

    def _kill(sig_num: "signal.Signals") -> None:
        raise SystemExit(128 + sig_num)

    def _exit_handler(sig_num: "signal.Signals") -> None:
        if flag:
            _kill(sig_num)
        else:
            _stop(sig_num)
            flag.append(True)

    for sig_num in signals:
        loop.add_signal_handler(sig_num, _exit_handler, sig_num)  # type: ignore
    try:
        yield
    finally:
        for sig_num in signals:
            loop.remove_signal_handler(sig_num)


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
        with _guard_server(server):
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
):
    """Serve both the HostRouter and the Supervisor."""
    from bench.system import (
        CELL_REGISTRY,
        DATABASE_REGISTRY,
        HostRouterService,
        SupervisorService,
        get_global_database_from_env,
    )

    global_database = get_global_database_from_env()
    network = RealNetwork()
    host_router = HostRouterService(
        id="host-router",
        network=network,
        oracle=REAL_ORACLE,
        global_database=global_database,
        cell_registry=CELL_REGISTRY,
        database_registry=DATABASE_REGISTRY,
        on_error=capture_exception,
    )
    services: list[ServiceBase] = [host_router]
    supervisor = SupervisorService(
        id="supervisor",
        global_database=global_database,
        network=network,
        oracle=REAL_ORACLE,
        cell_registry=CELL_REGISTRY,
        database_registry=DATABASE_REGISTRY,
        on_error=capture_exception,
    )
    services.append(supervisor)
    await _do_serve(handlers=services, network=network, host=host, port=port, watch=watch)


@app.command()
@async_to_sync
async def supervisor(host: str, port: int, watch: bool = False, no_check: bool = False):
    """Serve the Supervisor."""
    from bench.system import (
        CELL_REGISTRY,
        DATABASE_REGISTRY,
        SupervisorService,
        get_global_database_from_env,
    )

    global_database = get_global_database_from_env()
    network = RealNetwork()
    supervisor = SupervisorService(
        id="supervisor",
        global_database=global_database,
        network=network,
        oracle=REAL_ORACLE,
        cell_registry=CELL_REGISTRY,
        database_registry=DATABASE_REGISTRY,
        on_error=capture_exception,
    )
    await _do_serve(handlers=[supervisor], network=network, host=host, port=port, watch=watch)


@app.command()
@async_to_sync
async def host(host: str, port: int, watch: bool = False, no_check: bool = False):
    """Serve the HostRouter."""
    from bench.system import (
        CELL_REGISTRY,
        DATABASE_REGISTRY,
        HostRouterService,
        get_global_database_from_env,
    )

    global_database = get_global_database_from_env()
    network = RealNetwork()
    host_router = HostRouterService(
        id="host-router",
        global_database=global_database,
        cell_registry=CELL_REGISTRY,
        database_registry=DATABASE_REGISTRY,
        network=network,
        oracle=REAL_ORACLE,
        on_error=capture_exception,
    )
    await _do_serve(handlers=[host_router], network=network, host=host, port=port, watch=watch)


@app.command()
@async_to_sync
async def runtime(host: str, port: int, *, process_id: int = -1, watch: bool = False):
    """Serve the Runtime."""
    from bench.language import ClientType
    from bench.runtime import RuntimeProcess, RuntimeProcessMode, RuntimeService

    logger.info("serve.runtime", host=host, port=port, env=ENV)

    supervisor_url = get_from_env("SUPERVISOR_URL", description="Supervisor URL")
    bench_id = get_from_env("BENCH_ID", typ=UUID, description="Node of current Bench")
    client_type = get_from_env("CLIENT_TYPE", typ=ClientType, description="Type of client")
    client_id = get_from_env("CLIENT_ID", typ=UUID, description="Node id of current client")
    client_access_token = get_from_env("CLIENT_ACCESS_TOKEN", description="Access token for client")
    machine_id = get_from_env_maybe(
        "MACHINE_ID", typ=UUID, description="Node id of current machine"
    )
    max_processs = get_from_env(
        "RUNTIME_PROCESSS", typ=int, default=1, description="Maximum number of runtime processs"
    )
    mode = get_from_env(
        "RUNTIME_PROCESS_MODE",
        typ=RuntimeProcessMode,
        default=RuntimeProcessMode.PROCESS,
        description="How to run runtime processs",
    )
    network = RealNetwork()
    supervisor_client = SupervisorClient(
        await network.get_channel(supervisor_url, source_id="runtime")
    )

    if process_id < 0:
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
            max_processs=max_processs,
            mode=mode,
            on_error=capture_exception,
        )
        await _do_serve(handlers=[runtime], network=network, host=host, port=port, watch=watch)
    else:
        process = RuntimeProcess(
            id=f"runtime-process-{process_id}",
            supervisor=supervisor_client,
            network=network,
            oracle=REAL_ORACLE,
            bench_id=bench_id,
            client_type=client_type,
            client_id=client_id,
            client_access_token=client_access_token,
            machine_id=machine_id,
            mode=mode,
            on_error=capture_exception,
        )
        await _do_serve(handlers=[process], network=network, host=host, port=port, watch=watch)
