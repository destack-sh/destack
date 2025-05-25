from typing import NamedTuple

from fastuuid import UUID

from bench.language import (
    Bench,
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
    main_package_name: str = "Home"
    main_package_slug: str = "home"
    local_database_name: str = "Local"


async def create_default_bench(  # noqa: RUF029
    *,
    handle: Handle,
    owned_by: User | Organization,
    region: Region,
    session: Session,
    options: CreateBenchOptions,
) -> Bench:
    """Creates a new Bench with all the default stuff."""

    # Bench
    bench = Bench(
        id=options.bench_id or Bench.__id_factory__(),
        handle=handle,
        slug=handle.slug,
        name=handle.slug,
        owned_by=owned_by,
        region=region,
        _is_new=True,
    )
    session.create(bench)
    session.stage()

    # main Package
    main_package = Package(
        parent=bench,
        id=options.main_package_id or Package.__id_factory__(),
        type=PackageType.OPEN,
        name=options.main_package_name,
        slug=options.main_package_slug,
        _is_new=True,
    )
    session.create(main_package)
    session.stage()
    bench.main_package = main_package

    # main Database
    database = Database(
        mode=NodeMode.BUILTIN,
        region=bench.region,
        name=options.local_database_name,
    )
    main_package.add_child(database)
    session.stage()
    bench.database = database
    session.stage()

    return bench
