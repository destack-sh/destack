import structlog
import typer

from bench.language import (
    BENCH_BENCH_ID,
    BENCH_BENCH_SLUG,
    BENCH_BUILTIN_PACKAGE_ID,
    BENCH_BUILTIN_PACKAGE_SLUG,
    SYSTEM_BENCH_ID,
    SYSTEM_BENCH_SLUG,
    SYSTEM_PACKAGE_ID,
    SYSTEM_PACKAGE_SLUG,
    Engine,
    Region,
    Store,
    User,
    UserStatus,
)
from bench.system.core import global_session
from bench.utils.oracle import REAL_ORACLE

from .bench import CreateBenchOptions, create_default_bench

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


async def create_system_benches(
    region: Region,
    global_store: Store,
    global_pg_engine: Engine,
    regional_store: Store,
    regional_pg_engine: Engine,
):
    """Bootstrap the Bench system."""

    async with global_session(
        global_store, (global_pg_engine, regional_pg_engine), REAL_ORACLE, epoch=0
    ) as session:
        system_user = User(
            name="System",
            slug="system",
            email="system@bench.com",
            region=Region.ZURICH,
            status=UserStatus.REGISTERED,
        )
        session._create(system_user)
        await session.flush(optimistic=True)
        # create builtin benches :Builtins
        system_user.main_handle = system_user.handles.create(slug=SYSTEM_BENCH_SLUG)
        system_bench = await create_default_bench(
            main_handle=system_user.main_handle,
            owned_by=system_user,
            region=region,
            session=session,
            options=CreateBenchOptions(
                create_computer_scaler=False,
                bench_id=SYSTEM_BENCH_ID,
                main_package_slug=SYSTEM_PACKAGE_SLUG,
                main_package_id=SYSTEM_PACKAGE_ID,
            ),
        )
        bench_bench_handle = system_user.handles.create(slug=BENCH_BENCH_SLUG)
        bench_bench = await create_default_bench(
            main_handle=bench_bench_handle,
            owned_by=system_user,
            region=region,
            session=session,
            options=CreateBenchOptions(
                bench_id=BENCH_BENCH_ID,
                main_package_slug=BENCH_BUILTIN_PACKAGE_SLUG,
                main_package_id=BENCH_BUILTIN_PACKAGE_ID,
                main_package_name="Builtin Package",
                create_computer_scaler=False,
            ),
        )
        logger.info(
            "system.bootstrap",
            system_user=system_user,
            system_bench=system_bench,
            bench_bench=bench_bench,
        )
        await session.commit()
