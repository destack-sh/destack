import structlog
import typer

from bench.language import (
    BENCH_BENCH_PACKAGE_ID,
    BENCH_ID,
    BENCH_SLUG,
    SYSTEM_ID,
    SYSTEM_SLUG,
    SYSTEM_SYSTEM_PACKAGE_ID,
    Bench,
    Handle,
    Region,
    Session,
    User,
    UserStatus,
)

from .bench import CreateBenchOptions, create_default_bench

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


async def create_system_benches(
    session: Session,
    region: Region,
    *,
    upsert: bool = False,
):
    """Bootstrap the Bench system."""

    system_user = await User.get(where=Bench.property("slug").eq(SYSTEM_SLUG)).execute_one_or_none()
    if system_user is None:
        # system user
        system_user = User(
            name="System",
            slug="system",
            email="system@bench.com",
            region=Region.ZURICH,
            status=UserStatus.CREATING,
            is_staff=True,
        )
        session.create(system_user)
        await session.stage()
        system_user.handle = Handle(slug=SYSTEM_SLUG)
        system_user.add_child(system_user.handle)
        logger.debug("system.bootstrap.create", system_user=system_user)
    else:
        assert upsert, f"{system_user!r} already exists"
        assert system_user.handle, f"{system_user!r} has no handle"
        logger.debug("system.bootstrap.exists", system_user=system_user)

    # builtin benches :Builtins
    system_bench = await Bench.get(where=Bench.property("id").eq(SYSTEM_ID)).execute_one_or_none()
    if system_bench is None:
        system_bench = await create_default_bench(
            handle=system_user.handle,
            owned_by=system_user,
            region=region,
            session=session,
            options=CreateBenchOptions(
                bench_id=SYSTEM_ID, main_package_id=SYSTEM_SYSTEM_PACKAGE_ID
            ),
        )
        logger.debug("system.bootstrap.create", system_bench=system_bench)
    else:
        assert upsert, f"{system_bench!r} already exists"
        assert system_bench.handle, f"{system_bench!r} has no main handle"
        logger.debug("system.bootstrap.exists", system_bench=system_bench)

    bench_bench = await Bench.get(where=Bench.property("id").eq(BENCH_ID)).execute_one_or_none()
    if bench_bench is None:
        bench_bench_handle = Handle(slug=BENCH_SLUG)
        system_user.add_child(bench_bench_handle)
        bench_bench = await create_default_bench(
            handle=bench_bench_handle,
            owned_by=system_user,
            region=region,
            session=session,
            options=CreateBenchOptions(bench_id=BENCH_ID, main_package_id=BENCH_BENCH_PACKAGE_ID),
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
