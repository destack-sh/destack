from bench.language import (
    Bench,
    Handle,
    Organization,
    PackageType,
    Region,
    ScalerStrategy,
    ScalerType,
    Session,
    Store,
    User,
)


async def create_default_bench(
    *,
    main_handle: Handle,
    owned_by: User | Organization,
    region: Region,
    global_store: Store,
    session: Session,
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
    main_package = bench.packages.create(type=PackageType.MAIN, name="Main", slug="main")
    await session.flush(optimistic=True)
    bench.main_package = main_package

    # main Store
    store = main_package.stores.create(region=bench.region, name="Store")
    await session.flush(optimistic=True)
    bench.main_store = store
    await session.flush(optimistic=True)

    # default Scalers
    machine_scaler = main_package.scalers.create(  # noqa: F841
        type=ScalerType.MACHINE,
        strategy=ScalerStrategy.AUTO,
        name="Machine Scaler",
        min_count=1,
        target_count=1,
        max_count=4,
        is_main=True,
    )

    return bench
