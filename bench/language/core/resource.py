from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Optional, TypeVar, Union
from uuid import UUID

from bench.pb2 import AnyNodeData

from .const import (
    REGION,
    BuiltinEnum,
    ColorType,
    EnumType,
    NodeType,
    Region,
    bittuple,
    enum_,
)
from .node import (
    InlineNode,
    IsClaimable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsTemplatable,
    Node,
    NodeReference,
    node_component_,
)
from .property import p_internal, p_node_parent, p_system

if TYPE_CHECKING:
    from bench.language import Channel, Package, Page, Region, Scaler, Thread

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.RESOURCE_STATUS)
class ResourceStatus(BuiltinEnum):
    """Generalized status of a Resource in its lifecycle."""

    # definition
    DECLARED = 1, None, None, "fas fa-circle-dot", ColorType.BLUE
    # extant
    UP = 10, None, None, "fas fa-circle-check", ColorType.GREEN
    SLEEPING = 11, None, None, "fas fa-zzz", ColorType.BLUE
    DOWN = 15, None, None, "fas fa-circle-xmark", ColorType.RED
    DEGRADED = 16, None, None, "fas fa-circle-exclamation", ColorType.YELLOW
    # terminal
    DECOMMISSIONED = 30, None, None, "fas fa-circle-slash", ColorType.GRAY

    @property
    def is_extant(self) -> bool:
        """Whether this resouce does/should exist."""
        return 10 <= self.value <= 20


EXTANT_RESOURCE_STATUSES = bittuple(*(s for s in ResourceStatus if 10 <= s.value <= 20))

NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)


@node_component_()
class Resource[NodeDataT: AnyNodeData](
    IsModal, IsTemplatable, IsOwnable, IsNamed, IsClaimable, InlineNode[NodeDataT]
):
    """
    A Resource in a Bench.
    Resources generally work on the 'desired state' principle.
    Where applicable, the target state is stored in target_* properties.
    """

    parent: Union["Package", "Page", "Channel", "Thread", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.THREAD, NodeType.PAGE, ckless=True
    )
    # ... space for type/name/...

    # meta
    region: Region = p_system(40, default=REGION, default_sql=None)
    scaler: Optional["Scaler"] = p_system(
        41, require=False, array=False, references=NodeType.SCALER
    )
    if TYPE_CHECKING:
        scaler_id: Optional[UUID] = None
        scaler_ptr: Optional[NodeReference] = None

    # status
    status: ResourceStatus = p_system(50, default=ResourceStatus.DECLARED, default_sql=None)
    # target
    activated_at: Optional[datetime] = p_internal(51, default=None)
    deactivated_at: Optional[datetime] = p_internal(52, default=None)
    reset_at: Optional[datetime] = p_internal(53, default=None)
    suspended_at: Optional[datetime] = p_internal(54, default=None)
    decommissioned_at: Optional[datetime] = p_internal(55, default=None)
    # current
    active_at: Optional[datetime] = p_system(56, default=None)

    def __content_str__(self):
        return Node.__default_content_str__(self)

    def _get_target_diff(self, *keys: str) -> dict[str, Any]:
        """
        Checks whether specific properties <key> differ from their target_<key> values.
        Returns the target values of differing properties.
        """
        target_diff: dict[str, Any] = {}
        for key in keys:
            prop = self.__properties__.get(key)
            assert prop is not None, f"no property '{key}' in {self.__class__.__name__}"
            current_value = getattr(self, key)
            target_value = getattr(self, f"target_{key}")
            if current_value != target_value:
                target_diff[key] = target_value
        return target_diff

    @property
    def target_status(self) -> ResourceStatus:
        """The implied target status of this Resource."""
        if self.decommissioned_at is not None:
            return ResourceStatus.DECOMMISSIONED
        elif self.suspended_at is not None and not (
            self.activated_at is not None and self.activated_at > self.suspended_at
        ):
            return ResourceStatus.SLEEPING
        elif self.deactivated_at is not None and not (
            self.activated_at is not None and self.activated_at > self.deactivated_at
        ):
            return ResourceStatus.DOWN
        else:
            return ResourceStatus.UP

    def provision(self) -> None:
        """Provision this Resource."""
        assert not self.status.is_extant, f"{self!r} already exists"
        self.activated_at = self.active_session._oracle.utc()

    def decommission(self) -> None:
        """Decommission this Resource."""
        assert self.status.is_extant, f"{self!r} does not exist"
        self.decommissioned_at = self.active_session._oracle.utc()

    async def wait_until_status(
        self, status: ResourceStatus, timeout: timedelta | None = None
    ) -> None:
        """Wait until this Resource reaches the given status."""
        await self.wait_until(lambda r: r.status == status, timeout=timeout)

    async def wait_until_ready(self, timeout: timedelta | None = None) -> None:
        """Wait until this Resource is ready."""
        await self.wait_until(lambda r: r.status == ResourceStatus.UP, timeout=timeout)
