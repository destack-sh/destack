import abc
from typing import TYPE_CHECKING, ClassVar, cast, final, override

import structlog

from bench.language import Bench, Drive, Machine, Resource, ResourceStatus, Server, Store
from bench.language.bench import MachineProfile
from bench.language.const import VERSION, NodeType
from bench.language.session import Session
from bench.sql.client import pg_cursor_to_store
from bench.sql.migration import sql_migrate
from bench.system.core import Commit, DeferredHostPlugin, HostSpec
from bench.system.neon import NeonApi
from bench.utils.dt import monotime
from bench.utils.env import ENVIRONMENT
from bench.utils.func import bittuple
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


class Provisioner[PT: Resource, WT: Resource](DeferredHostPlugin[WT], abc.ABC):
    """
    A provisioner for resources of the declared types.
    Synchronize the declared state of Bench resources with their actual (external) state (both ways).
    """

    """The nodes this Provisioner can handle (separate from node types to watch in HostPlugin.)"""
    provision_types: ClassVar[bittuple[NodeType]]

    @override
    async def start(self, session: Session) -> None:
        await super().start(session)

        resources = tuple(
            cast(PT, r) for r in self._bench.resources if r.metatype in self.provision_types
        )
        for resource in resources:
            # provision newly declared resources
            if resource.status == ResourceStatus.DECLARED:
                await self.provision(session, resource)
                await session.commit()
            # 'update' other resources
            else:
                # auto migrate resources to current version
                # NOTE :Robustness: unsure when to migrate resources
                if "version" in resource.__properties__ and getattr(resource, "version") != VERSION:
                    setattr(resource, "version", VERSION)
                await self.update(session, resource)
                await session.commit()

    @override
    async def _on_commit_deferred(self, session: Session, commit: Commit[WT]) -> None:
        # handle edit by updating resource
        if commit.has(self.provision_types):
            subcommit = cast(Commit[PT], commit.trim_to(self.provision_types))
            for resource in subcommit.added:
                if resource.status == ResourceStatus.DECLARED:
                    await self.provision(session, resource)
                    await session.commit()
            for resource in subcommit.updated:
                if resource.status.is_extant:
                    await self.update(session, resource)
                    await session.commit()
            for resource in subcommit.removed:
                if resource.status.is_extant:
                    await self.decommission(session, resource)
                    await session.commit()

    @final
    async def provision(self, session: Session, resource: PT):
        """Provision the resource."""
        try:
            start = monotime()
            await self._provision(session, resource)
            logger.info("resource.provision", resource=resource, duration=monotime() - start)
        except Exception as e:
            logger.error("resource.provision.error", resource=resource, error=e, exc_info=True)
            raise

    @abc.abstractmethod
    async def _provision(self, session: Session, resource: PT): ...

    @final
    async def update(self, session: Session, resource: PT):
        """Update the resource properties."""
        try:
            start = monotime()
            await self._update(session, resource)
            logger.trace("resource.update", resource=resource, duration=monotime() - start)
        except Exception as e:
            logger.error("resource.update.error", resource=resource, error=e, exc_info=True)
            raise

    async def _update(self, session: Session, resource: PT):
        pass

    @final
    async def decommission(self, session: Session, resource: PT):
        """Decommission the resource."""
        try:
            start = monotime()
            await self._decommission(session, resource)
            logger.info("resource.decommission", resource=resource, duration=monotime() - start)
        except Exception as e:
            logger.error("resource.decommission.error", resource=resource, error=e, exc_info=True)
            raise

    @abc.abstractmethod
    async def _decommission(self, session: Session, resource: PT): ...


class NeonStoreProvisioner(Provisioner[Store, Store]):
    """Provision Stores with the Neon API."""

    watch_types = bittuple(NodeType.STORE)
    provision_types = bittuple(NodeType.STORE)

    def __init__(self, host: "HostSpec", bench: Bench, neon_api: "NeonApi"):
        super().__init__(host, bench)
        self._neon_api = neon_api

    async def _migrate(self, resource: Store):
        assert resource.version, f"{resource!r} has no version"
        try:
            start = monotime()
            async with pg_cursor_to_store(resource) as cur:
                await sql_migrate(cur, target=resource.version, is_global=False, store=resource)
                await cur.connection.commit()
            resource.current_version = resource.version
            logger.info("resource.migrate", resource=resource, duration=monotime() - start)
        except Exception as e:
            logger.error("resource.migrate.error", resource=resource, error=e, exc_info=True)
            raise

    @override
    async def _provision(self, session: Session, resource: Store):
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            resource.external_name = f"{ENVIRONMENT}-{resource.bench_id}"
        neon_project = await self._neon_api.create_project(
            name=resource.external_name, region=resource.region, pg_version=16
        )
        resource.external_id = neon_project.project_id
        resource.connection_uri = neon_project.connection_uri
        if not resource.version:
            resource.version = VERSION
        await self._migrate(resource)
        resource.status = ResourceStatus.HEALTHY

    @override
    async def _update(self, session: Session, resource: Store):
        assert resource.current_version, f"{resource!r} has no current version"
        if resource.version != resource.current_version:
            await self._migrate(resource)

    @override
    async def _decommission(self, session: Session, resource: Store):
        assert resource.external_id, f"{resource!r} has no external ID"
        await self._neon_api.delete_project(project_id=resource.external_id)
        resource.status = ResourceStatus.DECOMMISSIONED


