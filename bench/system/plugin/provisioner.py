import abc
import asyncio
from typing import TYPE_CHECKING, cast, final

import structlog
from opentelemetry import trace

from bench.language import (
    NODE_CLASS_BY_TYPE,
    Bench,
    IsProvisionable,
    NodeMode,
    NodeType,
    ResourceStatus,
)
from bench.system.host import Commit, DeferredHostPlugin, HostService

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Provisioner[PT: IsProvisionable, WT: IsProvisionable](DeferredHostPlugin[WT], abc.ABC):
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
        resources = await provision_cls.search(
            where=provision_cls.property("bench").eq(self.bench)
            & provision_cls.property("mode").lt(NodeMode.TEMPLATE)
            & provision_cls.property("status").lt(ResourceStatus.OFFLINE)
        ).execute_list()
        return resources

    def _set_resource_status(self, resource: PT, status: ResourceStatus) -> None:
        """Set the status of the Resource, emitting any Messages."""
        # create message if status changed
        # if resource.status != status and isinstance(thread := resource.parent, Thread):
        #     message = Message(
        #         parent=thread,
        #         type=MessageType.RESOURCE,
        #         nodes=[resource],
        #         resource_status=status,
        #         _supergraph=thread._supergraph,
        #     )
        #     thread.add_child(message)

        resource.update_status(status)

    @final
    async def start(self) -> None:
        await self._do_start()
        await super().start()
        raise NotImplementedError

    async def _do_start(self) -> None:
        pass  # to be overridden

    @tracer.start_as_current_span("provisioner.post_commit_deferred")
    async def post_commit_deferred(self, commit: Commit[WT]) -> None:
        trace.get_current_span().set_attribute("plugin", self.name)

        raise NotImplementedError
