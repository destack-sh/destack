from typing import NamedTuple

from bench.language import (
    Bench,
    Handle,
    Organization,
    PackageType,
    Region,
    ScalerStrategy,
    ScalerType,
    Session,
    User,
)


class CreateBenchOptions(NamedTuple):
    create_machine_scaler: bool = True


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
        main_handle=main_handle,
        slug=main_handle.slug,
        name=main_handle.slug,
        owned_by=owned_by,
        region=region,
    )
    session._create(bench)
    await session.flush(optimistic=True)

    # main Package
    main_package = bench.packages.create(type=PackageType.OPEN, name="Main Package", slug="main")
    await session.flush(optimistic=True)
    bench.main_package = main_package

    # main Store
    store = main_package.stores.create(region=bench.region, name="Local Store")
    await session.flush(optimistic=True)
    bench.main_store = store
    await session.flush(optimistic=True)

    if options.create_machine_scaler:
        machine_scaler = main_package.scalers.create(  # noqa: F841
            type=ScalerType.MACHINE,
            strategy=ScalerStrategy.AUTO,
            name="Machine Scaler",
            min_count=0,
            target_count=0,
            max_count=4,
            is_main=True,
        )

    return bench
