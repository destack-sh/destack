import random
import secrets
import string
from typing import Optional, cast

import boto3
import structlog

from bench.language import (
    Bench,
    Drive,
    Handle,
    Organization,
    Region,
    Resource,
    ResourceStatus,
    Server,
    ServerProfile,
    Session,
    Store,
    StoreEngineType,
    StoreKind,
    Tenancy,
    User,
)
from bench.language.resource import ResourceCredential
from bench.opensearch.engine import create_local_os_store
from bench.sql.engine import create_local_pg_store
from bench.system.auth import generate_encryption_key
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)


def generate_random_slug(length: int = 32) -> str:
    """Random alphanumeric slug."""
    letters = tuple(secrets.choice(string.ascii_lowercase) for _ in range(length))
    return "".join(letters)


def generate_random_username(length: int = 32, lowercase: bool = False) -> str:
    """Random alphanumeric username (starts with a text character)."""
    if lowercase:
        pool = string.ascii_lowercase + string.digits
    else:
        pool = string.ascii_letters + string.digits
    name = random.choice(string.ascii_lowercase)
    name += "".join(random.choice(pool) for _ in range(length - 1))
    return name


def generate_random_password(length: int = 48) -> str:
    """URL-safe password."""
    password = secrets.token_urlsafe(length - 4)[: length - 4]
    # ensure at least 1 lowercase, 1 uppercase, 1 digit, 1 'special' character
    password += random.choice(string.ascii_lowercase)
    password += random.choice(string.ascii_uppercase)
    password += random.choice(string.digits)
    password += random.choice("!@$^&*()_+-=")
    return password


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
        name="Main Server",
    )
    store = bench.stores.create(
        region=bench.region,
        tenancy=Tenancy.DEDICATED,
        kind=StoreKind.RELATIONAL,
        engine=StoreEngineType.POSTGRES,
        name="Main Store",
    )
    search = bench.stores.create(
        region=bench.region,
        tenancy=Tenancy.DEDICATED,
        kind=StoreKind.SEARCH,
        engine=StoreEngineType.OPENSEARCH,
        name="Main Search",
    )
    drive = bench.drives.create(region=bench.region, tenancy=Tenancy.SHARED, name="Main Drive")

    # create main environment/branch/package
    environment = bench.environments.create(
        name="Main Environment", store=store, search=search, drive=drive, server=server
    )
    branch = bench.branches.create(name="Main", slug="main")
    package = bench.packages.create(environment=environment)
    await session.flush()
    branch.main_package = package
    bench.main_environment = environment
    bench.main_branch = branch

    return bench


async def provision_resource(resource: Resource, session: Session) -> None:
    """Provisions a newly created resource."""
    assert resource.status == ResourceStatus.PENDING, f"{resource!r} is already provisioned"
    logger.info("resource.provision", resource=resource)
    if isinstance(resource, Server):
        cast(Server, resource)
        ...  # where should Servers & Machines be provisioned?
    elif isinstance(resource, Store):
        store = cast(Store, resource)
        store.database = generate_random_slug()
        store.main_credential = ResourceCredential(
            username=generate_random_username(),
            password=generate_random_password(),
        )
        if resource.engine == StoreEngineType.POSTGRES:
            await create_local_pg_store(store)
        elif resource.engine == StoreEngineType.OPENSEARCH:
            await create_local_os_store(store)
        else:
            raise NotImplementedError(f"unexpected store {store!r} (yet)")
        store.status = ResourceStatus.HEALTHY
    elif isinstance(resource, Drive):
        drive = cast(Drive, resource)
        # nothing to create for drives
        drive.status = ResourceStatus.HEALTHY
    else:
        raise NotImplementedError(f"cannot provision {resource!r} (yet)")


async def provision_pending_resources(bench: Bench, session: Session):
    """Provisions all pending resources in a bench."""
    for resource in bench.resources:
        if resource.status == ResourceStatus.PENDING:
            await provision_resource(resource, session)


_s3_client: Optional["boto3.client"] = None


def get_s3_client() -> "boto3.client":
    global _s3_client
    if _s3_client is None:
        _s3_client = boto3.client(
            "s3",
            endpoint_url=get_from_env("AWS_ENDPOINT_URL"),
        )
    return _s3_client
