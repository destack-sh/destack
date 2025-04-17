from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

from bench.pb2 import AnyNodeData

from .const import (
    REGION,
    BuiltinEnum,
    ColorType,
    EnumType,
    NodeType,
    Region,
    enum_,
)
from .node import InlineNode, Node, NodeReference, node_component_
from .property import p_internal, p_node_parent, p_system
from .trait import IsClaimable, IsInstantiable, IsModal, IsNamed, IsOwnable

if TYPE_CHECKING:
    from bench.language import Channel, Package, Page, Region, Scaler, Thread

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.RESOURCE_STATUS)
class ResourceStatus(BuiltinEnum):
    """Generalized status of a Resource in its lifecycle."""

    # pre
    PENDING = (
        1,
        "Pending",
        "Waiting for provisioning",
        "fas fa-hourglass-start",
        ColorType.BLUE,
    )
    CREATING = (
        2,
        "Creating",
        "Actively provisioning",
        "fas fa-hourglass-start",
        ColorType.BLUE,
    )
    RETRYING = (
        3,
        "Retrying",
        "Retrying provisioning",
        "fas fa-exclamation-triangle",
        ColorType.YELLOW,
    )

    # active states
    AVAILABLE = (
        10,
        "Available",
        "Operational and available",
        "fas fa-check-circle",
        ColorType.GREEN,
    )
    SLEEPING = (
        11,
        "Sleeping",
        "Available but not running",
        "fas fa-moon",
        ColorType.BLUE,
    )
    UNAVAILABLE = (
        15,
        "Unavailable",
        "Unavailable or not responding",
        "fas fa-plug-circle-xmark",
        ColorType.YELLOW,
    )
    IMPAIRED = (
        16,
        "Impaired",
        "Operational but experiencing issues",
        "fas fa-exclamation-triangle",
        ColorType.YELLOW,
    )
    # terminal
    OFFLINE = (
        30,
        "Offline",
        "Decommissioned and unavailable",
        "fas fa-power-off",
        ColorType.GRAY,
    )
    FAILED = (
        31,
        "Failed",
        "Failed to provision",
        "fas fa-exclamation-triangle",
        ColorType.RED,
    )

    @property
    def is_pre(self) -> bool:
        """Whether this Resource is in the pre-provisioning state."""
        return 1 <= self.value < 10

    @property
    def is_extant(self) -> bool:
        """Whether this Resource does/should exist."""
        return 10 <= self.value <= 20

    @property
    def is_terminal(self) -> bool:
        """Whether this Resource is terminal."""
        return 30 <= self.value <= 40


@node_component_()
class Resource[NodeDataT: AnyNodeData](
    IsModal, IsInstantiable, IsOwnable, IsNamed, IsClaimable, InlineNode[NodeDataT]
):
    """
    A Resource in a Bench.
    Resources generally work on the 'desired state' principle.
    Where applicable, the target state is stored in target_* properties.
    """

    # meta
    parent: Union["Package", "Page", "Channel", "Thread", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.PAGE, NodeType.CHANNEL, NodeType.THREAD, ckless=True
    )
    # ... space for type/name/...
    region: Region = p_system(38, default=REGION, default_sql=None)
    scaler: Optional["Scaler"] = p_system(
        39, require=False, array=False, references=NodeType.SCALER
    )
    if TYPE_CHECKING:
        scaler_ptr: Optional[NodeReference] = None
        scaler_id: Optional[UUID] = None

    # status
    status: ResourceStatus = p_system(40, default=ResourceStatus.PENDING, default_sql=None)
    activated_at: Optional[datetime] = p_internal(41, default=None)
    deactivated_at: Optional[datetime] = p_internal(42, default=None)
    reset_at: Optional[datetime] = p_internal(43, default=None)
    suspended_at: Optional[datetime] = p_internal(44, default=None)
    decommissioned_at: Optional[datetime] = p_internal(45, default=None)
    active_at: Optional[datetime] = p_system(46, default=None)
    failed_at: Optional[datetime] = p_system(47, default=None)
    failed_attempts: int = p_system(48, default=0)

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
    def should_retry(self) -> bool:
        """Whether this Resource should be retried."""
        return self.failed_at is None or (
            self.reset_at is not None and self.reset_at > self.failed_at
        )

    @property
    def target_status(self) -> ResourceStatus:
        """The implied target status of this Resource."""
        if self.decommissioned_at is not None:
            return ResourceStatus.OFFLINE
        elif self.suspended_at is not None and not (
            self.activated_at is not None and self.activated_at > self.suspended_at
        ):
            return ResourceStatus.SLEEPING
        elif self.deactivated_at is not None and not (
            self.activated_at is not None and self.activated_at > self.deactivated_at
        ):
            return ResourceStatus.UNAVAILABLE
        else:
            return ResourceStatus.AVAILABLE

    def provision(self) -> None:
        """Provision this Resource."""
        self.activated_at = self.active_session._oracle.utc()

    def decommission(self) -> None:
        """Decommission this Resource."""
        self.decommissioned_at = self.active_session._oracle.utc()

    def update_status(self, status: ResourceStatus) -> None:
        """Set the status of this Resource."""
        self.status = status
        if status == ResourceStatus.FAILED or status == ResourceStatus.RETRYING:
            self.failed_at = self.active_session._oracle.utc()
            self.failed_attempts += 1
        elif status.is_extant:
            self.active_at = self.active_session._oracle.utc()
            self.failed_attempts = 0

    async def wait_until_status(
        self, status: ResourceStatus, timeout: timedelta | None = None
    ) -> None:
        """Wait until this Resource reaches the given status."""
        await self.wait_until(lambda r: r.status == status, timeout=timeout)

    async def wait_until_ready(self, timeout: timedelta | None = None) -> None:
        """Wait until this Resource is ready."""
        await self.wait_until(lambda r: r.status == ResourceStatus.AVAILABLE, timeout=timeout)
