import abc
from typing import TYPE_CHECKING, ClassVar, Collection, cast, final, override

import structlog
from opentelemetry import trace

from bench.language import Bench, Drive, Machine, Resource, ResourceStatus, Server, Store
from bench.language.bench import MachineProfile
from bench.language.const import VERSION, NodeType
from bench.sql.client import pg_cursor_to_store
from bench.sql.migration import sql_migrate
from bench.system.core import Commit, DeferredHostPlugin, HostSpec
from bench.system.neon import NeonApi
from bench.utils.env import ENVIRONMENT
from bench.utils.func import bittuple
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Provisioner[PT: Resource, WT: Resource](DeferredHostPlugin[WT], abc.ABC):
    """
    A provisioner for resources of the declared types.
    Synchronize the declared state of Bench resources with their actual (external) state (both ways).
    """

    """The nodes this Provisioner can handle (separate from node types to watch in HostPlugin.)"""
    provision_types: ClassVar[bittuple[NodeType]]

    @override
    async def start(self) -> None:
        # check resources
        resources = tuple(
            cast(PT, r) for r in self._bench.resources if r.metatype in self.provision_types
        )
        for resource in resources:
            # provision newly declared resources
            if resource.status == ResourceStatus.DECLARED:
                await self.provision(resource)
            # 'update' other resources
            else:
                # auto migrate resources to current version
                # NOTE :Robustness: unsure when to migrate resources
                if "version" in resource.__properties__ and getattr(resource, "version") != VERSION:
                    async with self._host.session(autocommit=True):
                        setattr(resource, "version", VERSION)
                await self.update(resource)

        # then start watching
        #  (Starting watch after above is important because there is no lock between this and on_commit_deferred,
        #   and we assume exclusivity in the provisioning methods. Host plugins starts the queue in .. start).
        await super().start()

    @final
    @tracer.start_as_current_span("provisioner.on_commit_deferred")
    async def on_commit_deferred(self, commit: Commit[WT]) -> None:
        trace.get_current_span().set_attribute("plugin", self.name)
        # handle edit by updating resource
        if commit.has(self.provision_types):
            subcommit = cast(Commit[PT], commit.trim_to(self.provision_types))
            for resource in subcommit.added:
                if resource.status == ResourceStatus.DECLARED:
                    await self.provision(resource)
            for resource in subcommit.updated:
                if resource.status.is_extant:
                    await self.update(resource)
            for resource in subcommit.removed:
                if resource.status.is_extant:
                    await self.decommission(resource)
        await self._on_commit_deferred(commit)

    async def _on_commit_deferred(self, commit: Commit[WT]) -> None:
        pass

    @final
    async def provision(self, resource: PT):
        """Provision the resource."""
        try:
            with tracer.start_as_current_span(
                "resource.provision", attributes={"resource": str(resource)}
            ):
                await self._provision(resource)
                logger.trace(
                    "resource.provision", provisioner=self, resource=resource, span="current"
                )
        except Exception as e:
            logger.error(
                "resource.provision.error",
                provisioner=self,
                resource=resource,
                error=e,
                exc_info=True,
                span="current",
            )
            raise

    @abc.abstractmethod
    async def _provision(self, resource: PT): ...

    @final
    async def update(self, resource: PT):
        """Update the resource properties."""
        try:
            with tracer.start_as_current_span(
                "resource.update", attributes={"resource": str(resource)}
            ):
                await self._update(resource)
                logger.trace("resource.update", provisioner=self, resource=resource, span='current')
        except Exception as e:
            logger.error(
                "resource.update.error", provisioner=self, resource=resource, error=e, exc_info=True, span='current'
            )
            raise

    async def _update(self, resource: PT):
        pass

    @final
    async def decommission(self, resource: PT):
        """Decommission the resource."""
        try:
            with tracer.start_as_current_span(
                "resource.decommission", attributes={"resource": str(resource)}
            ):
                await self._decommission(resource)
                logger.trace("resource.decommission", provisioner=self, resource=resource, span='current')
        except Exception as e:
            logger.error(
                "resource.decommission.error",
                provisioner=self,
                resource=resource,
                error=e,
                exc_info=True, span='current'
            )
            raise

    @abc.abstractmethod
    async def _decommission(self, resource: PT): ...


