import abc
import asyncio
from typing import TYPE_CHECKING, cast, final

import structlog
from opentelemetry import trace

from bench.language import (
    NODE_CLASS_BY_TYPE,
    Bench,
    Message,
    MessageType,
    NodeMode,
    NodeType,
    ProvisionableResource,
    ResourceStatus,
    Thread,
)
from bench.system.host import Commit, DeferredHostPlugin, HostService

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Provisioner[PT: ProvisionableResource, WT: ProvisionableResource](
    DeferredHostPlugin[WT], abc.ABC
):
    """
    A provisioner for some type of Resource.
    Synchronizes the declared state of Resources with their actual (external) state (bidirectionally).
    """

    provision_type: NodeType

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._lock = asyncio.Lock()
        self._slug = self.provision_type.bench_name.lower()

    @property
    def slug(self) -> str:
        return self._slug

    async def _get_resources(self) -> list[PT]:
        """Gets all Resource for this Provisioner."""
        provision_cls = cast(type[PT], NODE_CLASS_BY_TYPE[self.provision_type])
        resources_query = (
            provision_cls.where(
                provision_cls.get_property("bench").eq(self.bench)
                & provision_cls.get_property("mode").lt(NodeMode.TEMPLATE)
                & provision_cls.get_property("status").lt(ResourceStatus.OFFLINE)
            )
            .include_ancestors()
            .select_all()
        )
        resources_query._include_memory = False
        resources = await resources_query.tolist()
        return resources

    def _set_resource_status(self, resource: PT, status: ResourceStatus) -> None:
        """Set the status of the Resource, emitting any Messages."""
        # create message if status changed
        if resource.status != status and isinstance(thread := resource.parent, Thread):
            message = Message.new(
                parent=thread,
                type=MessageType.RESOURCE,
                nodes=[resource],
                resource_status=status,
                _supergraph=thread._supergraph,
            )
            thread.messages.append(message)

        resource.update_status(status)

    @final
    async def start(self) -> None:
        await self._do_start()

        resources = await self._get_resources()

        # auto migrate resources to current version
        for resource in resources:
            # provision/update/decommission
            try:
                if resource.target_status.is_extant:
                    if resource.status.is_extant:
                        await self.update(resource)
                    elif resource.should_retry:
                        await self.provision(resource)
                elif resource.status.is_extant:
                    await self.decommission(resource)
            except Exception:
                # suppress error, already logged, continue
                continue

        # then start watching in host plugin
        #  (starting watch after above is important because there is no lock between this and post_commit_deferred,
        #   and we assume exclusivity in the provisioning methods. Host plugins start the queue in .start)
        await super().start()

    async def _do_start(self) -> None:
        pass  # to be overridden

    @tracer.start_as_current_span("provisioner.post_commit_deferred")
    async def post_commit_deferred(self, commit: Commit[WT]) -> None:
        trace.get_current_span().set_attribute("plugin", self.name)

        # handle edit by updating resource
        if commit.has(self.provision_type):
            subcommit = cast(
                Commit[PT],
                commit.trim_to(
                    lambda r: r.metatype == self.provision_type and r.mode < NodeMode.TEMPLATE
                ),
            )
            for resource in subcommit.added:
                if resource.target_status.is_extant and not resource.status.is_extant:
                    await self.provision(resource)
            for resource in subcommit.updated:
                if resource.target_status.is_extant:
                    if resource.status.is_extant:
                        await self.update(resource)
                    elif resource.should_retry:
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
                if not resource.status.is_pre:
                    return  # already provisioned (while waiting)
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
            self.host.on_error(e)
            async with self.host.session(commit=True):
                self._set_resource_status(resource, ResourceStatus.RETRYING)
            raise

    @abc.abstractmethod
    async def _do_provision(self, resource: PT): ...

    @final
    async def update(self, resource: PT):
        """Update the Resource properties."""
        try:
            async with self._lock:
                if not resource.status.is_extant:
                    return  # no longer around (while waiting)
                with tracer.start_as_current_span(
                    f"{self.slug}.update", attributes={"resource": str(resource)}
                ):
                    await self._do_update(resource)
                    logger.trace(
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
            self.host.on_error(e)
            raise

    async def _do_update(self, resource: PT): ...

    @final
    async def decommission(self, resource: PT):
        """Decommission the Resource."""
        try:
            async with self._lock:
                if not resource.status.is_extant:
                    return  # was already decommissioned (while waiting)
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
            self.host.on_error(e)
            async with self.host.session(commit=True):
                self._set_resource_status(resource, ResourceStatus.FAILED)
            raise

    @abc.abstractmethod
    async def _do_decommission(self, resource: PT): ...
