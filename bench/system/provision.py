import abc
from typing import TYPE_CHECKING, ClassVar, Collection, Iterable, override

import structlog

from bench.language import Bench, Machine, Resource, ResourceStatus, Server, Store
from bench.language.const import VERSION, NodeType
from bench.language.session import Session
from bench.sql.client import pg_cursor_to_store
from bench.sql.migration import has_migration_after, migrate
from bench.system.core import AsyncHostPlugin, CommittedChange, HostSpec
from bench.system.neon import NeonApi
from bench.utils.env import ENVIRONMENT
from bench.utils.func import bittuple
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


class Provisioner[T: Resource](AsyncHostPlugin[T], abc.ABC):
    """
    A provisioner for resources of the declared types.
    Synchronize the declared state of Bench resources with their actual (external) state (both ways).
    """

    """
    The types of nodes this Provisioner can handle
    (separate from node types to watch in plugin.)
    """
    provision_types: ClassVar[bittuple[NodeType]]

    @override
    async def on_graph_commit_async(self, commit: CommittedChange[T]) -> None:
        if commit.has(self.provision_types):
            provision_commit = commit.trim_to(self.provision_types)
            async with self._host.session() as session:
                for resource in provision_commit.added:
                    await self.provision(resource)
                    await session.commit()
                for resource in provision_commit.updated:
                    await self.update(resource)
                    await session.commit()
                for resource in provision_commit.removed:
                    await self.decommission(resource)
                    await session.commit()

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

    watch_types = provision_types = bittuple(NodeType.STORE)

    def __init__(self, host: "HostSpec", bench: Bench, neon_api: "NeonApi"):
        super().__init__(host, bench)
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


class ElasticServerProvisioner(Provisioner[Server]):
    """Provision Servers by creating/deleting/scaling Machines 'on-demand'."""

    watch_types = bittuple(NodeType.SERVER, NodeType.MACHINE)
    provision_types = bittuple(NodeType.SERVER)

    # TODO :Broken: scale machines properly for server

    @override
    async def provision(self, resource: Server):
        machine = Machine(name="Machine1", profile=resource.profile)
        resource.machines.append(machine)

    @override
    async def update(self, resource: Server):
        pass  # see above

    @override
    async def decommission(self, resource: Server):
        pass  # nothing special, child machines are automatically removed too


class LocalhostMachineProvisioner(Provisioner[Machine]):
    """Provision Machines by short-circuiting to localhost."""

    watch_types = provision_types = bittuple(NodeType.MACHINE)

    def __init__(self, host: HostSpec, bench: Bench, local_machine_url: str):
        super().__init__(host, bench)
        self._local_machine_url = local_machine_url

    @override
    async def provision(self, resource: Machine):
        resource.connection_uri = self._local_machine_url
        resource.status = ResourceStatus.HEALTHY

    @override
    async def decommission(self, resource: Machine):
        resource.status = ResourceStatus.DESTROYED


class DockerMachineProvisioner(Provisioner[Machine]):
    """Provision Machines as containers in a Docker installation."""

    watch_types = provision_types = bittuple(NodeType.MACHINE)

    # TODO :Incomplete: DockerMachineProvisioner


class KubernetesMachineProvisioner(Provisioner[Machine]):
    """Provision Machines as Pods on Kubernetes."""

    watch_types = provision_types = bittuple(NodeType.MACHINE)

    # TODO :Incomplete: KubernetesMachineProvisioner


def get_provisioners_for(host: HostSpec, bench: Bench) -> list[Provisioner]:
    from bench.system.neon import neon_api

    if ENVIRONMENT == "dev" or ENVIRONMENT == "test":
        return [
            NeonStoreProvisioner(host, bench, neon_api),
            ElasticServerProvisioner(host, bench),
            LocalhostMachineProvisioner(host, bench, get_from_env("LOCAL_MACHINE_URL")),
        ]
    elif ENVIRONMENT == "prod":
        return [
            NeonStoreProvisioner(host, bench, neon_api),
            ElasticServerProvisioner(host, bench),
            KubernetesMachineProvisioner(host, bench),
        ]
    else:
        raise RuntimeError(f"unexpected environment: {ENVIRONMENT!r}")


def get_provisioner(
    resource: Resource, provisioners: Collection[Provisioner]
) -> Provisioner | None:
    """Gets the first suitable provisioner (if any)"""
    for provisioner in provisioners:
        if resource.metatype in provisioner.provision_types:
            return provisioner
    return None


async def provision_resource(resource: Resource, provisioners: Collection[Provisioner]):
    provisioner = get_provisioner(resource, provisioners)
    if provisioner is None:
        raise ValueError(f"no provisioner for {resource!r} in {provisioners!r}")
    await provisioner.provision(resource)


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
