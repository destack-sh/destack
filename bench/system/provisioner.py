import abc
from typing import TYPE_CHECKING, ClassVar, Collection, cast, final

import docker
import structlog
from opentelemetry import trace

from bench.language import Bench, ResourceNode, ResourceStatus
from bench.language.const import VERSION, NodeType
from bench.system.core import Commit, DeferredHostPlugin, HostApi
from bench.utils.env import ENV, Env
from bench.utils.func import bittuple
from bench.utils.utils import get_from_env_maybe

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Provisioner[PT: ResourceNode, WT: ResourceNode](DeferredHostPlugin[WT], abc.ABC):
    """
    A provisioner for resources of the declared types.
    Synchronize the declared state of Bench resources with their actual (external) state (both ways).
    """

    """The nodes this Provisioner can handle (separate from node types to watch in HostPlugin.)"""
    provision_types: ClassVar[bittuple[NodeType]]

    @final
    async def start(self) -> None:
        # check resources
        resources = tuple(
            cast(PT, r) for r in self.bench.resources if r.metatype in self.provision_types
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
                    async with self.host.session(commit=True):
                        setattr(resource, "version", VERSION)
                await self.update(resource)

        # then start watching
        #  (Starting watch after above is important because there is no lock between this and on_commit_deferred,
        #   and we assume exclusivity in the provisioning methods. Host plugins starts the queue in .start).
        await super().start()

        await self._do_start()

    async def _do_start(self) -> None:
        pass  # to be overridden

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
        await self._do_on_commit_deferred(commit)

    async def _do_on_commit_deferred(self, commit: Commit[WT]) -> None:
        pass  # to be overridden

    @final
    async def provision(self, resource: PT):
        """Provision the resource."""
        try:
            with tracer.start_as_current_span(
                "resource.provision", attributes={"resource": str(resource)}
            ):
                await self._do_provision(resource)
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
    async def _do_provision(self, resource: PT): ...

    @final
    async def update(self, resource: PT):
        """Update the resource properties."""
        try:
            with tracer.start_as_current_span(
                "resource.update", attributes={"resource": str(resource)}
            ):
                await self._do_update(resource)
                logger.trace("resource.update", provisioner=self, resource=resource, span="current")
        except Exception as e:
            logger.error(
                "resource.update.error",
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
        """Decommission the resource."""
        try:
            with tracer.start_as_current_span(
                "resource.decommission", attributes={"resource": str(resource)}
            ):
                await self._do_decommission(resource)
                logger.trace(
                    "resource.decommission", provisioner=self, resource=resource, span="current"
                )
        except Exception as e:
            logger.error(
                "resource.decommission.error",
                provisioner=self,
                resource=resource,
                error=e,
                exc_info=True,
                span="current",
            )
            raise

    @abc.abstractmethod
    async def _do_decommission(self, resource: PT): ...


LOCAL_NACHINE_URL = get_from_env_maybe(
    "LOCAL_MACHINE_URL", description="URL for local machine runtime"
)


def get_provisioners_for(host: HostApi, bench: Bench) -> list[Provisioner]:
    """Gets all available provisioners for that Bench in *this* environment"""
    from bench.system.neon import neon_api
    from bench.system.server import (
        DockerApi,
        DockerMachineProvisioner,
        ElasticServerProvisioner,
        KubernetesMachineProvisioner,
        LocalhostMachineProvisioner,
    )
    from bench.system.store import (
        LocalhostStoreProvisioner,
        NeonStoreProvisioner,
        S3DriveProvisioner,
    )

    if ENV == Env.TEST:
        assert LOCAL_NACHINE_URL, "no LOCAL_MACHINE_URL"
        return [
            LocalhostStoreProvisioner(host, bench),
            ElasticServerProvisioner(host, bench),
            LocalhostMachineProvisioner(host, bench, LOCAL_NACHINE_URL),
            S3DriveProvisioner(host, bench),
        ]
    elif ENV == Env.DEV:
        return [
            LocalhostStoreProvisioner(host, bench),
            ElasticServerProvisioner(host, bench),
            DockerMachineProvisioner(host, bench, docker_api=DockerApi(docker.from_env())),
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


async def provision(host: HostApi, bench: Bench, resources: Collection[ResourceNode]) -> None:
    """Provisions the given resources in *this* environment"""
    provisioners = get_provisioners_for(host, bench)
    for resource in resources:
        for provisioner in provisioners:
            if resource.metatype in provisioner.provision_types:
                await provisioner.provision(resource)
                break
        else:
            raise RuntimeError(f"no provisioner for {resource!r} in {provisioners!r}")
