import abc
from typing import ClassVar, Collection, Iterable, override

import structlog

from bench.language import Bench, Resource, Server, Store
from bench.language.bench import ResourceStatus
from bench.language.const import VERSION, NodeType
from bench.language.session import Session
from bench.sql.client import pg_cursor_to_store
from bench.sql.migration import has_migration_after, migrate
from bench.system.core import HostPlugin
from bench.system.neon import NeonApi
from bench.utils.env import ENVIRONMENT
from bench.utils.func import bittuple

logger = structlog.get_logger(__name__)


class Provisioner[T: Resource](HostPlugin[T], abc.ABC):
    """
    A provisioner for resources of the declared types.
    Synchronize the declared state of Bench resources with their actual (external) state (both ways).
    """

    node_types: ClassVar[bittuple[NodeType]]

    def __init__(self, bench: Bench):
        super().__init__(bench)

    async def provision(self, resource: T):
        """Provision the resource."""
        raise NotImplementedError

    async def update(self, resource: T):
        """Update the resource properties."""
        pass

    async def migrate(self, resource: T):
        """Migrate the resource to the current version."""
        pass

    async def decommission(self, resource: T):
        """Decommission the resource."""
        raise NotImplementedError


class NeonStoreProvisioner(Provisioner[Store]):
    """Provision Stores with the Neon API."""

    node_types = bittuple(NodeType.STORE)

    def __init__(self, bench: Bench, neon_api: "NeonApi"):
        super().__init__(bench)
        self._neon_api = neon_api

    @override
    async def provision(self, resource: Store):
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            resource.external_name = f"{ENVIRONMENT}-{resource.bench_id}"
        neon_project = await self._neon_api.create_project(
            name=resource.external_name, region=resource.region, pg_version=16
        )
        resource.external_id = neon_project.project_id
        resource.connection_uri = neon_project.connection_uri
        resource.status = ResourceStatus.HEALTHY

    @override
    async def migrate(self, resource: Store):
        # NOTE :Robustness: unsure when to migrate local stores :StoreMigration
        if resource.version is not None and not has_migration_after(
            resource.version, is_global=False
        ):
            return  # nothing to do
        async with pg_cursor_to_store(resource) as cur:
            await migrate(cur, target=VERSION, is_global=False, store=resource)
            await cur.connection.commit()
        resource.version = VERSION

    @override
    async def decommission(self, resource: Store):
        assert resource.external_id, f"{resource!r} has no external ID"
        await self._neon_api.delete_project(project_id=resource.external_id)
        resource.status = ResourceStatus.DESTROYED


# nocheckin: implement provisioners
class LocalhostServerProvisioner(Provisioner[Server]):
    """Provision Servers by short-circuiting Machines to localhost."""

    node_types = bittuple(NodeType.SERVER, NodeType.MACHINE)


class DockerServerProvisioner(Provisioner[Server]):
    """Provision Servers by deploying Machines as containers in a Docker installation."""

    node_types = bittuple(NodeType.SERVER, NodeType.MACHINE)


class KubernetesServerProvisioner(Provisioner[Server]):
    """Provision Servers by deploying Machines as Pods on Kubernetes."""

    node_types = bittuple(NodeType.SERVER, NodeType.MACHINE)


def get_provisioners_for(bench: Bench) -> list[Provisioner]:
    from bench.system.neon import neon_api

    if ENVIRONMENT == "dev" or ENVIRONMENT == "test":
        return [
            NeonStoreProvisioner(bench, neon_api),
            LocalhostServerProvisioner(bench),
        ]
    elif ENVIRONMENT == "prod":
        return [
            NeonStoreProvisioner(bench, neon_api),
            KubernetesServerProvisioner(bench),
        ]
    else:
        raise RuntimeError(f"unexpected environment: {ENVIRONMENT!r}")


def get_provisioner(
    resource: Resource, provisioners: Collection[Provisioner]
) -> Provisioner | None:
    """Gets the first suitable provisioner (if any)"""
    for provisioner in provisioners:
        if resource.metatype in provisioner.node_types:
            return provisioner
    return None


async def provision_resource(resource: Resource, provisioners: Collection[Provisioner]):
    provisioner = get_provisioner(resource, provisioners)
    if provisioner is None:
        raise ValueError(f"no provisioner for {resource!r} in {provisioners!r}")


async def provision_resources(
    resources: Iterable[Resource], provisioners: Collection[Provisioner], session: Session
):
    for resource in resources:
        if resource.status != ResourceStatus.PENDING:
            continue
        await provision_resource(resource, provisioners)
        await session.commit()


async def migrate_resource(resource: Resource, provisioners: Collection[Provisioner]):
    provisioner = get_provisioner(resource, provisioners)
    if provisioner is None:
        raise ValueError(f"no provisioner for {resource!r} in {provisioners!r}")
    await provisioner.migrate(resource)


async def migrate_resources(
    resources: Iterable[Resource], provisioners: Collection[Provisioner], session: Session
):
    for resource in resources:
        if resource.status != ResourceStatus.HEALTHY:
            continue
        await migrate_resource(resource, provisioners)
        await session.commit()


async def decommission_resource(resource: Resource, provisioners: Collection[Provisioner]):
    provisioner = get_provisioner(resource, provisioners)
    if provisioner is None:
        raise ValueError(f"no provisioner for {resource!r} in {provisioners!r}")
    await provisioner.decommission(resource)


async def decommission_all_resources(
    resources: Iterable[Resource], provisioners: Collection[Provisioner], session: Session
):
    for resource in resources:
        if resource.status == ResourceStatus.PENDING or resource.status == ResourceStatus.DESTROYED:
            continue
        await decommission_resource(resource, provisioners)
        await session.commit()
