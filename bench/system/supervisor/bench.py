from typing import NamedTuple
from uuid import UUID

from bench.language import (
    Bench,
    Handle,
    Membership,
    NodeMode,
    Organization,
    Package,
    PackageType,
    Region,
    ScalerStrategy,
    ScalerType,
    Session,
    User,
)


class CreateBenchOptions(NamedTuple):
    bench_id: UUID | None = None
    main_package_id: UUID | None = None
    main_package_name: str = "Main"
    main_package_slug: str = "main"
    local_store_name: str = "Local"
    create_computer_scaler: bool = True


async def create_default_bench(
    *,
    main_handle: Handle,
    owned_by: User | Organization,
    region: Region,
    session: Session,
    options: CreateBenchOptions,
) -> Bench:
    """Creates a new Bench with all the default stuff."""

    # Bench
    bench = Bench(
        id=options.bench_id or Bench.__id_factory__(),
        main_handle=main_handle,
        slug=main_handle.slug,
        name=main_handle.slug,
        owned_by=owned_by,
        region=region,
        _is_new=True,
    )
    session._create(bench)
    await session.flush(optimistic=True)

    # main Package
    main_package = Package(
        parent=bench,
        id=options.main_package_id or Package.__id_factory__(),
        type=PackageType.OPEN,
        name=options.main_package_name,
        slug=options.main_package_slug,
        _is_new=True,
    )
    main_package.memberships.append(Membership.new(owned_by, mode=NodeMode.BUILTIN))
    session._create(main_package)
    await session.flush(optimistic=True)
    bench.main_package = main_package

    # main Store
    store = main_package.stores.create(
        mode=NodeMode.BUILTIN,
        region=bench.region,
        name=options.local_store_name,
    )
    await session.flush(optimistic=True)
    bench.main_store = store
    await session.flush(optimistic=True)

    if options.create_computer_scaler:
        _ = main_package.scalers.create(
            type=ScalerType.COMPUTER,
            mode=NodeMode.BUILTIN,
            strategy=ScalerStrategy.AUTO,
            name="Runtime Scaler",
            name_template="Runtime Computer",
            min_count=1,
            target_count=1,
            max_count=4,
        )

    return bench
