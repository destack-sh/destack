import json

import structlog
import typer
from more_itertools import first
from rich import print_json

from bench.cli.utils import async_to_sync_blocking, check_is_consistent
from bench.language import Bench, Region, User
from bench.language.const import (
    ClientType,
    NodeType,
    UserStatus,
)
from bench.system.access import ACCESS_TOKEN_LENGTH
from bench.system.core import (
    global_pg_engine_from_store,
    global_session,
    global_store_from_env,
)
from bench.system.supervisor import create_default_bench
from bench.utils.func import generate_access_token
from bench.utils.oracle import REAL_ORACLE

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="check whether the current Bench state is properly migrated")
@async_to_sync_blocking
async def check(check_db: bool = False):
    await check_is_consistent(check_db=check_db)


@app.command(help="create 'bench' and 'system' Benches (owned by 'system' User)")
@async_to_sync_blocking
async def bootstrap(region: Region = Region.EUROPE_CENTRAL):
    global_store = global_store_from_env()
    global_pg_engine = global_pg_engine_from_store(global_store)
    async with global_session(global_store, (global_pg_engine,), REAL_ORACLE, epoch=0) as session:
        system_user = User(
            name="System", slug="system", email="system@bench.com", status=UserStatus.REGISTERED
        )
        session._create(system_user)
        await session.flush()
        system_user.main_handle = system_user.handles.create(slug="system")
        system_bench = await create_default_bench(
            main_handle=system_user.main_handle,
            owner=system_user,
            region=region,
            global_store=global_store,
            session=session,
        )
        bench_bench_handle = system_user.handles.create(slug="bench")
        bench_bench = await create_default_bench(
            main_handle=bench_bench_handle,
            owner=system_user,
            region=region,
            global_store=global_store,
            session=session,
        )
        logger.info(
            "system.bootstrap",
            system_user=system_user,
            system_bench=system_bench,
            bench_bench=bench_bench,
        )
        await session.commit()


@app.command(name="make-server-client", help="gets or creates a server Client for a Bench")
@async_to_sync_blocking
async def make_server_client(bench_slug: str, name: str = "Localhost"):
    global_store = global_store_from_env()
    global_pg_engine = global_pg_engine_from_store(global_store)
    async with global_session(global_store, (global_pg_engine,), REAL_ORACLE, epoch=0) as session:
        bench = (
            await Bench.descendants(NodeType.SERVER, NodeType.MACHINE, NodeType.CLIENT)
            .select_all()
            .get(slug=bench_slug)
        )
        assert len(bench.servers) == 1, f"{bench!r} has unexpected servers: {bench.servers!r}"
        server = bench.servers[0]
        client = first((c for c in server.clients if c.type == ClientType.BENCH_SERVER), None)
        if client is None:
            client = server.clients.create(
                type=ClientType.BENCH_SERVER,
                name=name,
                access_token=generate_access_token(ACCESS_TOKEN_LENGTH),
                seen_at=REAL_ORACLE.utc(),
            )
        machine = first(server.machines, None)
        if machine is None:
            raise ValueError(f"{server!r} has no machines")

        client_env = {
            "BENCH_ID": str(bench.id),
            "MACHINE_ID": str(machine.id),
            "CLIENT_TYPE": str(int(client.type)),
            "CLIENT_ID": str(client.id),
            "CLIENT_ACCESS_TOKEN": client.access_token,
        }
        print_json(json.dumps(client_env, indent=4))

        await session.commit()