class NeonStoreProvisioner(Provisioner[Store, Store]):
    """Provision Stores with the Neon API."""

    watch_types = bittuple(NodeType.STORE)
    provision_types = bittuple(NodeType.STORE)

    def __init__(self, host: "HostSpec", bench: Bench, neon_api: "NeonApi"):
        super().__init__(host, bench)
        self._neon_api = neon_api

    async def _migrate(self, resource: Store):
        assert resource.version, f"{resource!r} has no version"
        async with pg_cursor_to_store(resource) as cur:
            await sql_migrate(cur, target=resource.version, is_global=False, store=resource)
            await cur.connection.commit()
        async with self._host.session(autocommit=True):
            resource.current_version = resource.version

    @override
    async def _provision(self, resource: Store):
        # need a name
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            async with self._host.session(autocommit=True):
                resource.external_name = f"{ENVIRONMENT}-{resource.bench_id}"

        # create Postgres database ('project')
        neon_project = await self._neon_api.create_project(
            name=resource.external_name, region=resource.region, pg_version=16
        )
        async with self._host.session(autocommit=True):
            resource.external_id = neon_project.project_id
            resource.connection_uri = neon_project.connection_uri
            if not resource.version:
                resource.version = VERSION
            resource.status = ResourceStatus.HEALTHY

        # migrate it immediately
        await self._migrate(resource)

    @override
    async def _update(self, resource: Store):
        # migrate
        if resource.version != resource.current_version:
            await self._migrate(resource)

    @override
    async def _decommission(self, resource: Store):
        assert resource.external_id, f"{resource!r} has no external ID"
        await self._neon_api.delete_project(project_id=resource.external_id)
        async with self._host.session(autocommit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class ElasticServerProvisioner(Provisioner[Server, Server | Machine]):
    """Provision Servers by creating/deleting/scaling Machines 'on-demand'."""

    watch_types = bittuple(NodeType.SERVER, NodeType.MACHINE)
    provision_types = bittuple(NodeType.SERVER)

    async def _tick(self, server: Server):
        # TODO :Incomplete: scale ElasticServerProvisioner properly (up/down/sleep/...)
        machines = server.machines.tolist()

        # rescale server if needed (poorly)
        if not machines:
            async with self._host.session(autocommit=True):
                machine = Machine(name="Machine1", profile=MachineProfile.TINY)
                server.machines.append(machine)
                server.status = ResourceStatus.PROVISIONING

        # update server status to reflect machines (if needed)
        if machines and all(m.status == ResourceStatus.HEALTHY for m in machines):
            actual_status = ResourceStatus.HEALTHY
        elif machines and any(m.status == ResourceStatus.UNHEALTHY for m in machines):
            actual_status = ResourceStatus.UNHEALTHY
        else:
            actual_status = ResourceStatus.HEALTHY
        if server.status != actual_status:
            async with self._host.session(autocommit=True):
                server.status = actual_status

    @override
    async def _on_commit_deferred(self, commit: Commit[Server | Machine]) -> None:
        # get any edited servers (directly or indirectly via machines)
        servers: set[Server] = set()
        for node in commit.edited:
            if isinstance(node, Server):
                servers.add(node)
            elif isinstance(node, Machine):
                servers.add(node.parent)
            else:
                raise TypeError(f"unexpected node type {type(node)}")

        # and check/update them
        for server in servers:
            if server.status != ResourceStatus.DECOMMISSIONED:
                await self._tick(server)

    @override
    async def _provision(self, resource: Server):
        await self._tick(resource)

    @override
    async def _update(self, resource: Server):
        await self._tick(resource)

    @override
    async def _decommission(self, resource: Server):
        # nothing special, child machines are automatically removed too
        async with self._host.session(autocommit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class LocalhostMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines by short-circuiting to localhost."""

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    def __init__(self, host: HostSpec, bench: Bench, local_machine_url: str):
        super().__init__(host, bench)
        self._local_machine_url = local_machine_url

    @override
    async def _provision(self, resource: Machine):
        async with self._host.session(autocommit=True):
            resource.connection_uri = self._local_machine_url
            resource.status = ResourceStatus.HEALTHY

    @override
    async def _decommission(self, resource: Machine):
        async with self._host.session(autocommit=True):
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
    async def _provision(self, resource: Drive):
        async with self._host.session(autocommit=True):
            resource.status = ResourceStatus.HEALTHY

    @override
    async def _decommission(self, resource: Drive):
        async with self._host.session(autocommit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


def get_provisioners_for(host: HostSpec, bench: Bench) -> list[Provisioner]:
    """Gets all available provisioners for that Bench in *this* environment"""
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


async def provision(host: HostSpec, bench: Bench, resources: Collection[Resource]) -> None:
    """Provisions the given resources in *this* environment"""
    provisioners = get_provisioners_for(host, bench)
    for resource in resources:
        for provisioner in provisioners:
            if resource.metatype in provisioner.provision_types:
                await provisioner.provision(resource)
                break
        else:
            raise RuntimeError(f"no provisioner for {resource!r} in {provisioners!r}")