class ElasticServerProvisioner(Provisioner[Server, Server | Machine]):
    """Provision Servers by creating/deleting/scaling Machines 'on-demand'."""

    watch_types = bittuple(NodeType.SERVER, NodeType.MACHINE)
    provision_types = bittuple(NodeType.SERVER)

    # TODO :Broken: scale & react to machines properly in ElasticServerProvisioner

    @override
    async def _on_commit(self, session: Session, commit: Commit[Server | Machine]) -> None:
        if commit.has(NodeType.MACHINE):
            # mark server as healthy/unhealthy based on its machines
            subcommit = cast(Commit[Machine], commit.trim_to(NodeType.MACHINE))
            servers = {machine.parent.id: machine.parent for machine in subcommit.edited}
            for server in servers.values():
                all_healthy = all(
                    machine.status == ResourceStatus.HEALTHY for machine in server.machines
                )
                if all_healthy and server.status != ResourceStatus.HEALTHY:
                    server.status = ResourceStatus.HEALTHY
                elif not all_healthy and server.status == ResourceStatus.HEALTHY:
                    server.status = ResourceStatus.UNHEALTHY

    @override
    async def _provision(self, session: Session, resource: Server):
        machine = Machine(name="Machine1", region=resource.region, profile=MachineProfile.TINY)
        resource.machines.append(machine)
        resource.status = ResourceStatus.PROVISIONING

    @override
    async def _update(self, session: Session, resource: Server):
        pass  # see above

    @override
    async def _decommission(self, session: Session, resource: Server):
        # nothing special, child machines are automatically removed too
        resource.status = ResourceStatus.DECOMMISSIONED


class LocalhostMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines by short-circuiting to localhost."""

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    def __init__(self, host: HostSpec, bench: Bench, local_machine_url: str):
        super().__init__(host, bench)
        self._local_machine_url = local_machine_url

    @override
    async def _provision(self, session: Session, resource: Machine):
        resource.connection_uri = self._local_machine_url
        resource.status = ResourceStatus.HEALTHY

    @override
    async def _decommission(self, session: Session, resource: Machine):
        resource.status = ResourceStatus.DECOMMISSIONED


class DockerMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines as containers in a Docker installation."""

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    # TODO :Incomplete: DockerMachineProvisioner


class KubernetesMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines as Pods on Kubernetes."""

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    # TODO :Incomplete: KubernetesMachineProvisioner


class S3DriveProvisioner(Provisioner[Drive, Drive]):
    """Provision Drives with an S3-compatible API."""

    watch_types = bittuple(NodeType.DRIVE)
    provision_types = bittuple(NodeType.DRIVE)

    # TODO :Incomplete: S3DriveProvisioner

    @override
    async def _provision(self, session: Session, resource: Drive):
        resource.status = ResourceStatus.HEALTHY

    @override
    async def _decommission(self, session: Session, resource: Drive):
        resource.status = ResourceStatus.DECOMMISSIONED


def get_provisioners_for(host: HostSpec, bench: Bench) -> list[Provisioner]:
    from bench.system.neon import neon_api

    if ENVIRONMENT == "dev" or ENVIRONMENT == "test":
        return [
            NeonStoreProvisioner(host, bench, neon_api),
            ElasticServerProvisioner(host, bench),
            LocalhostMachineProvisioner(host, bench, get_from_env("LOCAL_MACHINE_URL")),
            S3DriveProvisioner(host, bench),
        ]
    elif ENVIRONMENT == "prod":
        return [
            NeonStoreProvisioner(host, bench, neon_api),
            ElasticServerProvisioner(host, bench),
            S3DriveProvisioner(host, bench),
        ]
    else:
        raise RuntimeError(f"unexpected environment: {ENVIRONMENT!r}")
