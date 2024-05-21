import boto3
import structlog

from bench.language import Bench, Drive, Resource, ResourceStatus, Server, Session, Store
from bench.system.neon import create_local_store, delete_local_store, migrate_local_store
from bench.utils.env import ENVIRONMENT
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)


async def provision_resource(resource: Resource, session: Session) -> None:
    """Provisions a newly created resource."""
    assert resource.status == ResourceStatus.PENDING, f"{resource!r} is already provisioned"
    logger.info("resource.provision", resource=resource)
    if isinstance(resource, Server):
        # nocheckin: where should Servers & Machines be provisioned?
        ...
    elif isinstance(resource, Store):
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            resource.external_name = f"{ENVIRONMENT}-{resource.bench_id}"
        await create_local_store(resource)
    elif isinstance(resource, Drive):
        pass  # nothing to do?
    else:
        raise RuntimeError(f"cannot provision {resource!r} (yet)")
    resource.status = ResourceStatus.HEALTHY


async def decommission_resource(resource: Resource, session: Session) -> None:
    """Decommissions a resource."""
    logger.info("resource.decommission", resource=resource)
    if isinstance(resource, Server):
        ...  # nothing to do?
    elif isinstance(resource, Store):
        await delete_local_store(resource)
    elif isinstance(resource, Drive):
        pass  # nothing to do?
    else:
        raise RuntimeError(f"cannot decommission {resource!r} (yet)")
    resource.status = ResourceStatus.DELETED


async def provision_resources(bench: Bench, session: Session, *, commit_per: bool):
    """Provisions all pending resources in a bench."""
    for resource in bench.resources:
        if resource.status == ResourceStatus.PENDING:
            await provision_resource(resource, session)
            if commit_per:
                await session.commit()


async def decommission_all_resources(bench: Bench, session: Session, *, commit_per: bool) -> None:
    """Decommissions all resources in a Bench."""
    for resource in bench.resources:
        if resource.status == ResourceStatus.HEALTHY:
            await decommission_resource(resource, session)
            if commit_per:
                await session.commit()


async def migrate_local_stores(bench: Bench, session: Session) -> None:
    """Migrates all local stores in a Bench (to our internal schema)."""
    for store in bench.stores:
        if store.status == ResourceStatus.HEALTHY:
            await migrate_local_store(store)


_s3_client = None


def get_s3_client():
    global _s3_client
    if _s3_client is None:
        _s3_client = boto3.client(
            "s3",
            endpoint_url=get_from_env("AWS_ENDPOINT_URL"),
        )
    return _s3_client
