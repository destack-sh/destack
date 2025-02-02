import abc
import asyncio
from typing import TYPE_CHECKING, Collection, cast, final

import structlog
from opentelemetry import trace

from bench.language import (
    NODE_CLASS_BY_TYPE,
    STATIC_RESOURCE_NODE_TYPES,
    VERSION,
    Bench,
    NodeType,
    Resource,
    ResourceStatus,
)
from bench.system.host.core import Commit, DeferredHostPlugin

if TYPE_CHECKING:
    from bench.system.host import HostService

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Provisioner[PT: Resource, WT: Resource](DeferredHostPlugin[WT], abc.ABC):
    """
    A provisioner for some type of Resource.
    Synchronizes the declared state of Resources with their actual (external) state (bidirectionally).
    """

    provision_type: NodeType
    provision_subtype: int | None = None

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._lock = asyncio.Lock()
        self._slug = self.provision_type.bench_name.lower()

    @property
    def slug(self) -> str:
        return self._slug

    def _filter_resource(self, resource: PT) -> bool:
        """Whether to consider this Resource for provisioning."""
        if self.provision_subtype is not None:
            return getattr(resource, "type", None) == self.provision_subtype
        return True

    @final
    async def start(self) -> None:
        # custom start for provisioner first to update resource status from external state
        await self._do_start()

        # check resources / provision declared resources
        if self.provision_type in STATIC_RESOURCE_NODE_TYPES:
            resources = cast(
                list[PT],
                self.bench._graph.get_descendants(self.bench, self.provision_type, recursive=True),
            )
            if self.provision_subtype is not None:
                resources = [r for r in resources if self._filter_resource(r)]
        else:
            provision_cls = cast(type[PT], NODE_CLASS_BY_TYPE[self.provision_type])
            resources_query = provision_cls.where(
                provision_cls.get_property("bench").eq(self.bench)
                & provision_cls.get_property("status").neq(ResourceStatus.DECOMMISSIONED)
            ).select_all()
            if self.provision_subtype is not None:
                resources_query = resources_query.where(
                    provision_cls.get_property("subtype").eq(self.provision_subtype)
                )
            resources = await resources_query.tolist()
            resources = [r for r in resources if self._filter_resource(r)]

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

    @tracer.start_as_current_span("provisioner.on_commit_deferred")
    async def on_commit_deferred(self, commit: Commit[WT]) -> None:
        trace.get_current_span().set_attribute("plugin", self.name)
        commit = commit.trim_to(
            lambda r: r.metatype != self.provision_type or self._filter_resource(cast(PT, r))
        )
        if commit.is_empty:
            return

        # handle edit by updating resource
        if commit.has(self.provision_type):
            subcommit = cast(Commit[PT], commit.trim_to(self.provision_type))
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


async def provision(host: "HostService", bench: Bench, resources: Collection[Resource]) -> None:
    """Provisions the given resources in *this* environment"""
    from .registry import get_provisioners

    provisioners = get_provisioners(host, bench)
    for resource in resources:
        for provisioner in provisioners:
            if resource.metatype == provisioner.provision_type:
                await provisioner.provision(resource)
                break
        else:
            raise RuntimeError(f"no provisioner for {resource!r} in {provisioners!r}")
