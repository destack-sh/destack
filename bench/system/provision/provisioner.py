import abc
import asyncio
import enum
from typing import TYPE_CHECKING, Collection, assert_never, cast, final

import structlog
from opentelemetry import trace

from bench.language import Bench, ResourceNode
from bench.language.bench import ResourceStatus
from bench.language.const import LOADED_BENCH_NODE_TYPES, VERSION, NodeType
from bench.language.setup import NODE_CLASS_BY_TYPE
from bench.system.host.core import Commit, DeferredHostPlugin, Host
from bench.utils.env import ENV, Env
from bench.utils.utils import get_from_env_maybe

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Provisioner[PT: ResourceNode, WT: ResourceNode](DeferredHostPlugin[WT], abc.ABC):
    """
    A provisioner for some types of Resources.
    Synchronizes the declared state of Resources with their actual (external) state (bidirectionally).
    """

    """The nodes this Provisioner can handle (separate from node types to watch in HostPlugin.)"""
    resource_type: NodeType

    def __init__(self, host: Host, bench: Bench):
        super().__init__(host, bench)
        self._lock = asyncio.Lock()

    @property
    def slug(self) -> str:
        return self.resource_type.bench_name.lower()

    @final
    async def start(self) -> None:
        # custom start for provisioner first to update resource status from external state
        await self._do_start()

        # check resources / provision declared resources
        if self.resource_type in LOADED_BENCH_NODE_TYPES:
            resources = cast(
                list[PT],
                self.bench._graph.get_descendants(self.bench, self.resource_type, recursive=True),
            )
        else:
            provision_cls = cast(type[PT], NODE_CLASS_BY_TYPE[self.resource_type])
            resources = await provision_cls.where(
                provision_cls.get_property("bench").eq(self.bench)
                & provision_cls.get_property("current_status").neq(ResourceStatus.GONE)
            ).tolist()

        for resource in resources:
            # auto migrate resources to current version
            # NOTE :Robustness: unsure when to migrate which resources
            if "version" in resource.__properties__ and getattr(resource, "version") != VERSION:
                async with self.host.session(commit=True):
                    setattr(resource, "version", VERSION)
            # provision/update/decommission
            if resource.status.is_extant:
                if resource.current_status.is_extant:
                    await self.update(resource)
                else:
                    await self.provision(resource)
            elif resource.current_status.is_extant:
                await self.decommission(resource)

        # then start watching in host plugin
        #  (starting watch after above is important because there is no lock between this and on_commit_deferred,
        #   and we assume exclusivity in the provisioning methods. Host plugins starts the queue in .start)
        await super().start()

    async def _do_start(self) -> None:
        pass  # to be overridden

    @final
    @tracer.start_as_current_span("provisioner.on_commit_deferred")
    async def on_commit_deferred(self, commit: Commit[WT]) -> None:
        trace.get_current_span().set_attribute("plugin", self.name)
        # handle edit by updating resource
        if commit.has(self.resource_type):
            subcommit = cast(Commit[PT], commit.trim_to(self.resource_type))
            for resource in subcommit.added:
                if resource.status.is_extant and not resource.current_status.is_extant:
                    await self.provision(resource)
            for resource in subcommit.updated:
                if resource.status.is_extant:
                    if resource.current_status.is_extant:
                        await self.update(resource)
                    else:
                        await self.provision(resource)
                elif resource.current_status.is_extant:
                    await self.decommission(resource)
            for resource in subcommit.removed:
                if resource.current_status.is_extant:
                    await self.decommission(resource)
        await self._do_on_commit_deferred(commit)

    async def _do_on_commit_deferred(self, commit: Commit[WT]) -> None:
        pass  # to be overridden

    @final
    async def provision(self, resource: PT):
        """Provision the Resource."""
        try:
            async with self._lock:
                with tracer.start_as_current_span(
                    f"{self.slug}.provision", attributes={"resource": str(resource)}
                ):
                    await self._do_provision(resource)
                    logger.debug(
                        f"{self.slug}.provision",
                        provisioner=self,
                        resource=resource,
                        span="current",
                    )
        except Exception as e:
            logger.error(
                f"{self.slug}.provision.error",
                provisioner=self,
                resource=resource,
                error=e,
                exc_info=True,
                span="current",
            )
            raise

    @abc.abstractmethod
    async def _do_provision(self, resource: PT): ...

    @final
    async def update(self, resource: PT):
        """Update the Resource properties."""
        try:
            async with self._lock:
                with tracer.start_as_current_span(
                    f"{self.slug}.update", attributes={"resource": str(resource)}
                ):
                    await self._do_update(resource)
                    logger.debug(
                        f"{self.slug}.update", provisioner=self, resource=resource, span="current"
                    )
        except Exception as e:
            logger.error(
                f"{self.slug}.update.error",
                provisioner=self,
                resource=resource,
                error=e,
                exc_info=True,
                span="current",
            )
            raise

    async def _do_update(self, resource: PT):
        pass

    @final
    async def decommission(self, resource: PT):
        """Decommission the Resource."""
        try:
            async with self._lock:
                with tracer.start_as_current_span(
                    f"{self.slug}.decommission", attributes={"resource": str(resource)}
                ):
                    await self._do_decommission(resource)
                    logger.debug(
                        f"{self.slug}.decommission",
                        provisioner=self,
                        resource=resource,
                        span="current",
                    )
        except Exception as e:
            logger.error(
                f"{self.slug}.decommission.error",
                provisioner=self,
                resource=resource,
                error=e,
                exc_info=True,
                span="current",
            )
            raise

    @abc.abstractmethod
    async def _do_decommission(self, resource: PT): ...


