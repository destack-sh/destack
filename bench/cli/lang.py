import structlog
import typer

from bench.cli.utils import _async_to_sync_blocking, _check_is_consistent
from bench.language import Bench, Package, Region, User
from bench.language.const import UserStatus
from bench.system.resource import create_default_bench
from bench.system.utils import global_session

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="check whether the current Bench state is properly migrated")
@_async_to_sync_blocking
async def check(check_db: bool = False):
    await _check_is_consistent(check_db=check_db)


@app.command(help="IPython shell with a global or Bench-local session")
@_async_to_sync_blocking
async def shell(bench: str = None, package: str = None):
    """Open a Session shell."""
    if bench is not None:
        async with global_session():
            bench: Bench = await Bench.get(slug=bench)
            if package is not None:
                package = await Package.get(parent=bench, slug=package)
    else:
        if package is not None:
            raise ValueError(f"Package {package} must be relative to a Bench")

    logger.info("lang.shell", bench=bench, package=package)
    raise NotImplementedError("TODO :Incomplete: lang.shell")


@app.command(help="Create 'bench' and 'system' Benches (owned by 'system' User)")
@_async_to_sync_blocking
async def bootstrap(region: Region = Region.EU_CENTRAL):
    async with global_session() as session:
        system_user = User(
            name="System", slug="system", email="system@bench.com", status=UserStatus.REGISTERED
        )
        session.create(system_user)
        await session.flush()
        system_user.main_handle = system_user.handles.create(slug="system")
        _ = await create_default_bench(
            main_handle=system_user.main_handle, owner=system_user, region=region, session=session
        )
        bench_bench_handle = system_user.handles.create(slug="bench")
        _ = await create_default_bench(
            main_handle=bench_bench_handle, owner=system_user, region=region, session=session
        )
        await session.commit()
