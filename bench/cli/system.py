import structlog
import typer
from more_itertools import first

from bench.cli.utils import async_to_sync_blocking, check_is_consistent
from bench.language import Bench, Region, User
from bench.language.const import RESOURCE_NODE_TYPES, ClientType, NodeType, UserStatus
from bench.system.access import ACCESS_TOKEN_LENGTH
from bench.system.core import global_session
from bench.system.supervisor import create_default_bench
from bench.system.test.test_host import MockHost
from bench.utils.func import generate_access_token

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
        logger.info(
            "system.bootstrap",
            system_user=system_user,
            system_bench=system_bench,
            bench_bench=bench_bench,
        )
        await session.commit()


@app.command(help="provision resources for a Bench")
@async_to_sync_blocking
async def provision(bench_slug: str):
    from bench.system.provisioner import provision

    async with global_session() as session:
        bench = await Bench.select_all().descendants(*RESOURCE_NODE_TYPES).get(slug=bench_slug)
        host = MockHost(session)
        await provision(host, bench, list(bench.resources))
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
                access_token=generate_access_token(ACCESS_TOKEN_LENGTH),
            )

        await session.commit()