class MachineProvisionerType(enum.StrEnum):
    LOCALHOST = "localhost"
    DOCKER = "docker"
    KUBERNETES = "kubernetes"


MACHINE_PROVISIONER_TYPE = get_from_env_maybe(
    "MACHINE_PROVISIONER_TYPE",
    typ=MachineProvisionerType,
    description="The machine provisioner to use (during development)",
)


def get_provisioners_for(host: Host, bench: Bench) -> list[Provisioner]:
    """Gets all available provisioners for that Bench in *this* environment"""
    from bench.system.provision.machine import (
        DockerMachineProvisioner,
        KubernetesMachineProvisioner,
        LocalhostMachineProvisioner,
    )
    from bench.system.provision.server import ElasticServerProvisioner
    from bench.system.provision.store import (
        LocalhostStoreProvisioner,
        NeonStoreProvisioner,
        S3DriveProvisioner,
        neon_api,
    )

    if ENV == Env.TEST:
        return [
            LocalhostStoreProvisioner(host, bench),
            ElasticServerProvisioner(host, bench),
            LocalhostMachineProvisioner(host, bench),
            S3DriveProvisioner(host, bench),
        ]
    elif ENV == Env.DEV:
        # dynamic machine provisioner
        assert MACHINE_PROVISIONER_TYPE is not None, "MACHINE_PROVISIONER_TYPE not set"
        if MACHINE_PROVISIONER_TYPE == MachineProvisionerType.LOCALHOST:
            machine_provisioner = LocalhostMachineProvisioner(host, bench)
        elif MACHINE_PROVISIONER_TYPE == MachineProvisionerType.DOCKER:
            machine_provisioner = DockerMachineProvisioner(host, bench)
        elif MACHINE_PROVISIONER_TYPE == MachineProvisionerType.KUBERNETES:
            machine_provisioner = KubernetesMachineProvisioner(host, bench)
        else:
            assert_never(MACHINE_PROVISIONER_TYPE)

        return [
            LocalhostStoreProvisioner(host, bench),
            ElasticServerProvisioner(host, bench),
            machine_provisioner,
            S3DriveProvisioner(host, bench),
        ]
    elif ENV == Env.STAGE or ENV == Env.PROD:
        return [
            NeonStoreProvisioner(host, bench, neon_api),
            ElasticServerProvisioner(host, bench),
            KubernetesMachineProvisioner(host, bench),
            S3DriveProvisioner(host, bench),
        ]
    else:
        raise RuntimeError(f"unexpected environment: {ENV!r}")


async def provision(host: Host, bench: Bench, resources: Collection[ResourceNode]) -> None:
    """Provisions the given resources in *this* environment"""
    provisioners = get_provisioners_for(host, bench)
    for resource in resources:
        for provisioner in provisioners:
            if resource.metatype == provisioner.resource_type:
                await provisioner.provision(resource)
                break
        else:
            raise RuntimeError(f"no provisioner for {resource!r} in {provisioners!r}")


async def decommission(host: Host, bench: Bench, resources: Collection[ResourceNode]) -> None:
    """Decommissions the given resources in *this* environment"""
    provisioners = get_provisioners_for(host, bench)
    for resource in resources:
        for provisioner in provisioners:
            if resource.metatype == provisioner.resource_type:
                await provisioner.decommission(resource)
                break
        else:
            raise RuntimeError(f"no provisioner for {resource!r} in {provisioners!r}")
