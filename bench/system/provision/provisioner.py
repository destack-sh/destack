import abc
import asyncio
from typing import TYPE_CHECKING, Collection, cast, final

import structlog
from opentelemetry import trace

from bench.language import Bench, Resource
from bench.language.bench import ResourceStatus
from bench.language.const import (
    VERSION,
    VIRTUAL_RESOURCE_NODE_TYPES,
    NodeType,
)
from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.system.host.core import Commit, DeferredHostPlugin, Host

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Provisioner[PT: Resource, WT: Resource](DeferredHostPlugin[WT], abc.ABC):
    """
    A provisioner for some type of Resource.
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
        if self.resource_type in VIRTUAL_RESOURCE_NODE_TYPES:
            resources = cast(
                list[PT],
                self.bench._graph.get_descendants(self.bench, self.resource_type, recursive=True),
            )
        else:
            provision_cls = cast(type[PT], NODE_CLASS_BY_TYPE[self.resource_type])
            resources = (
                await provision_cls.where(
                    provision_cls.get_property("bench").eq(self.bench)
                    & provision_cls.get_property("status").neq(ResourceStatus.DECOMMISSIONED)
                )
                .select_all()
                .tolist()
            )

        # auto migrate resources to current version
        for resource in resources:
            # NOTE :Robustness: unsure when to migrate which resources
            if "version" in resource.__properties__ and getattr(resource, "version") != VERSION:
                async with self.host.session(commit=True):
                    setattr(resource, "version", VERSION)
            # provision/update/decommission
            if resource.target_status.is_extant:
                if resource.status.is_extant:
                    await self.update(resource)
                else:
                    await self.provision(resource)
            elif resource.status.is_extant:
                await self.decommission(resource)

        # then start watching in host plugin
        #  (starting watch after above is important because there is no lock between this and on_commit_deferred,
        #   and we assume exclusivity in the provisioning methods. Host plugins start the queue in .start)
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
                if resource.target_status.is_extant and not resource.status.is_extant:
                    await self.provision(resource)
            for resource in subcommit.updated:
                if resource.target_status.is_extant:
                    if resource.status.is_extant:
                        await self.update(resource)
                    else:
                        await self.provision(resource)
                elif resource.status.is_extant:
                    await self.decommission(resource)
            for resource in subcommit.removed:
                if resource.status.is_extant:
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


async def provision(host: Host, bench: Bench, resources: Collection[Resource]) -> None:
    """Provisions the given resources in *this* environment"""
    from bench.system.provision.registry import get_provisioners_for

    provisioners = get_provisioners_for(host, bench)
    for resource in resources:
        for provisioner in provisioners:
            if resource.metatype == provisioner.resource_type:
                await provisioner.provision(resource)
                break
        else:
            raise RuntimeError(f"no provisioner for {resource!r} in {provisioners!r}")


async def decommission(host: Host, bench: Bench, resources: Collection[Resource]) -> None:
    """Decommissions the given resources in *this* environment"""
    from bench.system.provision.registry import get_provisioners_for

    provisioners = get_provisioners_for(host, bench)
    for resource in resources:
        for provisioner in provisioners:
            if resource.metatype == provisioner.resource_type:
                await provisioner.decommission(resource)
                break
        else:
            raise RuntimeError(f"no provisioner for {resource!r} in {provisioners!r}")
