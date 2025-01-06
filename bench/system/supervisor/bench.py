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
from bench.system.core import local_pg_engine_from_store
from bench.system.host import HostProxy


async def create_default_bench(
    *,
    main_handle: Handle,
    owner: User | Organization,
    region: Region,
    global_store: Store,
    session: Session,
) -> Bench:
    """Creates a new Bench with all the default stuff."""

    from bench.system.provision.provisioner import provision

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
    # immediately provision local store
    await provision(HostProxy(global_store, session), bench, (store,))

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

    # main Package
    session._engines += (local_pg_engine_from_store(name=f"pg-local-{bench.slug}", store=store),)
    main_package = bench.packages.create(type=PackageType.ROOT, name="Main", slug="main")
    await session.flush(optimistic=True)
    bench.main_package = main_package

    return bench
