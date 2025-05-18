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
    Bench,
    Database,
    Engine,
    Handle,
    NodeReference,
    NodeType,
    Region,
    Supergraph,
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
    global_database: Database,
    global_pg_engine: Engine,
    regional_database: Database,
    regional_pg_engine: Engine,
    *,
    upsert: bool = False,
):
    """Bootstrap the Bench system."""

    supergraph = Supergraph(
        name="System", root_ptr=NodeReference(node_type=NodeType.BENCH, id=SYSTEM_ID)
    )

    async with global_session(
        None, (global_pg_engine, regional_pg_engine), REAL_ORACLE, supergraph=supergraph, epoch=0
    ) as session:
        system_user = (
            await User.where(slug=SYSTEM_SLUG).include_descendants(NodeType.HANDLE).one_or_none()
        )
        if system_user is None:
            # system user
            system_user = User(
                name="System",
                slug="system",
                email="system@bench.com",
                region=Region.ZURICH,
                status=UserStatus.REGISTERED,
                is_staff=True,
            )
            session._create(system_user)
            session.stage()
            system_user.handle = Handle(slug=SYSTEM_SLUG)
            system_user.add_child(system_user.handle)
            logger.debug("system.bootstrap.create", system_user=system_user)
        else:
            assert upsert, f"{system_user!r} already exists"
            assert system_user.handle, f"{system_user!r} has no handle"
            logger.debug("system.bootstrap.exists", system_user=system_user)

        # builtin benches :Builtins
        system_bench = await Bench.where(id=SYSTEM_ID).one_or_none()
        if system_bench is None:
            system_bench = await create_default_bench(
                handle=system_user.handle,
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
            logger.debug("system.bootstrap.create", system_bench=system_bench)
        else:
            assert upsert, f"{system_bench!r} already exists"
            assert system_bench.handle, f"{system_bench!r} has no main handle"
            logger.debug("system.bootstrap.exists", system_bench=system_bench)

        bench_bench = await Bench.where(id=BENCH_ID).one_or_none()
        if bench_bench is None:
            bench_bench_handle = Handle(slug=BENCH_SLUG)
            system_user.add_child(bench_bench_handle)
            bench_bench = await create_default_bench(
                handle=bench_bench_handle,
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
            logger.debug("system.bootstrap.create", bench_bench=bench_bench)
        else:
            assert upsert, f"{bench_bench!r} already exists"
            assert bench_bench.handle, f"{bench_bench!r} has no main handle"
            logger.debug("system.bootstrap.exists", bench_bench=bench_bench)

        logger.info(
            "system.bootstrap",
            system_user=system_user,
            system_bench=system_bench,
            bench_bench=bench_bench,
        )
        await session.commit()
