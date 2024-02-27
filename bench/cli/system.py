import structlog
import typer

from bench.cli.utils import _async_to_sync_blocking, _check_is_consistent
from bench.language import Bench, Region, User
from bench.language.const import NodeType, UserStatus
from bench.system.client import global_session
from bench.system.resource import create_default_bench, provision_pending_resources

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="check whether the current Bench state is properly migrated")
@_async_to_sync_blocking
async def check(check_db: bool = False):
    await _check_is_consistent(check_db=check_db)


@app.command(help="Create 'bench' and 'system' Benches (owned by 'system' User)")
@_async_to_sync_blocking
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
        # immediately provision resources
        await provision_pending_resources(system_bench, session)
        await provision_pending_resources(bench_bench, session)
        await session.commit()


@app.command(help="Provision any pending resources for a Bench")
@_async_to_sync_blocking
async def provision(bench: str):
    async with global_session() as session:
        bench: Bench = await Bench.descendants(
            NodeType.SERVER, NodeType.STORE, NodeType.DRIVE, NodeType.CACHE
        ).get(slug=bench)
        await provision_pending_resources(bench, session)
        await session.commit()
