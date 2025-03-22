import abc
import asyncio
from typing import TYPE_CHECKING, Any, Sequence, cast, final, override

import structlog
from opentelemetry import trace

from bench.language import (
    NODE_CLASS_BY_TYPE,
    Bench,
    Computer,
    NodeMode,
    NodeType,
    Resource,
    ResourceStatus,
    Scaler,
    ScalerType,
    Session,
    bittuple,
    isolated_graph,
)
from bench.system.host import Commit
from bench.utils.func import group_by
from bench.utils.naming import generate_random_name

from .provisioner import Provisioner

if TYPE_CHECKING:
    from bench.system.host import HostService

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class ScalerProvisioner[WT: Resource](Provisioner[Scaler, Scaler | WT], abc.ABC):
    """A Provisioner that scales a dynamic Resource for all the Scalers of its type."""

    provision_type = NodeType.SCALER
    provision_subtype: ScalerType
    scale_type: NodeType

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._reconcile_event: asyncio.Event = asyncio.Event()
        resource_cls = NODE_CLASS_BY_TYPE[self.scale_type]
        assert issubclass(resource_cls, Resource), f"bad scaler type {self!r}"
        self._resource_cls: type[Resource] = resource_cls
        self._slug = f"{self.provision_subtype.bench_name.lower()}_scaler"

    @property
    def slug(self) -> str:
        return self._slug

    async def _get_scaled_resources(self) -> Sequence[tuple[Scaler, Sequence[WT]]]:
        """Gets all the scaled Resources for this Provisioner."""
        scalers = tuple(
            s
            for s in self.main_package.scalers
            if s.mode <= NodeMode.TEMPLATE and s.type == self.provision_subtype
        )
        if self.scale_type in self.bench._graph.node_types:  # :NodeOverload
            # get from memory
            resources = cast(
                list[WT],
                self.bench._graph.get_descendants(self.main_package, self.scale_type),
            )
        else:
            # get from database
            provision_cls = cast(type[WT], NODE_CLASS_BY_TYPE[self.scale_type])
            resources_query = provision_cls.where(
                provision_cls.get_property("bench").eq(self.bench)
                & provision_cls.get_property("status").neq(ResourceStatus.DECOMMISSIONED)
            ).select_all()
            if self.provision_subtype is not None:
                resources_query = resources_query.where(
                    provision_cls.get_property("type").eq(self.provision_subtype)
                )
            resources = await resources_query.tolist()
        resources = [r for r in resources if r.mode < NodeMode.TEMPLATE]
        resources_by_scaler = group_by(resources, lambda r: r.scaler_id)
        return [(s, resources_by_scaler.get(s.id, ())) for s in scalers]

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
            for scaler, resource_group in resources_by_scaler:
                if not scaler.is_extant:
                    # decommission
                    for resource in resource_group:
                        resource.decommission()
                elif scaler.is_active:
                    # rebalance resource group
                    self._rebalance(session, scaler, resource_group)

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
                resource_kwargs: dict[str, Any] = {
                    "parent": self.main_package,
                    "scaler": scaler,
                    "name": generate_random_name(),
                }
                resource = cast(WT, self._resource_cls(**resource_kwargs))
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
    @tracer.start_as_current_span("scaler.on_commit_deferred")
    async def on_commit_deferred(self, commit: Commit[Scaler | WT]) -> None:
        trace.get_current_span().set_attribute("plugin", self.name)
        commit = commit.trim_to(
            lambda r: r.metatype != self.provision_type or self._filter_resource(cast(Scaler, r))
        )
        if commit.is_empty:
            return

        # add scalers to reconcile
        for node in commit.edited:
            if node.metatype == self.provision_type:
                self._reconcile_event.set()
            elif node.metatype == self.scale_type:
                scaler = getattr(node, "scaler", None)
                if isinstance(scaler, Scaler) and scaler.type == self.provision_subtype:
                    self._reconcile_event.set()

    @override
    async def _do_provision(self, resource: Scaler):
        self._reconcile_event.set()
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.UP  # Scalar is automatically considered up?

    @override
    async def _do_update(self, resource: Scaler):
        self._reconcile_event.set()

    @override
    async def _do_decommission(self, resource: Scaler):
        self._reconcile_event.set()
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class ComputerScalerProvisioner(ScalerProvisioner[Computer]):
    watch_types = bittuple(NodeType.SCALER, NodeType.COMPUTER)
    provision_subtype = ScalerType.COMPUTER
    scale_type = NodeType.COMPUTER
