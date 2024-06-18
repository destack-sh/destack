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
from bench.system.host import HostRouter
from bench.system.supervisor import Supervisor
from bench.utils.env import ENV, IS_DEV
from bench.utils.oracle import REAL_ORACLE
from bench.utils.utils import get_from_env, get_from_env_maybe
from bench.utils.watch import restart_on_file_changes

app = typer.Typer(short_help="run the services")
logger = structlog.get_logger(__name__)


@app.command()
@async_to_sync_blocking
async def system(
    host: str, port: int, watch: bool = False, no_supervisor: bool = False, skip_check: bool = False
):
    if not skip_check:
        await check_is_consistent(check_db=True)
    start = time_ns()
    logger.info("serve.system", host=host, port=port, env=ENV)
    services: list[ServiceBase] = [HostRouter(oracle=REAL_ORACLE)]
    if not no_supervisor:
        services.append(Supervisor(oracle=REAL_ORACLE))
    server = GrpcServer(handlers=services)
    if IS_DEV and watch:
        _ = asyncio.create_task(restart_on_file_changes())  # noqa: RUF006
    with graceful_exit([server]):
        await server.start(host=host, port=port)
    await server.wait_closed()
    logger.info("serve.system.done", uptime=(time_ns() - start) / 1e9)


@app.command()
@async_to_sync_blocking
async def runtime(host: str, port: int, watch: bool = False, skip_check: bool = False):
    if not skip_check:
        await check_is_consistent(check_db=True)
    start = time_ns()
    logger.info("serve.runtime", host=host, port=port, env=ENV)
    server = Runtime(
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
        oracle=REAL_ORACLE,
    )
    services = [server]
    server = GrpcServer(services)

    if IS_DEV and watch:
        _ = asyncio.create_task(restart_on_file_changes())  # noqa: RUF006
    with graceful_exit([server]):
        await server.start(host=host, port=port)
    await server.wait_closed()
    logger.info("serve.runtime.done", uptime=(time_ns() - start) / 1e9)
