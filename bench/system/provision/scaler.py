import abc
import asyncio
from typing import TYPE_CHECKING, Any, Sequence, cast, final, override

import structlog
from opentelemetry import trace

from bench.language import (
    NODE_CLASS_BY_TYPE,
    Bench,
    Browser,
    DynamicResource,
    Machine,
    NodeType,
    Resource,
    ResourceStatus,
    Scaler,
    ScalerType,
    Session,
    bittuple,
    connection_capture,
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


class ScalerProvisioner[WT: DynamicResource](Provisioner[Scaler, Scaler | WT], abc.ABC):
    """A Provisioner that scales a dynamic Resource for all the Scalers of its type."""

    provision_type = NodeType.SCALER
    provision_subtype: ScalerType
    scale_type: NodeType

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._scalers_to_reconcile: set[Scaler] = set()
        self._reconcile_event: asyncio.Event = asyncio.Event()
        resource_cls = NODE_CLASS_BY_TYPE[self.scale_type]
        assert issubclass(resource_cls, Resource), f"bad scaler type {self!r}"
        self._resource_cls: type[Resource] = resource_cls
        self._slug = f"{self.provision_subtype.bench_name.lower()}_scaler"

    @property
    def slug(self) -> str:
        return self._slug

    @final
    @tracer.start_as_current_span("scaler.reconcile")
    @connection_capture("close_and_release")
    async def _do_reconcile(self, scalers: tuple[Scaler, ...] | None = None) -> None:
        """Reconcile the Scalers and the Resources they scale."""

        # get scalers
        scalers = scalers if scalers is not None else tuple(self.bench.scalers)
        scalers = tuple(s for s in scalers if s.type == self.provision_subtype)
        self._scalers_to_reconcile.clear()
        self._reconcile_event.clear()

        # get resources
        resources_query = self._resource_cls.where(
            self._resource_cls.get_property("bench").eq(self.bench)
            & self._resource_cls.get_property("status").neq(ResourceStatus.DECOMMISSIONED)
            & self._resource_cls.get_property("scaler").exists()
        ).select_all()
        resources = cast(list[WT], await resources_query.tolist())
        resources_by_scalar = group_by(resources, lambda r: r.scaler_id)

        # bail if nothing to do
        if not scalers and not resources:
            return

        # reconcile
        async with self.host.session(commit=True) as session:
            for scaler in scalers:
                resource_group = resources_by_scalar.get(scaler.id) or ()
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
                resource_kwargs: dict[str, Any] = {"scaler": scaler, "name": generate_random_name()}
                resource = cast(WT, self._resource_cls(**resource_kwargs))
                self.bench.append(resource)
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
            scalers = tuple(self._scalers_to_reconcile)
            self._scalers_to_reconcile.clear()
            self._reconcile_event.clear()
            await self._do_reconcile(scalers or None)

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
                self._scalers_to_reconcile.add(cast(Scaler, node))
                self._reconcile_event.set()
            elif node.metatype == self.scale_type:
                scaler = getattr(node, "scaler", None)
                if isinstance(scaler, Scaler) and scaler.type == self.provision_subtype:
                    self._scalers_to_reconcile.add(scaler)
                    self._reconcile_event.set()

    @override
    async def _do_provision(self, resource: Scaler):
        self._scalers_to_reconcile.add(resource)
        self._reconcile_event.set()
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.UP  # Scalar is automatically considered up?

    @override
    async def _do_update(self, resource: Scaler):
        self._scalers_to_reconcile.add(resource)
        self._reconcile_event.set()

    @override
    async def _do_decommission(self, resource: Scaler):
        self._scalers_to_reconcile.add(resource)
        self._reconcile_event.set()
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class MachineScalerProvisioner(ScalerProvisioner[Machine]):
    watch_types = bittuple(NodeType.SCALER, NodeType.MACHINE)
    provision_subtype = ScalerType.MACHINE
    scale_type = NodeType.MACHINE


class BrowserScalerProvisioner(ScalerProvisioner[Browser]):
    watch_types = bittuple(NodeType.SCALER, NodeType.BROWSER)
    provision_subtype = ScalerType.BROWSER
    scale_type = NodeType.BROWSER
