import structlog
import typer
from more_itertools import first

from bench.cli.utils import async_to_sync_blocking, check_is_consistent
from bench.language import Bench, Region, User
from bench.language.const import ClientType, NodeType, UserStatus
from bench.system.access import generate_access_token
from bench.system.core import DEAD_HOST, global_session
from bench.system.provision import get_provisioners_for, migrate_resources, provision_resources
from bench.system.supervisor import create_default_bench

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="check whether the current Bench state is properly migrated")
@async_to_sync_blocking
async def check(check_db: bool = False):
    await check_is_consistent(check_db=check_db)


@app.command(help="create 'bench' and 'system' Benches (owned by 'system' User)")
@async_to_sync_blocking
async def bootstrap(region: Region = Region.EUROPE_CENTRAL):
    async with global_session() as session:
        system_user = User(
            name="System", slug="system", email="system@bench.com", status=UserStatus.REGISTERED
        )
        session.create(system_user)
        await session.flush()
        system_user.main_handle = system_user.handles.create(slug="system")
        system_bench = await create_default_bench(
            main_handle=system_user.main_handle, owner=system_user, region=region, session=session
        )
        bench_bench_handle = system_user.handles.create(slug="bench")
        bench_bench = await create_default_bench(
            main_handle=bench_bench_handle, owner=system_user, region=region, session=session
        )
        # immediately provision
        provisioners = get_provisioners_for(DEAD_HOST, system_bench)
        await provision_resources(system_bench.resources, provisioners, session)
        await provision_resources(bench_bench.resources, provisioners, session)
        await session.commit()


@app.command(help="provision all esources for a Bench")
@async_to_sync_blocking
async def provision(bench_slug: str):
    async with global_session() as session:
        bench = (
            await Bench.descendants(NodeType.SERVER, NodeType.STORE)
            .select_all()
            .get(slug=bench_slug)
        )
        provisioners = get_provisioners_for(DEAD_HOST, bench)
        await provision_resources(bench.resources, provisioners, session)
        await migrate_resources(bench.resources, provisioners, session)
        await session.commit()


@app.command(help="gets or creates a server Client for a Bench")
@async_to_sync_blocking
async def make_client(bench_slug: str, name: str = "Local Server"):
    async with global_session() as session:
        bench = (
            await Bench.descendants(NodeType.SERVER, NodeType.CLIENT)
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
                access_token=generate_access_token(),
            )

        print(f"=== Client {name} to Bench {bench_slug} ===")
        print(f"Bench ID: {bench.id}")
        print(f"Server ID: {server.id}")
        print(f"Client ID: {client.id}")
        print(f"Client Access Token: {client.access_token}")

        await session.commit()
