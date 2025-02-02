from bench.language import (
    Bench,
    Handle,
    Organization,
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
    owner: User | Organization,
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
        owner=owner,
        region=region,
    )
    session._create(bench)
    await session.flush(optimistic=True)

    # main Store
    store = bench.stores.create(region=bench.region, name="Store")
    await session.flush(optimistic=True)
    bench.main_store = store
    await session.flush(optimistic=True)

    # default Scalers
    machine_scaler = bench.scalers.create(  # noqa: F841
        type=ScalerType.MACHINE,
        strategy=ScalerStrategy.AUTO,
        name="Machine Scaler",
        min_count=1,
        target_count=1,
        max_count=4,
        is_main=True,
    )

    return bench
