from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.pb2 import AnyNodeData

from .const import REGION, BuiltinEnum, EnumType, NodeType, Region, enum_
from .node import Node, NodeReference, PageNode, node_component_
from .property import p_internal, p_node_parent, p_system
from .trait import IsClaimable, IsInstantiable, IsModal, IsNamed, IsOwnable

if TYPE_CHECKING:
    from bench.language import Package, Page, Region, Scaler, Thread

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.RESOURCE_STATUS)
class ResourceStatus(BuiltinEnum):
    """Generalized status of a Resource in its lifecycle."""

    # pre
    PENDING = (1, "Pending", "Waiting for provisioning", "fas fa-hourglass-start")
    CREATING = (2, "Creating", "Actively provisioning", "fas fa-hourglass-start")
    RETRYING = (3, "Retrying", "Retrying provisioning", "fas fa-exclamation-triangle")

    # active states
    AVAILABLE = (10, "Available", "Operational and available", "fas fa-check-circle")
    SLEEPING = (11, "Sleeping", "Available but not running", "fas fa-moon")
    UNAVAILABLE = (15, "Unavailable", "Unavailable or not responding", "fas fa-plug-circle-xmark")
    IMPAIRED = (
        16,
        "Impaired",
        "Operational but experiencing issues",
        "fas fa-exclamation-triangle",
    )
    # terminal
    OFFLINE = (30, "Offline", "Decommissioned and unavailable", "fas fa-power-off")
    FAILED = (31, "Failed", "Failed to provision", "fas fa-exclamation-triangle")

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
class ResourceBase[NodeDataT: AnyNodeData](
    IsModal, IsInstantiable, IsOwnable, IsNamed, IsClaimable, PageNode[NodeDataT]
):
    """
    A Resource in a Bench.
    """

    # meta
    parent: Union["Package", "Page", "Thread", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.PAGE, NodeType.THREAD, ckless=True
    )
    # ... space for type/name/...
    region: Region = p_system(38, default=REGION, default_sql=None)


@node_component_()
class ProvisionableResourceBase[NodeDataT: AnyNodeData](ResourceBase[NodeDataT]):
    """
    A Resource that can be provisioned.
    """

    # status
    status: ResourceStatus = p_system(40, default=ResourceStatus.PENDING, default_sql=None)
    requested_activate_at: Optional[datetime] = p_internal(41, default=None)
    requested_deactivate_at: Optional[datetime] = p_internal(42, default=None)
    requested_reset_at: Optional[datetime] = p_internal(43, default=None)
    requested_suspend_at: Optional[datetime] = p_internal(44, default=None)
    requested_decommission_at: Optional[datetime] = p_internal(45, default=None)
    active_at: Optional[datetime] = p_system(46, default=None)
    failed_at: Optional[datetime] = p_system(47, default=None)
    failed_attempts: int = p_system(48, default=0)
    scaler: Optional["Scaler"] = p_system(
        49, require=False, array=False, references=NodeType.SCALER
    )
    if TYPE_CHECKING:
        scaler_ptr: Optional[NodeReference] = None
        scaler_id: Optional[UUID] = None

    def __content_str__(self):
        return Node.__default_content_str__(self)

    @property
    def should_retry(self) -> bool:
        """Whether this Resource should be retried."""
        return self.failed_at is None or (
            self.requested_reset_at is not None and self.requested_reset_at > self.failed_at
        )

    @property
    def should_reset(self) -> bool:
        """Whether this Resource should be reset."""
        return (
            self.status.is_extant
            and self.requested_reset_at is not None
            and self.requested_activate_at is not None
            and self.requested_reset_at > self.requested_activate_at
        )

    @property
    def target_status(self) -> ResourceStatus:
        """The implied target status of this Resource."""
        if self.requested_decommission_at is not None:
            return ResourceStatus.OFFLINE
        elif self.requested_suspend_at is not None and not (
            self.requested_activate_at is not None
            and self.requested_activate_at > self.requested_suspend_at
        ):
            return ResourceStatus.SLEEPING
        elif self.requested_deactivate_at is not None and not (
            self.requested_activate_at is not None
            and self.requested_activate_at > self.requested_deactivate_at
        ):
            return ResourceStatus.UNAVAILABLE
        else:
            return ResourceStatus.AVAILABLE

    def provision(self) -> None:
        """Request to provision this Resource."""
        self.requested_activate_at = self.active_session._oracle.utc()

    def decommission(self) -> None:
        """Request to decommission this Resource."""
        self.requested_decommission_at = self.active_session._oracle.utc()

    def update_status(self, status: ResourceStatus) -> None:
        """Set the actual current status of this Resource."""
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
