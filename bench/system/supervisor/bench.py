from typing import NamedTuple

from fastuuid import UUID, uuid4

from bench.language import (
    Bench,
    BenchStatus,
    Database,
    Handle,
    NodeMode,
    Organization,
    Package,
    PackageType,
    Region,
    Session,
    User,
)


class CreateBenchOptions(NamedTuple):
    bench_id: UUID | None = None
    main_package_id: UUID | None = None


async def create_default_bench(
    *,
    bench: Bench | None = None,
    handle: Handle,
    owned_by: User | Organization,
    region: Region,
    session: Session,
    options: CreateBenchOptions,
) -> Bench:
    """Creates a new Bench with all the default stuff."""

    # Bench
    if bench is None:
        bench = Bench(
            id=options.bench_id or uuid4(),
            handle=handle,
            slug=handle.slug,
            name=handle.slug,
            owned_by=owned_by,
            region=region,
        )
    session.create(bench)
    await session.stage()

    # main Package
    main_package = Package(
        parent=bench,
        id=options.main_package_id or uuid4(),
        type=PackageType.HOME,
        name="Home",
        slug="home",
    )
    session.create(main_package)
    await session.stage()
    bench.main_package = main_package

    # main Database
    database = Database(mode=NodeMode.BUILTIN, region=bench.region, name="Database")
    main_package.add_child(database)
    await session.stage()
    bench.database = database
    bench.status = BenchStatus.ACTIVE
    await session.stage()

    return bench
