import asyncio
from typing import TYPE_CHECKING, Any, Mapping, Sequence, cast, final, override

import structlog
from opentelemetry import trace

from bench.language import (
    NODE_CLASS_BY_TYPE,
    Bench,
    NodeMode,
    NodeType,
    Resource,
    ResourceStatus,
    Scaler,
    Session,
    bittuple,
    isolated_graph,
)
from bench.system.host import Commit

from .provisioner import Provisioner

if TYPE_CHECKING:
    from bench.system.host import HostService

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class ScalerProvisioner[WT: Resource](Provisioner[Scaler, Scaler | WT]):
    """
    A Provisioner that scales a dynamic Resource for all the Scalers of its type.
    NOTE :Incomplete: ScalerProvisioner should probably be closer to the kubernetes cluster?
    """

    provision_type = NodeType.SCALER
    watch_types = bittuple(NodeType.SCALER, NodeType.COMPUTER)
    scale_types = (NodeType.COMPUTER,)

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._reconcile_event: asyncio.Event = asyncio.Event()

    @property
    def slug(self) -> str:
        return self._slug

    async def _get_scaled_resources(self) -> Mapping[Scaler, list[WT]]:
        """Gets all the scaled Resources for this Provisioner."""
        resources_by_scaler: dict[Scaler, list[WT]] = {}
        for scaler in self.main_package.scalers:
            if scaler.is_active:
                resources_by_scaler[scaler] = []
        for scale_type in self.scale_types:
            provision_cls = cast(type[WT], NODE_CLASS_BY_TYPE[scale_type])
            resources_query = (
                provision_cls.where(
                    provision_cls.get_property("bench").eq(self.bench)
                    & provision_cls.get_property("scaler").is_not_none()
                    & provision_cls.get_property("mode").lt(NodeMode.TEMPLATE)
                    & provision_cls.get_property("status").lt(ResourceStatus.OFFLINE)
                )
                .include_ancestors()
                .select_all()
            )
            resources_query._include_memory = False
            resources = await resources_query.tolist()
            for r in resources:
                scaler = r.scaler
                if scaler is None:
                    continue
                if scaler not in resources_by_scaler:
                    resources_by_scaler[scaler] = []
                resources_by_scaler[scaler].append(r)
        return resources_by_scaler

    @final
    @tracer.start_as_current_span("scaler.reconcile")
    @isolated_graph()
    async def _do_reconcile(self) -> None:
        """Reconcile the Scalers and the Resources they scale."""
        self._reconcile_event.clear()

        # get scalers
        resources_by_scaler = await self._get_scaled_resources()
        if not resources_by_scaler:
            return

        # reconcile
        async with self.host.session(commit=True) as session:
            for scaler, resource_group in resources_by_scaler.items():
                if scaler.is_active:
                    # rebalance resource group
                    self._rebalance(session, scaler, resource_group)
                else:
                    # decommission
                    for resource in resource_group:
                        resource.decommission()

    def _rebalance(self, session: Session, scaler: Scaler, resource_group: Sequence[WT]) -> None:
        """Rebalance a Scaler's (dynamic) Resource group."""
        added: list[WT] = []
        removed: list[WT] = []
        if len(resource_group) > scaler.target_count:
            # decommission excess resources
            for resource in resource_group[scaler.target_count :]:
                resource.decommission()
                removed.append(resource)
        elif len(resource_group) < scaler.target_count:
            # provision missing resources
            for _ in range(scaler.target_count - len(resource_group)):
                resource_cls = cast(type[WT], NODE_CLASS_BY_TYPE[cast(NodeType, scaler.type)])
                resource_kwargs: dict[str, Any] = {"parent": self.main_package, "scaler": scaler}
                resource = cast(WT, resource_cls(**resource_kwargs))
                resource.name = scaler.name_template
                resource.mode = scaler.mode
                session._create(resource)
                added.append(resource)
        if added or removed:
            logger.info(
                "scaler.rebalance",
                scaler=scaler,
                resource_group=resource_group,
                added=added,
                removed=removed,
            )
        else:
            logger.trace("scaler.rebalance.noop", scaler=scaler, resource_group=resource_group)

    @final
    async def _reconcile_forever(self):
        while True:
            await self._reconcile_event.wait()
            self._reconcile_event.clear()
            await self._do_reconcile()

    @override
    async def _do_start(self) -> None:
        self._reconcile_event.set()  # always reconcile on start
        self.tasks.run(self._reconcile_forever(), task_id=f"{self.slug}.reconcile")

    @final
    @tracer.start_as_current_span("scaler.post_commit_deferred")
    async def post_commit_deferred(self, commit: Commit[Scaler | WT]) -> None:
        trace.get_current_span().set_attribute("plugin", self.name)
        # reconcile
        for node in commit.edited:
            if node.mode >= NodeMode.TEMPLATE:
                continue
            elif node.metatype == self.provision_type:
                self._reconcile_event.set()
            elif node.metatype in self.scale_types:
                scaler = getattr(node, "scaler", None)
                if isinstance(scaler, Scaler):
                    self._reconcile_event.set()

    @override
    async def _do_provision(self, resource: Scaler):
        self._reconcile_event.set()
        async with self.host.session(commit=True):
            resource.update_status(ResourceStatus.AVAILABLE)

    @override
    async def _do_update(self, resource: Scaler):
        self._reconcile_event.set()

    @override
    async def _do_decommission(self, resource: Scaler):
        self._reconcile_event.set()
        async with self.host.session(commit=True):
            resource.update_status(ResourceStatus.OFFLINE)
