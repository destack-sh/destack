import structlog
import typer

from bench.language import (
    BENCH_BENCH_PACKAGE_ID,
    BENCH_BENCH_PACKAGE_SLUG,
    BENCH_ID,
    BENCH_SLUG,
    SYSTEM_ID,
    SYSTEM_SLUG,
    SYSTEM_SYSTEM_PACKAGE_ID,
    SYSTEM_SYSTEM_PACKAGE_SLUG,
    Engine,
    NodeReference,
    NodeSuperGraph,
    NodeType,
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

    supergraph = NodeSuperGraph(
        name="System", root_ptr=NodeReference(node_type=NodeType.BENCH, id=SYSTEM_ID)
    )

    async with global_session(
        None, (global_pg_engine, regional_pg_engine), REAL_ORACLE, supergraph=supergraph, epoch=0
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
        system_user.main_handle = system_user.handles.create(slug=SYSTEM_SLUG)
        system_bench = await create_default_bench(
            main_handle=system_user.main_handle,
            owned_by=system_user,
            region=region,
            session=session,
            options=CreateBenchOptions(
                create_computer_scaler=False,
                bench_id=SYSTEM_ID,
                main_package_slug=SYSTEM_SYSTEM_PACKAGE_SLUG,
                main_package_id=SYSTEM_SYSTEM_PACKAGE_ID,
            ),
        )
        bench_bench_handle = system_user.handles.create(slug=BENCH_SLUG)
        bench_bench = await create_default_bench(
            main_handle=bench_bench_handle,
            owned_by=system_user,
            region=region,
            session=session,
            options=CreateBenchOptions(
                bench_id=BENCH_ID,
                main_package_slug=BENCH_BENCH_PACKAGE_SLUG,
                main_package_id=BENCH_BENCH_PACKAGE_ID,
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
