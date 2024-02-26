from typing import Optional

import boto3

from bench.language import (
    Bench,
    Handle,
    Organization,
    Region,
    ServerProfile,
    Session,
    StoreEngineType,
    StoreKind,
    Tenancy,
    User,
)
from bench.system.auth import generate_encryption_key
from bench.utils.utils import get_from_env


async def create_default_bench(
    main_handle: Handle, owner: User | Organization, region: Region, session: Session
) -> Bench:
    # create bench
    bench = Bench(
        main_handle=main_handle,
        slug=main_handle.slug,
        name=main_handle.slug,
        owner=owner,
        encryption_key=generate_encryption_key(),
        region=region,
    )
    session.create(bench)
    await session.flush()

    # create resources (in pending state, resources are managed by hosts)
    server = bench.servers.create(
        region=bench.region,
        tenancy=Tenancy.SHARED,
        profile=ServerProfile.SMALL,
        name="Server",
    )
    store = bench.stores.create(
        region=bench.region,
        tenancy=Tenancy.DEDICATED,
        kind=StoreKind.RELATIONAL,
        engine=StoreEngineType.POSTGRES,
        name="Store",
    )
    search = bench.stores.create(
        region=bench.region,
        tenancy=Tenancy.DEDICATED,
        kind=StoreKind.SEARCH,
        engine=StoreEngineType.OPENSEARCH,
        name="Search",
    )
    drive = bench.drives.create(region=bench.region, tenancy=Tenancy.SHARED, name="Drive")

    # create main environment/branch/package
    environment = bench.environments.create(
        name="Main", store=store, search=search, drive=drive, server=server
    )
    branch = bench.branches.create(name="Main", slug="main")
    package = bench.packages.create(environment=environment)
    await session.flush()
    branch.main_package = package
    bench.main_environment = environment
    bench.main_branch = branch

    return bench


_s3_client: Optional["boto3.client"] = None


def get_s3_client() -> "boto3.client":
    global _s3_client
    if _s3_client is None:
        _s3_client = boto3.client(
            "s3",
            endpoint_url=get_from_env("AWS_ENDPOINT_URL"),
        )
    return _s3_client
